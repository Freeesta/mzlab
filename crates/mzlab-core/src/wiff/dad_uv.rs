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
    let lo = dad
        .wavelengths_nm
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min)
        + END_ABSORPTION_NM;
    // above the skipped range only a band counts (a local maximum), not the tail of the end absorption
    let band = |j: usize| (j == 0 || d[j] >= d[j - 1]) && (j + 1 == nc || d[j] >= d[j + 1]);
    let best = |skip: bool| {
        (0..nc)
            .filter(|&j| !skip || (dad.wavelengths_nm[j] >= lo && band(j)))
            .filter(|&j| d[j] > 0.0)
            .max_by(|&a, &b| d[a].total_cmp(&d[b]).then(b.cmp(&a)))
    };
    let j = best(true).or_else(|| best(false)).unwrap_or(nc / 2);
    let rt = rows
        .get(rows.len() / 2)
        .and_then(|&i| dad.time_min.get(i))
        .copied()?;
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
        (0..np)
            .map(|i| {
                dad.spectrum_at(i)
                    .unwrap_or_default()
                    .iter()
                    .map(|&v| f64::from(v))
                    .sum()
            })
            .collect()
    };
    let e = (np / 20).max(1);
    let mean = |s: &[f64]| s.iter().sum::<f64>() / s.len().max(1) as f64;
    let (a, b) = (mean(&twc[..e]), mean(&twc[np - e..]));
    let line = |i: usize| a + (b - a) * i as f64 / (np - 1).max(1) as f64;
    (0..np)
        .max_by(|&x, &y| {
            (twc[x] - line(x))
                .total_cmp(&(twc[y] - line(y)))
                .then(y.cmp(&x))
        })
        .unwrap_or(0)
}

/// The baseline spectrum: per channel, the median of the rows in the first and the last 5 % of the run (at least one row at each end).
fn baseline_spectrum(dad: &DadData) -> Vec<f64> {
    let (np, nc) = (dad.points(), dad.channels());
    let e = (np / 20).max(1);
    let ends: Vec<usize> = (0..e.min(np))
        .chain(np.saturating_sub(e).max(e.min(np))..np)
        .collect();
    (0..nc)
        .map(|j| {
            let mut v: Vec<f64> = ends
                .iter()
                .filter_map(|&i| dad.spectrum_at(i)?.get(j))
                .map(|&x| f64::from(x))
                .collect();
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
#[path = "dad_uv_tests.rs"]
mod tests;
