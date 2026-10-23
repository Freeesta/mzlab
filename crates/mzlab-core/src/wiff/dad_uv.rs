//! UV spectrum of the DAD (`/api/uvspec`): the mean absorbance of the optical rows in a window of time, as JSON. Used by the DAD reader of the `.wiff` (browser and native).

use super::dad::DadData;

/// `/api/uvspec`: the UV spectrum (absorbance against wavelength) at the times `rt0`..`rt1`: the mean of the rows inside, or the nearest row when none falls inside. `None` for a non-finite time or an empty matrix.
pub fn dad_spectrum_json(dad: &DadData, rt0: f64, rt1: f64) -> Option<String> {
    if !rt0.is_finite() || !rt1.is_finite() || dad.points() == 0 {
        return None;
    }
    let (lo, hi) = (rt0.min(rt1), rt0.max(rt1));
    let mut rows: Vec<usize> = (0..dad.points())
        .filter(|&i| (lo..=hi).contains(&dad.time_min[i]))
        .collect();
    if rows.is_empty() {
        let mid = (lo + hi) / 2.0;
        let near = (0..dad.points()).min_by(|&a, &b| {
            (dad.time_min[a] - mid)
                .abs()
                .total_cmp(&(dad.time_min[b] - mid).abs())
        })?;
        rows.push(near);
    }
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
    Some(format!(
        "{{\"nm\":[{}],\"mau\":[{}],\"rt0\":{},\"rt1\":{},\"n\":{n},\"unit\":\"mAU\"}}",
        join(&dad.wavelengths_nm, 2),
        join(&mau, 4),
        dad.time_min[rows[0]],
        dad.time_min[rows[n - 1]]
    ))
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
        let j = dad_spectrum_json(&d, 0.01, 0.02).unwrap();
        assert!(
            j.contains("\"nm\":[200.00,201.00,202.00]")
                && j.contains("\"mau\":[4.5000,5.5000,6.5000]")
                && j.contains("\"n\":2")
        );
        // between two rows, reversed bounds: the nearest row (0.03 -> row 3)
        let j = dad_spectrum_json(&d, 0.0305, 0.0295).unwrap();
        assert!(j.contains("\"mau\":[9.0000,10.0000,11.0000]") && j.contains("\"n\":1"));
        assert!(dad_spectrum_json(&d, f64::NAN, 0.0).is_none());
    }
}
