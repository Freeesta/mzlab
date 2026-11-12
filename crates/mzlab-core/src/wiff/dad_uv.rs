//! UV spectrum of the DAD (`/api/uvspec`): the mean absorbance of the optical rows in a window of time, as JSON; the wavelength trace (`/api/uvxic`) and the
//! band proposed for an XWC (`/api/uvpropose`). Used by the DAD reader of the `.wiff` and by the mzML matrix (browser and native).

use super::dad::DadData;

/// `/api/uvspec`: the UV spectrum (absorbance against wavelength) at the times `rt0`..`rt1`: the mean of the rows inside, or the nearest row when none falls inside. `None` for a non-finite time or an empty matrix.
pub fn dad_spectrum_json(dad: &DadData, rt0: f64, rt1: f64) -> Option<String> {
    if !rt0.is_finite() || !rt1.is_finite() {
        return None;
    }
    let rows = window_rows(dad, rt0.min(rt1), rt0.max(rt1))?;
    let mut mean = vec![0.0f64; dad.channels()];
    for &i in &rows {
        for (m, &v) in mean.iter_mut().zip(dad.spectrum_at(i)?) {
            *m += f64::from(v);
        }
    }
    let join = |v: &[f64], d: usize| {
        v.iter()
            .map(|x| format!("{x:.d$}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let n = rows.len();
    let mau: Vec<f64> = mean.iter().map(|m| m / n as f64).collect();
    let time_of = |i: Option<&usize>| i.and_then(|&i| dad.time_min.get(i)).copied();
    Some(format!(
        "{{\"nm\":[{}],\"mau\":[{}],\"rt0\":{},\"rt1\":{},\"n\":{n},\"unit\":\"mAU\"}}",
        join(&dad.wavelengths_nm, 2),
        join(&mau, 4),
        time_of(rows.first())?,
        time_of(rows.last())?
    ))
}

/// `/api/uvxic`: the trace of the DAD at the wavelengths `nm0`..`nm1` (mean of the channels inside, or the nearest channel when none falls inside), as `{rt, y, nm0, nm1, n}`. `None` for a non-finite wavelength or an empty matrix.
pub fn dad_wavelength_json(dad: &DadData, nm0: f64, nm1: f64) -> Option<String> {
    if !nm0.is_finite() || !nm1.is_finite() {
        return None;
    }
    let cols = window_cols(dad, nm0.min(nm1), nm0.max(nm1))?;
    let y: Vec<String> = (0..dad.points())
        .map(|i| {
            let row = dad.spectrum_at(i).unwrap_or_default();
            let sum: f64 = cols
                .iter()
                .filter_map(|&j| row.get(j))
                .map(|&v| f64::from(v))
                .sum();
            format!("{:.4}", sum / cols.len() as f64)
        })
        .collect();
    let rt: Vec<String> = dad.time_min.iter().map(|t| format!("{t:.4}")).collect();
    let nm_of = |j: Option<&usize>| j.and_then(|&j| dad.wavelengths_nm.get(j)).copied();
    Some(format!(
        "{{\"rt\":[{}],\"y\":[{}],\"nm0\":{},\"nm1\":{},\"n\":{}}}",
        rt.join(","),
        y.join(","),
        nm_of(cols.first())?,
        nm_of(cols.last())?,
        cols.len()
    ))
}

/// Width of the proposed band (nm): λ ± 2 nm, the usual signal bandwidth of a DAD.
const PROPOSE_BW: f64 = 4.0;
/// The lowest wavelengths skipped by the λmax search (nm): end absorption of solvent and buffer, not a band of the compound.
const END_ABSORPTION_NM: f64 = 10.0;

/// `/api/uvpropose`: the band of an XWC proposed from the data, `{nm, bw, rt, n}`. Without a window (`rt0`/`rt1` not finite) the spectrum is the one at the
/// apex of the strongest TWC peak (above the straight line between the run ends); with a window, the mean spectrum of that window. The baseline spectrum
/// (median of the rows at the run ends) is subtracted, and λmax skips the lowest 10 nm unless no band rises above them; a flat matrix proposes the middle
/// of the range. `nm` is a channel of the matrix, `n` the rows of the spectrum. A default to start from, never an identification. `None` for an empty matrix.
pub fn dad_propose_json(dad: &DadData, rt0: f64, rt1: f64) -> Option<String> {
    let (np, nc) = (dad.points(), dad.channels());
    if np == 0 || nc == 0 || dad.matrix_mau.len() < np * nc {
        return None;
    }
    let rows = if rt0.is_finite() && rt1.is_finite() {
        window_rows(dad, rt0.min(rt1), rt0.max(rt1))?
    } else {
        let apex = twc_apex(dad);
        (apex.saturating_sub(1)..=(apex + 1).min(np - 1)).collect()
    };
    let base = baseline_spectrum(dad);
    let mut d = vec![0.0f64; nc];
    for &i in &rows {
        for (x, &v) in d.iter_mut().zip(dad.spectrum_at(i)?) {
            *x += f64::from(v);
        }
    }
    for (x, b) in d.iter_mut().zip(&base) {
        *x = *x / rows.len() as f64 - b;
    }
    let lo = dad.wavelengths_nm.iter().copied().fold(f64::INFINITY, f64::min) + END_ABSORPTION_NM;
    // above the skipped range only a band counts (a local maximum), not the tail of the end absorption
    let band = |j: usize| (j == 0 || d[j] >= d[j - 1]) && (j + 1 == nc || d[j] >= d[j + 1]);
    let best = |skip: bool| {
        (0..nc)
            .filter(|&j| !skip || (dad.wavelengths_nm[j] >= lo && band(j)))
            .filter(|&j| d[j] > 0.0)
            .max_by(|&a, &b| d[a].total_cmp(&d[b]).then(b.cmp(&a)))
    };
    let j = best(true).or_else(|| best(false)).unwrap_or(nc / 2);
    let rt = rows.get(rows.len() / 2).and_then(|&i| dad.time_min.get(i)).copied()?;
    Some(format!(
        "{{\"nm\":{},\"bw\":{PROPOSE_BW},\"rt\":{rt:.4},\"n\":{}}}",
        dad.wavelengths_nm[j],
        rows.len()
    ))
}

/// The row of the strongest TWC peak: the largest excess over the straight line between the means of the first and the last 5 % of the run.
fn twc_apex(dad: &DadData) -> usize {
    let np = dad.points();
    let twc: Vec<f64> = if dad.twc_mau.len() == np {
        dad.twc_mau.clone()
    } else {
        (0..np).map(|i| dad.spectrum_at(i).unwrap_or_default().iter().map(|&v| f64::from(v)).sum()).collect()
    };
    let e = (np / 20).max(1);
    let mean = |s: &[f64]| s.iter().sum::<f64>() / s.len().max(1) as f64;
    let (a, b) = (mean(&twc[..e]), mean(&twc[np - e..]));
    let line = |i: usize| a + (b - a) * i as f64 / (np - 1).max(1) as f64;
    (0..np).max_by(|&x, &y| (twc[x] - line(x)).total_cmp(&(twc[y] - line(y))).then(y.cmp(&x))).unwrap_or(0)
}

/// The baseline spectrum: per channel, the median of the rows in the first and the last 5 % of the run (at least one row at each end).
fn baseline_spectrum(dad: &DadData) -> Vec<f64> {
    let (np, nc) = (dad.points(), dad.channels());
    let e = (np / 20).max(1);
    let ends: Vec<usize> = (0..e.min(np)).chain(np.saturating_sub(e).max(e.min(np))..np).collect();
    (0..nc)
        .map(|j| {
            let mut v: Vec<f64> = ends.iter().filter_map(|&i| dad.spectrum_at(i)?.get(j)).map(|&x| f64::from(x)).collect();
            v.sort_by(f64::total_cmp);
            match v.len() {
                0 => 0.0,
                n if n % 2 == 1 => v[n / 2],
                n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
            }
        })
        .collect()
}

/// The channels with a wavelength in `lo..=hi`, or the single one nearest to the middle when none falls inside; `None` for an empty matrix.
fn window_cols(dad: &DadData, lo: f64, hi: f64) -> Option<Vec<usize>> {
    let cols: Vec<usize> = dad
        .wavelengths_nm
        .iter()
        .enumerate()
        .filter(|(_, w)| (lo..=hi).contains(w))
        .map(|(j, _)| j)
        .collect();
    if !cols.is_empty() {
        return Some(cols);
    }
    let mid = (lo + hi) / 2.0;
    let dist = |w: &f64| (w - mid).abs();
    Some(vec![
        dad.wavelengths_nm
            .iter()
            .enumerate()
            .min_by(|a, b| dist(a.1).total_cmp(&dist(b.1)))?
            .0,
    ])
}

/// The rows with a time in `lo..=hi`, or the single row nearest to the middle when none falls inside; `None` for an empty matrix.
fn window_rows(dad: &DadData, lo: f64, hi: f64) -> Option<Vec<usize>> {
    let rows: Vec<usize> = dad
        .time_min
        .iter()
        .enumerate()
        .filter(|(_, t)| (lo..=hi).contains(t))
        .map(|(i, _)| i)
        .collect();
    if !rows.is_empty() {
        return Some(rows);
    }
    let mid = (lo + hi) / 2.0;
    let dist = |t: &f64| (t - mid).abs();
    let near = dad
        .time_min
        .iter()
        .enumerate()
        .min_by(|a, b| dist(a.1).total_cmp(&dist(b.1)))?;
    Some(vec![near.0])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dad(nt: usize, nl: usize) -> DadData {
        DadData {
            time_min: (0..nt).map(|i| i as f64 * 0.01).collect(),
            twc_mau: vec![0.0; nt],
            wavelengths_nm: (0..nl).map(|j| 200.0 + j as f64).collect(),
            wl_start_nm: 200.0,
            wl_stop_nm: 200.0 + (nl - 1) as f64,
            wl_step_nm: 1.0,
            duration_min: nt as f64 * 0.01,
            matrix_mau: (0..nt * nl).map(|k| k as f32).collect(),
        }
    }

    #[test]
    fn uv_spectrum_is_the_mean_of_the_window_or_the_nearest_row() {
        let d = dad(5, 3);
        // rows 1 and 2 (0.01, 0.02): means of [3,4,5] and [6,7,8]
        let j = dad_spectrum_json(&d, 0.01, 0.02).unwrap_or_default();
        assert!(j.contains("\"nm\":[200.00,201.00,202.00]"), "{j}");
        assert!(
            j.contains("\"mau\":[4.5000,5.5000,6.5000]") && j.contains("\"n\":2"),
            "{j}"
        );
        // between two rows, reversed bounds: the nearest row (0.03 -> row 3)
        let j = dad_spectrum_json(&d, 0.0305, 0.0295).unwrap_or_default();
        assert!(
            j.contains("\"mau\":[9.0000,10.0000,11.0000]") && j.contains("\"n\":1"),
            "{j}"
        );
        assert!(dad_spectrum_json(&d, f64::NAN, 0.0).is_none());
    }

    #[test]
    fn wavelength_trace_is_the_mean_of_the_channels_in_range() {
        let d = dad(3, 3);
        // channels 0 and 1 (200, 201): rows [0,1,2] -> 0.5, [3,4,5] -> 3.5, [6,7,8] -> 6.5
        let j = dad_wavelength_json(&d, 201.0, 200.0).unwrap_or_default();
        assert!(
            j.contains("\"y\":[0.5000,3.5000,6.5000]") && j.contains("\"n\":2"),
            "{j}"
        );
        // no channel inside: the nearest one (202)
        let j = dad_wavelength_json(&d, 250.0, 260.0).unwrap_or_default();
        assert!(
            j.contains("\"y\":[2.0000,5.0000,8.0000]") && j.contains("\"nm0\":202"),
            "{j}"
        );
        assert!(dad_wavelength_json(&d, f64::NAN, 0.0).is_none());
    }
    /// A run of `nt` rows at 0.01 min, 200..=299 nm: a Gaussian elution at row `at` whose spectrum is `band(nm)`, plus `end(nm)` on every row.
    fn synth(nt: usize, at: f64, band: impl Fn(f64) -> f64, end: impl Fn(f64) -> f64) -> DadData {
        let nm: Vec<f64> = (0..100).map(|j| 200.0 + j as f64).collect();
        let mut m = Vec::with_capacity(nt * nm.len());
        for i in 0..nt {
            let c = (-((i as f64 - at) / 4.0).powi(2) / 2.0).exp();
            m.extend(nm.iter().map(|&w| (c * band(w) + end(w)) as f32));
        }
        DadData {
            time_min: (0..nt).map(|i| i as f64 * 0.01).collect(),
            twc_mau: vec![],
            wl_start_nm: 200.0,
            wl_stop_nm: 299.0,
            wl_step_nm: 1.0,
            duration_min: nt as f64 * 0.01,
            wavelengths_nm: nm,
            matrix_mau: m,
        }
    }
    fn gauss(w: f64, c: f64, s: f64, h: f64) -> f64 {
        h * (-((w - c) / s).powi(2) / 2.0).exp()
    }

    #[test]
    fn proposal_is_the_lambda_max_of_the_strongest_peak() {
        // two compounds: a strong one at row 30 (band at 274 nm) and a weak one at row 70 (band at 230 nm)
        let d = synth(100, 30.0, |w| gauss(w, 274.0, 6.0, 300.0), |_| 0.0);
        let mut d2 = synth(100, 70.0, |w| gauss(w, 230.0, 6.0, 40.0), |_| 0.0);
        for (a, b) in d2.matrix_mau.iter_mut().zip(&d.matrix_mau) {
            *a += b;
        }
        let j = dad_propose_json(&d2, f64::NAN, f64::NAN).unwrap_or_default();
        assert_eq!(j, "{\"nm\":274,\"bw\":4,\"rt\":0.3000,\"n\":3}");
        // a window on the weak compound: its own λmax
        let j = dad_propose_json(&d2, 0.69, 0.71).unwrap_or_default();
        assert!(j.starts_with("{\"nm\":230,\"bw\":4,"), "{j}");
    }

    #[test]
    fn end_absorption_is_skipped_unless_nothing_else_absorbs() {
        // the compound absorbs most at 200 nm (end absorption) and has a real band at 254 nm
        let d = synth(100, 50.0, |w| gauss(w, 200.0, 3.0, 900.0) + gauss(w, 254.0, 5.0, 120.0), |_| 0.0);
        assert!(dad_propose_json(&d, f64::NAN, f64::NAN).unwrap_or_default().starts_with("{\"nm\":254,"));
        // only end absorption: it is the answer
        let d = synth(100, 50.0, |w| gauss(w, 200.0, 3.0, 900.0), |_| 0.0);
        assert!(dad_propose_json(&d, f64::NAN, f64::NAN).unwrap_or_default().starts_with("{\"nm\":200,"));
        // a constant solvent background (baseline) is removed before the search
        let d = synth(100, 50.0, |w| gauss(w, 280.0, 5.0, 50.0), |w| gauss(w, 240.0, 8.0, 500.0));
        assert!(dad_propose_json(&d, f64::NAN, f64::NAN).unwrap_or_default().starts_with("{\"nm\":280,"));
    }

    #[test]
    fn a_flat_matrix_proposes_the_middle_and_an_empty_one_nothing() {
        let d = synth(40, 20.0, |_| 0.0, |_| 5.0);
        assert!(dad_propose_json(&d, f64::NAN, f64::NAN).unwrap_or_default().starts_with("{\"nm\":250,\"bw\":4,"));
        assert!(dad_propose_json(&dad(0, 3), f64::NAN, f64::NAN).is_none());
    }
}
