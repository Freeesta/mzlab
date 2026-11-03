//! UV spectrum of the DAD (`/api/uvspec`): the mean absorbance of the optical rows in a window of time, as JSON. Used by the DAD reader of the `.wiff` (browser and native).

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
}
