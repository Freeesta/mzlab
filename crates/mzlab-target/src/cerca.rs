// High-resolution mass spectrometry (HRMS) target screening engine.
// Performs targeted substance screening across chromatographic traces, S/N peak detection,
// isotope pattern verification, MS2 precursor matching, and prediction proposal assignment.

use serde::{Deserialize, Serialize};

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

/// Estimate chromatographic baseline noise using Median Absolute Deviation (MAD).
fn estimate_noise(y: &[f64]) -> f64 {
    if y.is_empty() {
        return 1.0;
    }
    let mut sorted = y.to_vec();
    mzlab_core::sortc::by(&mut sorted, |a, b| a.total_cmp(b));
    let mid = sorted.len() / 2;
    let med = if sorted.len().is_multiple_of(2) && sorted.len() > 1 {
        (sorted[mid - 1] + sorted[mid]) * 0.5
    } else {
        sorted[mid]
    };

    let mut dev: Vec<f64> = y.iter().map(|v| (v - med).abs()).collect();
    mzlab_core::sortc::by(&mut dev, |a, b| a.total_cmp(b));
    let dev_mid = dev.len() / 2;
    let mad = if dev.len().is_multiple_of(2) && dev.len() > 1 {
        (dev[dev_mid - 1] + dev[dev_mid]) * 0.5
    } else {
        dev[dev_mid]
    };
    let n = mad * 1.4826;
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
fn add_count(counts: &mut Vec<(String, i64)>, el: &str, delta: i64) {
    if let Some(entry) = counts.iter_mut().find(|(e, _)| e == el) {
        entry.1 += delta;
    } else {
        counts.push((el.to_string(), delta));
    }
}

/// Compute isotope pattern score by comparing theoretical and observed isotopologues.
fn compute_isotope_score(
    neutral_formula: &str,
    adduct: AdductType,
    target_mz: f64,
    spec: &Spectrum,
    tolerance_ppm: f64,
) -> Option<f64> {
    let parsed = parse_formula(neutral_formula).ok()?;
    let mut counts: Vec<(String, i64)> = parsed.into_iter().map(|(el, n)| (el, n as i64)).collect();
    match adduct {
        AdductType::MH => add_count(&mut counts, "H", 1),
        AdductType::MNa => add_count(&mut counts, "Na", 1),
        AdductType::MNH4 => {
            add_count(&mut counts, "N", 1);
            add_count(&mut counts, "H", 4);
        }
    }

    let obs: Vec<(f64, f64)> = spec
        .mz
        .iter()
        .zip(&spec.intensity)
        .filter(|(&m, _)| m >= target_mz - 0.5 && m <= target_mz + 3.5)
        .map(|(&m, &i)| (m, i as f64))
        .collect();

    let opts = mzlab_core::isoscore::IsoOpts {
        r200: 20000.0,
        sigma_ppm: tolerance_ppm,
        sigma_int: 0.25,
        min_rel: 1.0,
        noise_rel: 0.1,
        norm: "mono",
    };

    if let Ok(iso) = mzlab_core::isoscore::isotope_score(&counts, 1, &obs, &opts) {
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

    let m0_peak = obs
        .iter()
        .find(|(m, _)| (m - target_mz).abs() <= target_mz * tolerance_ppm * 1e-6);
    let m1_peak = obs.iter().find(|(m, _)| (m - exp_m1_mz).abs() <= tol);

    match (m0_peak, m1_peak) {
        (Some((_, i0)), Some((m1_obs, i1))) if *i0 > 0.0 => {
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

/// Fast XIC extraction over MS1 spectra for a given m/z window.
fn extract_xic(ms1_spectra: &[&Spectrum], lo: f64, hi: f64) -> (Vec<f64>, f64) {
    let mut y_trace = Vec::with_capacity(ms1_spectra.len());
    let mut max_y = 0.0f64;
    for s in ms1_spectra {
        let mut sum_i = 0.0f64;
        for (&m, &inten) in s.mz.iter().zip(&s.intensity) {
            if m >= lo && m <= hi {
                sum_i += inten as f64;
            }
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
    ms1_spectra: &[&Spectrum],
    ms2_spectra: &[&Spectrum],
    peak: DetectedPeak,
    opts: &TargetSearchOptions,
) -> Option<TargetMatch> {
    let spec = ms1_spectra[peak.apex_idx];
    let tol_da = target_mz * opts.tolerance_ppm * 1e-6;
    let lo = target_mz - tol_da;
    let hi = target_mz + tol_da;

    let mut observed_mz = target_mz;
    let mut max_apex_i = 0.0f32;
    for (&m, &inten) in spec.mz.iter().zip(&spec.intensity) {
        if m >= lo && m <= hi && inten > max_apex_i {
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
    let isotope_score = compute_isotope_score(
        &entry.neutral_formula,
        adduct,
        target_mz,
        spec,
        opts.tolerance_ppm,
    );

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
    let mut matches = Vec::new();

    for entry in list.entries() {
        for &adduct in &opts.adducts {
            let target_mz = entry.mz_for_adduct(adduct);
            let tol_da = target_mz * opts.tolerance_ppm * 1e-6;
            let (y_trace, max_y) =
                extract_xic(&ms1_spectra, target_mz - tol_da, target_mz + tol_da);
            if max_y <= 0.0 {
                continue;
            }

            let noise = estimate_noise(&y_trace);
            let peaks = detect_chrom_peaks(&ms1_rts, &y_trace, opts.min_sn, opts.min_area, noise);

            for peak in peaks {
                if let Some(m) = evaluate_peak(
                    entry,
                    adduct,
                    target_mz,
                    &ms1_spectra,
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
    mzlab_core::sortc::any_by(&mut matches, |a, b| {
        proposal_rank(&a.proposal)
            .cmp(&proposal_rank(&b.proposal))
            .then_with(|| b.sn_ratio.total_cmp(&a.sn_ratio))
            .then_with(|| b.height.total_cmp(&a.height))
    });

    matches
}
