// High-resolution mass spectrometry (HRMS) target screening engine.
// Performs targeted substance screening across chromatographic traces, S/N peak detection,
// isotope pattern verification, MS2 precursor matching, and prediction proposal assignment.

use serde::{Deserialize, Serialize};

use mzlab_core::composition::Counts;
use mzlab_core::elements::parse_formula;
use mzlab_core::mzml::Spectrum;

use crate::entry::AdductType;
use crate::indice::TargetList;

/// A confirmed or proposed match for a target substance in chromatographic HRMS data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetMatch {
    /// Unique identifier slug of the target entry.
    pub id: String,
    /// English common name.
    pub name_en: String,
    /// Molecular formula of the neutral active principle.
    pub neutral_formula: String,
    /// Detected electrospray ionization adduct type.
    pub adduct: AdductType,
    /// Theoretical m/z for this substance and adduct.
    pub target_mz: f64,
    /// Observed m/z at the chromatographic peak apex.
    pub observed_mz: f64,
    /// Exact mass measurement error in parts-per-million (ppm).
    pub ppm_error: f64,
    /// Retention time at the peak apex in minutes.
    pub rt_apex: f64,
    /// Retention time at the start of the chromatographic peak in minutes.
    pub rt_start: f64,
    /// Retention time at the end of the chromatographic peak in minutes.
    pub rt_end: f64,
    /// Peak height (maximum intensity at apex).
    pub height: f64,
    /// Peak area calculated by trapezoidal integration (intensity * minutes).
    pub area: f64,
    /// Signal-to-noise ratio relative to local baseline noise.
    pub sn_ratio: f64,
    /// Isotope pattern match score (0.0 to 1.0) when verified.
    pub isotope_score: Option<f64>,
    /// True if a corresponding precursor was found in MS2 scans near apex RT.
    pub ms2_matched: bool,
    /// Identification proposal ("Prev. 1", "Prev. 2", or "Prev. 3").
    pub proposal: String,
    /// Scientific rationale and evidence summary.
    pub motif: String,
}

/// Search parameters and thresholds for HRMS target screening.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetSearchOptions {
    /// Mass accuracy tolerance in ppm (default: 5.0 ppm).
    pub tolerance_ppm: f64,
    /// Minimum signal-to-noise ratio required for chromatographic peak detection (default: 3.0).
    pub min_sn: f64,
    /// Minimum peak area required for a valid chromatographic peak (default: 1000.0).
    pub min_area: f64,
    /// Adduct types to search (default: [M+H]+, [M+Na]+, [M+NH4]+).
    pub adducts: Vec<AdductType>,
}

impl Default for TargetSearchOptions {
    fn default() -> Self {
        Self {
            tolerance_ppm: 5.0,
            min_sn: 3.0,
            min_area: 1000.0,
            adducts: vec![AdductType::MH, AdductType::MNa, AdductType::MNH4],
        }
    }
}

/// Internal detected peak structure on a single XIC trace.
struct DetectedPeak {
    apex_idx: usize,
    rt_apex: f64,
    rt_start: f64,
    rt_end: f64,
    height: f64,
    area: f64,
    sn_ratio: f64,
}

/// Median of `buf` (reordered): by selection, with the full sort kept for the NaN case (the order of NaN there is the sort's).
fn median_of(buf: &mut [f64]) -> f64 {
    if let Some(m) = mzlab_core::sortc::select_median(buf) {
        return m;
    }
    mzlab_core::sortc::by(buf, |a, b| a.total_cmp(b));
    let mid = buf.len() / 2;
    if buf.len().is_multiple_of(2) && buf.len() > 1 {
        (buf[mid - 1] + buf[mid]) * 0.5
    } else {
        buf[mid]
    }
}

/// Estimate chromatographic baseline noise using Median Absolute Deviation (MAD); `buf` is scratch space reused between calls.
/// A trace that is flat for more than half of its points has MAD 0: the noise is then the fixed 1.0 (intensity units), which makes S/N equal to the raw height.
fn estimate_noise(y: &[f64], buf: &mut Vec<f64>) -> f64 {
    if y.is_empty() {
        return 1.0;
    }
    buf.clear();
    buf.extend_from_slice(y);
    let med = median_of(buf);
    buf.clear();
    buf.extend(y.iter().map(|v| (v - med).abs()));
    let n = median_of(buf) * 1.4826;
    if n > 1e-6 {
        n
    } else {
        1.0
    }
}

/// Detect local chromatographic peak maxima that exceed the noise threshold.
fn detect_chrom_peaks(
    rt: &[f64],
    y: &[f64],
    min_sn: f64,
    min_area: f64,
    noise: f64,
) -> Vec<DetectedPeak> {
    let n = y.len();
    if n < 3 {
        return Vec::new();
    }
    let threshold = min_sn * noise;
    let mut peaks = Vec::new();
    let mut i = 1;
    while i < n - 1 {
        if y[i] > y[i - 1] && y[i] >= y[i + 1] && y[i] >= threshold {
            let mut j = i;
            while j < n - 1 && y[j + 1] == y[i] {
                j += 1;
            }
            if j == n - 1 || y[j + 1] < y[i] {
                let apex_idx = (i + j) / 2;
                let height = y[apex_idx];
                let sn_ratio = height / noise;
                let frac_h = 0.05 * height;

                let mut left = i;
                while left > 0 && y[left - 1] <= y[left] && y[left - 1] > frac_h {
                    left -= 1;
                }
                if left > 0 && y[left - 1] <= frac_h {
                    left -= 1;
                }

                let mut right = j;
                while right < n - 1 && y[right + 1] <= y[right] && y[right + 1] > frac_h {
                    right += 1;
                }
                if right < n - 1 && y[right + 1] <= frac_h {
                    right += 1;
                }

                let mut area = 0.0;
                for k in left..right {
                    let dt = rt[k + 1] - rt[k];
                    let avg_y = (y[k] + y[k + 1]) * 0.5;
                    area += avg_y * dt;
                }

                if area >= min_area && sn_ratio >= min_sn {
                    peaks.push(DetectedPeak {
                        apex_idx,
                        rt_apex: rt[apex_idx],
                        rt_start: rt[left],
                        rt_end: rt[right],
                        height,
                        area,
                        sn_ratio,
                    });
                }
                i = right + 1;
                continue;
            }
        }
        i += 1;
    }
    peaks
}

/// Helper to add an element count to the Counts vector.
fn add_count(counts: &mut Counts, el: &str, delta: i64) {
    if let Some(entry) = counts.iter_mut().find(|(e, _)| e == el) {
        entry.1 += delta;
    } else {
        counts.push((el.to_string(), delta));
    }
}

/// Element counts of the adduct ion of a neutral formula (`None` when the formula does not parse); built once per entry and adduct.
fn adduct_counts(neutral: &Counts, adduct: AdductType) -> Counts {
    let mut counts = neutral.clone();
    match adduct {
        AdductType::MH => add_count(&mut counts, "H", 1),
        AdductType::MNa => add_count(&mut counts, "Na", 1),
        AdductType::MNH4 => {
            add_count(&mut counts, "N", 1);
            add_count(&mut counts, "H", 4);
        }
    }
    counts
}

/// Peaks of `spec` with m/z in [lo, hi], in file order. The window starts at a bisection when the m/z array is ascending (`sorted`), and the same filter then keeps only the peaks inside it.
fn window<'a>(
    spec: &'a Spectrum,
    sorted: bool,
    lo: f64,
    hi: f64,
) -> impl Iterator<Item = (f64, f32)> + 'a {
    let n = spec.mz.len().min(spec.intensity.len());
    // the window holds few peaks: after the bisection for its start, its end is found by walking
    let (a, b) = if sorted {
        let a = spec.mz[..n].partition_point(|&m| m < lo);
        let len = spec.mz[a..n].iter().position(|&m| m > hi);
        (a, len.map_or(n, |k| a + k))
    } else {
        (0, n)
    };
    spec.mz[a..b]
        .iter()
        .zip(&spec.intensity[a..b])
        .filter(move |(&m, _)| m >= lo && m <= hi)
        .map(|(&m, &i)| (m, i))
}

/// True when the m/z array can be bisected: ascending (ties allowed) and free of NaN.
fn is_ascending(spec: &Spectrum) -> bool {
    spec.mz.windows(2).all(|w| w[0] <= w[1])
}

/// Of the peaks accepted by `ok`, the most intense one; on equal intensity the closest to `theo`, then the first.
fn strongest(obs: &[(f64, f64)], theo: f64, ok: impl Fn(f64) -> bool) -> Option<(f64, f64)> {
    let mut best: Option<(f64, f64)> = None;
    for &(m, i) in obs.iter().filter(|(m, _)| ok(*m)) {
        let better = match best {
            None => true,
            Some((bm, bi)) => i > bi || (i == bi && (m - theo).abs() < (bm - theo).abs()),
        };
        if better {
            best = Some((m, i));
        }
    }
    best
}

/// Compute isotope pattern score by comparing theoretical and observed isotopologues.
fn compute_isotope_score(
    counts: &Counts,
    target_mz: f64,
    spec: &Spectrum,
    sorted: bool,
    tolerance_ppm: f64,
) -> Option<f64> {
    let obs: Vec<(f64, f64)> = window(spec, sorted, target_mz - 0.5, target_mz + 3.5)
        .map(|(m, i)| (m, i as f64))
        .collect();

    let opts = mzlab_core::isoscore::IsoOpts {
        r200: 20000.0,
        sigma_ppm: tolerance_ppm,
        sigma_int: 0.25,
        min_rel: 1.0,
        noise_rel: 0.1,
        norm: "mono",
    };

    if let Ok(iso) = mzlab_core::isoscore::isotope_score(counts, 1, &obs, &opts) {
        if iso.score > 0.0 {
            return Some((iso.score * 100.0).round() / 100.0);
        }
    }

    // Fallback: estimate M+1 presence at ~+1.00335 Da
    let n_c = counts
        .iter()
        .find(|(e, _)| e == "C")
        .map(|(_, k)| *k as f64)
        .unwrap_or(0.0);
    let n_n = counts
        .iter()
        .find(|(e, _)| e == "N")
        .map(|(_, k)| *k as f64)
        .unwrap_or(0.0);
    let exp_m1_ratio = (n_c * 0.0108 + n_n * 0.0037).max(0.01);
    let exp_m1_mz = target_mz + 1.003355;
    let tol = exp_m1_mz * tolerance_ppm * 1e-6;

    let m0_tol = target_mz * tolerance_ppm * 1e-6;
    let m0_peak = strongest(&obs, target_mz, |m| (m - target_mz).abs() <= m0_tol);
    let m1_peak = strongest(&obs, exp_m1_mz, |m| (m - exp_m1_mz).abs() <= tol);

    match (m0_peak, m1_peak) {
        (Some((_, i0)), Some((m1_obs, i1))) if i0 > 0.0 => {
            let obs_ratio = i1 / i0;
            let d_ppm = (m1_obs - exp_m1_mz) / exp_m1_mz * 1e6;
            let tm = (-0.5 * (d_ppm / tolerance_ppm).powi(2)).exp();
            let ti = (-0.5 * ((obs_ratio / exp_m1_ratio).ln() / 0.35).powi(2)).exp();
            let s = tm * ti;
            Some((s * 100.0).round() / 100.0)
        }
        _ => None,
    }
}

/// Verify if an MS2 precursor matching target_mz is present near apex retention time.
fn check_ms2_match(
    target_mz: f64,
    rt_apex: f64,
    peak_width: f64,
    ms2_spectra: &[&Spectrum],
    tolerance_ppm: f64,
) -> bool {
    let window_rt = peak_width.max(0.05);
    for s in ms2_spectra {
        if (s.rt - rt_apex).abs() <= window_rt {
            if let Some(prec) = s.precursor {
                let diff_ppm = ((prec - target_mz) / target_mz * 1e6).abs();
                if diff_ppm <= tolerance_ppm {
                    return true;
                }
            }
        }
    }
    false
}

/// Helper to assign numerical rank for sorting proposals.
fn proposal_rank(p: &str) -> u8 {
    match p {
        "Prev. 1" => 1,
        "Prev. 2" => 2,
        "Prev. 3" => 3,
        _ => 4,
    }
}

/// Fast XIC extraction over MS1 spectra for a given m/z window; `sorted[k]` says that spectrum k can be bisected.
fn extract_xic(ms1_spectra: &[&Spectrum], sorted: &[bool], lo: f64, hi: f64) -> (Vec<f64>, f64) {
    let mut y_trace = Vec::with_capacity(ms1_spectra.len());
    let mut max_y = 0.0f64;
    for (s, &asc) in ms1_spectra.iter().zip(sorted) {
        let mut sum_i = 0.0f64;
        for (_, inten) in window(s, asc, lo, hi) {
            sum_i += inten as f64;
        }
        if sum_i > max_y {
            max_y = sum_i;
        }
        y_trace.push(sum_i);
    }
    (y_trace, max_y)
}

/// Format proposal and scientific rationale motif.
fn format_proposal_and_motif(
    peak: &DetectedPeak,
    ppm_error: f64,
    isotope_score: Option<f64>,
    ms2_matched: bool,
    tol_ppm: f64,
) -> Option<(String, String)> {
    let iso_val = isotope_score.unwrap_or(0.0);
    let proposal = if peak.sn_ratio >= 5.0 && iso_val >= 0.7 && ms2_matched {
        "Prev. 1".to_string()
    } else if peak.sn_ratio >= 5.0 && iso_val >= 0.7 {
        "Prev. 2".to_string()
    } else if peak.sn_ratio >= 3.0 && ppm_error.abs() <= tol_ppm {
        "Prev. 3".to_string()
    } else {
        return None;
    };

    let iso_str = match isotope_score {
        Some(s) if s >= 0.7 => format!("isotope pattern confirmed (score {:.2})", s),
        Some(s) => format!("isotope pattern unconfirmed (score {:.2})", s),
        None => "isotope pattern unconfirmed".to_string(),
    };
    let ms2_str = if ms2_matched {
        "MS2 precursor matched"
    } else {
        "no MS2 precursor matched"
    };
    let motif = format!(
        "Peak at RT {:.2} min, S/N {:.1}, mass error {:+.1} ppm, {}, {}",
        peak.rt_apex, peak.sn_ratio, ppm_error, iso_str, ms2_str
    );
    Some((proposal, motif))
}

/// Evaluate a chromatographic peak against target entry and assign proposal.
fn evaluate_peak(
    entry: &crate::entry::TargetEntry,
    adduct: AdductType,
    target_mz: f64,
    counts: Option<&Counts>,
    ms1_spectra: &[&Spectrum],
    sorted: &[bool],
    ms2_spectra: &[&Spectrum],
    peak: DetectedPeak,
    opts: &TargetSearchOptions,
) -> Option<TargetMatch> {
    let spec = ms1_spectra[peak.apex_idx];
    let asc = sorted[peak.apex_idx];
    let tol_da = target_mz * opts.tolerance_ppm * 1e-6;
    let lo = target_mz - tol_da;
    let hi = target_mz + tol_da;

    let mut observed_mz = target_mz;
    let mut max_apex_i = 0.0f32;
    for (m, inten) in window(spec, asc, lo, hi) {
        if inten > max_apex_i {
            max_apex_i = inten;
            observed_mz = m;
        }
    }

    let ppm_error = (observed_mz - target_mz) / target_mz * 1e6;
    let peak_width = peak.rt_end - peak.rt_start;
    let ms2_matched = check_ms2_match(
        target_mz,
        peak.rt_apex,
        peak_width,
        ms2_spectra,
        opts.tolerance_ppm,
    );
    let isotope_score =
        counts.and_then(|c| compute_isotope_score(c, target_mz, spec, asc, opts.tolerance_ppm));

    let (proposal, motif) = format_proposal_and_motif(
        &peak,
        ppm_error,
        isotope_score,
        ms2_matched,
        opts.tolerance_ppm,
    )?;

    Some(TargetMatch {
        id: entry.id.clone(),
        name_en: entry.name_en.clone(),
        neutral_formula: entry.neutral_formula.clone(),
        adduct,
        target_mz,
        observed_mz,
        ppm_error,
        rt_apex: peak.rt_apex,
        rt_start: peak.rt_start,
        rt_end: peak.rt_end,
        height: peak.height,
        area: peak.area,
        sn_ratio: peak.sn_ratio,
        isotope_score,
        ms2_matched,
        proposal,
        motif,
    })
}

/// Search an indexed target list against chromatographic spectra with HRMS verification.
pub fn cerca_targets(
    list: &TargetList,
    spectra: &[Spectrum],
    opts: &TargetSearchOptions,
) -> Vec<TargetMatch> {
    let ms1_spectra: Vec<&Spectrum> = spectra.iter().filter(|s| s.level == 1).collect();
    let ms2_spectra: Vec<&Spectrum> = spectra.iter().filter(|s| s.level == 2).collect();

    if ms1_spectra.is_empty() || list.is_empty() {
        return Vec::new();
    }

    let ms1_rts: Vec<f64> = ms1_spectra.iter().map(|s| s.rt).collect();
    let sorted: Vec<bool> = ms1_spectra.iter().map(|s| is_ascending(s)).collect();
    let mut noise_buf = Vec::with_capacity(ms1_spectra.len());
    let mut matches = Vec::new();

    for entry in list.entries() {
        let neutral: Option<Counts> = parse_formula(&entry.neutral_formula)
            .ok()
            .map(|p| p.into_iter().map(|(el, n)| (el, n as i64)).collect());
        for &adduct in &opts.adducts {
            let target_mz = entry.mz_for_adduct(adduct);
            let tol_da = target_mz * opts.tolerance_ppm * 1e-6;
            let (y_trace, max_y) = extract_xic(
                &ms1_spectra,
                &sorted,
                target_mz - tol_da,
                target_mz + tol_da,
            );
            if max_y <= 0.0 {
                continue;
            }

            let noise = estimate_noise(&y_trace, &mut noise_buf);
            let peaks = detect_chrom_peaks(&ms1_rts, &y_trace, opts.min_sn, opts.min_area, noise);

            if peaks.is_empty() {
                continue;
            }
            let counts = neutral.as_ref().map(|n| adduct_counts(n, adduct));
            for peak in peaks {
                if let Some(m) = evaluate_peak(
                    entry,
                    adduct,
                    target_mz,
                    counts.as_ref(),
                    &ms1_spectra,
                    &sorted,
                    &ms2_spectra,
                    peak,
                    opts,
                ) {
                    matches.push(m);
                }
            }
        }
    }

    // Sort matches: Prev. 1 in front, then Prev. 2, then Prev. 3, ordered by descending S/N
    mzlab_core::sortc::slice_by(&mut matches, |a, b| {
        proposal_rank(&a.proposal)
            .cmp(&proposal_rank(&b.proposal))
            .then_with(|| b.sn_ratio.total_cmp(&a.sn_ratio))
            .then_with(|| b.height.total_cmp(&a.height))
    });

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 >> 33
        }
        fn unit(&mut self) -> f64 {
            self.next() as f64 / (1u64 << 31) as f64
        }
    }

    fn spec(mz: Vec<f64>, intensity: Vec<f32>) -> Spectrum {
        Spectrum {
            index: 0,
            id: String::new(),
            level: 1,
            rt: 0.0,
            polarity: 1,
            profile: false,
            filter: String::new(),
            precursor: None,
            mz,
            intensity,
            wide: None,
        }
    }

    fn linear(s: &Spectrum, lo: f64, hi: f64) -> Vec<(f64, f32)> {
        s.mz.iter()
            .zip(&s.intensity)
            .filter(|(&m, _)| m >= lo && m <= hi)
            .map(|(&m, &i)| (m, i))
            .collect()
    }

    #[test]
    fn window_equals_the_linear_scan_sorted_or_not() {
        let mut r = Lcg(7);
        for round in 0..400 {
            let n = (r.next() % 40) as usize;
            // few distinct values: many ties and peaks exactly on the window edges
            let mut mz: Vec<f64> = (0..n).map(|_| 100.0 + (r.next() % 12) as f64 * 0.25).collect();
            if round % 2 == 0 {
                mz.sort_by(|a, b| a.total_cmp(b));
            }
            if round % 7 == 0 && n > 0 {
                mz[n / 2] = f64::NAN;
            }
            let intensity: Vec<f32> = (0..n).map(|_| (r.next() % 50) as f32).collect();
            let s = spec(mz, intensity);
            let asc = is_ascending(&s);
            assert_eq!(asc, s.mz.windows(2).all(|w| w[0] <= w[1]));
            for _ in 0..10 {
                let lo = 99.5 + r.unit() * 3.5;
                let hi = lo + r.unit() * 1.5;
                let (lo, hi) = if round % 3 == 0 { (100.5, 101.25) } else { (lo, hi) };
                let got: Vec<(f64, f32)> = window(&s, asc, lo, hi).collect();
                let want = linear(&s, lo, hi);
                assert_eq!(got.len(), want.len());
                for (g, w) in got.iter().zip(&want) {
                    assert_eq!((g.0.to_bits(), g.1), (w.0.to_bits(), w.1));
                }
            }
        }
    }

    #[test]
    fn window_survives_short_intensity_array() {
        let s = spec(vec![1.0, 2.0, 3.0], vec![5.0]);
        assert_eq!(window(&s, true, 0.0, 9.0).count(), 1);
        assert_eq!(window(&s, false, 0.0, 9.0).count(), 1);
    }

    fn noise_reference(y: &[f64]) -> f64 {
        if y.is_empty() {
            return 1.0;
        }
        let med = |v: &mut Vec<f64>| {
            v.sort_by(|a, b| a.total_cmp(b));
            let mid = v.len() / 2;
            if v.len() % 2 == 0 && v.len() > 1 {
                (v[mid - 1] + v[mid]) * 0.5
            } else {
                v[mid]
            }
        };
        let m = med(&mut y.to_vec());
        let mut dev: Vec<f64> = y.iter().map(|v| (v - m).abs()).collect();
        let n = med(&mut dev) * 1.4826;
        if n > 1e-6 {
            n
        } else {
            1.0
        }
    }

    #[test]
    fn noise_matches_the_sorted_median_bit_for_bit() {
        let mut r = Lcg(11);
        let mut buf = Vec::new();
        for round in 0..500 {
            let n = (r.next() % 60) as usize;
            let mut y: Vec<f64> = (0..n)
                .map(|_| match round % 4 {
                    0 => (r.next() % 5) as f64,
                    1 => r.unit() * 1e6,
                    2 => 0.0,
                    _ => (r.next() % 3) as f64 * 1000.5,
                })
                .collect();
            if round % 11 == 0 && n > 2 {
                y[1] = f64::NAN;
            }
            if round % 13 == 0 && n > 2 {
                y[0] = f64::INFINITY;
            }
            let want = noise_reference(&y);
            let got = estimate_noise(&y, &mut buf);
            assert_eq!(got.to_bits(), want.to_bits(), "round {round} {y:?}");
        }
    }

    #[test]
    fn flat_trace_noise_falls_back_to_one() {
        let mut buf = Vec::new();
        assert_eq!(estimate_noise(&[0.0; 9], &mut buf), 1.0);
        assert_eq!(estimate_noise(&[], &mut buf), 1.0);
    }

    fn gauss(rt: f64, c: f64, h: f64, w: f64) -> f64 {
        h * (-0.5 * ((rt - c) / w).powi(2)).exp()
    }

    #[test]
    fn two_coeluting_peaks_with_a_shallow_valley_are_both_found() {
        // valley at ~60 % of the lower apex, far above the 5 % tail cut: the first peak's right edge stops at the valley minimum
        let rt: Vec<f64> = (0..120).map(|k| k as f64 * 0.01).collect();
        let y: Vec<f64> = rt
            .iter()
            .map(|&t| gauss(t, 0.45, 10_000.0, 0.06) + gauss(t, 0.72, 7_000.0, 0.06))
            .collect();
        let peaks = detect_chrom_peaks(&rt, &y, 3.0, 1.0, 10.0);
        let apexes: Vec<f64> = peaks.iter().map(|p| rt[p.apex_idx]).collect();
        assert_eq!(apexes.len(), 2, "{apexes:?}");
        assert!((apexes[0] - 0.45).abs() < 0.011 && (apexes[1] - 0.72).abs() < 0.011);
        // the second peak starts right where the first one ends (the valley minimum)
        assert_eq!(peaks[0].rt_end, peaks[1].rt_start);
    }

    #[test]
    fn second_apex_right_after_the_valley_and_after_a_tail_cut_is_found() {
        // apex, drop below 5 % of the height, immediately a new apex
        let rt: Vec<f64> = (0..12).map(|k| k as f64).collect();
        let y = [0.0, 100.0, 200.0, 1.0, 50.0, 90.0, 40.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let peaks = detect_chrom_peaks(&rt, &y, 1.0, 0.0, 1.0);
        let apexes: Vec<usize> = peaks.iter().map(|p| p.apex_idx).collect();
        assert_eq!(apexes, vec![2, 5]);
        // a rise right after the cut point: apex at the very next sample
        let y = [0.0, 100.0, 200.0, 1.0, 80.0, 30.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let apexes: Vec<usize> = detect_chrom_peaks(&rt, &y, 1.0, 0.0, 1.0)
            .iter()
            .map(|p| p.apex_idx)
            .collect();
        assert_eq!(apexes, vec![2, 4]);
    }

    #[test]
    fn fallback_isotope_peaks_prefer_the_strongest_then_the_closest() {
        let obs = [(100.0, 5.0), (100.0002, 50.0), (100.0001, 50.0), (101.0, 9.0)];
        let ok = |m: f64| (m - 100.0).abs() <= 0.001;
        assert_eq!(strongest(&obs, 100.0, ok), Some((100.0001, 50.0)));
        assert_eq!(strongest(&obs, 100.0, |m| m > 200.0), None);
    }
}
