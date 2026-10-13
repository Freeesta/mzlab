//! Gap filling: port of `TP_Mine/py/tpmine/hr/features.py::fill_gaps` and `trace_area`. A cell of the aligned table where the detection found no
//! peak (`CELL_ABSENT`) is integrated again at the intensity floor, in the window of m/z of the reference ion of the row and in the window of RT where
//! the peak is expected (from the RT map, or from the files where the row was seen). One file is open at a time; the cells of a file are served
//! in slabs of m/z (`sweep::windows`). A filled cell gets `CELL_FILLED`: it is never counted as detected.

use crate::sortc;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::rtmap::RtMap;
use crate::sweep::{ppm_ranges, windows};
use crate::table::{Aligned, CELL_ABSENT, CELL_FILLED};
use crate::trace::{Params, SLAB_PEAKS};

/// Parameters of the integration (defaults of `fill_gaps` and `trace_area`).
#[derive(Debug, Clone, Copy)]
pub struct FillParams {
    pub floor: f64,
    pub ppm: f64,
    /// Half width (minutes) added to the window of RT.
    pub rt_tol: f64,
    pub min_points: usize,
    pub max_gap: usize,
    /// A trace longer than this many scans is background and is not taken.
    pub long_trace: usize,
}

impl Default for FillParams {
    fn default() -> Self {
        FillParams {
            floor: 2e4,
            ppm: 5.0,
            rt_tol: 0.1,
            min_points: 5,
            max_gap: 2,
            long_trace: 400,
        }
    }
}

/// `np.median` of values already sorted by the caller's `total_cmp` (mean of the two middle ones for an even count).
fn median(mut v: Vec<f64>) -> f64 {
    sortc::by(&mut v, f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// The same in `f32` (numpy keeps the dtype of the array).
fn median32(mut v: Vec<f32>) -> f32 {
    sortc::by(&mut v, f32::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Where the peak of `row` is expected in `file`: (centre, half width) in minutes, `None` when no file saw the row.
pub(crate) fn expected_window(
    al: &Aligned,
    row: usize,
    file: usize,
    map: Option<&RtMap>,
    p: &FillParams,
) -> Option<(f64, f64)> {
    let nf = al.n_files;
    let seen: Vec<usize> = (0..nf)
        .filter(|&j| !al.apex_rt[row * nf + j].is_nan())
        .collect();
    if seen.is_empty() {
        return None;
    }
    if let Some(m) = map.filter(|m| m.ok()) {
        let refs = seen
            .iter()
            .map(|&j| m.reference(al.apex_rt[row * nf + j] as f64, j))
            .collect();
        return Some((m.expected(median(refs), file), p.rt_tol + 2.0 * m.rms[file]));
    }
    let centre = median32(seen.iter().map(|&j| al.apex_rt[row * nf + j]).collect());
    let width = median32(seen.iter().map(|&j| al.width[row * nf + j]).collect());
    Some((centre as f64, width as f64 / 2.0 + p.rt_tol))
}

/// Area (counts x seconds) of the strongest trace whose apex lies in the scans `i0..i1`; `y` holds the summed intensity of the scans `s0..`.
pub(crate) fn trace_area(
    y: &[f64],
    s0: usize,
    i0: usize,
    i1: usize,
    dt: f64,
    p: &FillParams,
) -> Option<f64> {
    let x: Vec<f64> = y
        .iter()
        .map(|&v| if v >= p.floor { v } else { 0.0 })
        .collect();
    let (a0, a1) = (i0 - s0, i1 - s0);
    let mut apex = None;
    for j in a0..a1 {
        if x[j] > 0.0 && apex.is_none_or(|b| x[j] > x[b]) {
            apex = Some(j);
        }
    }
    let apex = apex?;
    let (mut a, mut z, mut miss) = (apex, apex, 0usize);
    for j in (0..apex).rev() {
        miss = if x[j] > 0.0 { 0 } else { miss + 1 };
        if miss > p.max_gap {
            break;
        }
        if x[j] > 0.0 {
            a = j;
        }
    }
    miss = 0;
    for (j, &v) in x.iter().enumerate().skip(apex + 1) {
        miss = if v > 0.0 { 0 } else { miss + 1 };
        if miss > p.max_gap {
            break;
        }
        if v > 0.0 {
            z = j;
        }
    }
    let seg = &x[a..=z];
    if seg.iter().filter(|&&v| v > 0.0).count() < p.min_points || z - a + 1 > p.long_trace {
        return None;
    }
    Some(seg.iter().sum::<f64>() * dt)
}

/// A cell to integrate: row, window of m/z of its reference ion and window of scans where the apex must lie.
struct Todo {
    row: usize,
    mz: f64,
    i0: usize,
    i1: usize,
}

/// The cells of `file` to integrate, sorted by m/z (then by row).
fn todo(al: &Aligned, file: usize, rt: &[f64], map: Option<&RtMap>, p: &FillParams) -> Vec<Todo> {
    let nf = al.n_files;
    let mut out = Vec::new();
    for row in (0..al.len()).filter(|&r| al.state[r * nf + file] == CELL_ABSENT) {
        let Some((c, w)) = expected_window(al, row, file, map, p) else {
            continue;
        };
        let i0 = rt.partition_point(|&r| r < c - w);
        let i1 = rt.partition_point(|&r| r <= c + w);
        if i1 > i0 {
            out.push(Todo {
                row,
                mz: al.mz[row],
                i0,
                i1,
            });
        }
    }
    sortc::any_by(&mut out, |a, b| {
        a.mz.total_cmp(&b.mz).then(a.row.cmp(&b.row))
    });
    out
}

/// Integrates the cells of `file` that have no peak, in the MS1 section of `index` (one polarity, `params.polarity`). Returns the number of cells
/// filled. `map`: the RT map of the series (`None` = expected RT from the median apex of the files that saw the row).
pub fn fill_file<R: ReadAt>(
    index: &mut Index<R>,
    al: &mut Aligned,
    file: usize,
    map: Option<&RtMap>,
    params: &Params,
    p: &FillParams,
) -> Result<usize> {
    let Some(si) = index.section_of(1, params.polarity) else {
        return Ok(0);
    };
    let rt: Vec<f64> = index.arrays(si)?.rt.clone();
    let nscan = rt.len();
    if nscan < 3 {
        return Ok(0);
    }
    let dt = median(rt.windows(2).map(|w| w[1] - w[0]).collect()) * 60.0;
    let list = todo(al, file, &rt, map, p);
    let ranges = ppm_ranges(&list.iter().map(|t| t.mz).collect::<Vec<_>>(), p.ppm);
    let pad = p.long_trace + p.max_gap + 2;
    let max_peaks = (params.budget / 24).clamp(1024, SLAB_PEAKS);
    let mut found: Vec<(usize, f64)> = Vec::new();
    windows(index, si, &ranges, max_peaks, |q, a, b, slab| {
        let t = &list[q];
        let (s0, s1) = (t.i0.saturating_sub(pad), (t.i1 + pad).min(nscan));
        let mut y = vec![0.0f64; s1 - s0];
        for j in a..b {
            let s = slab.pos[j] as usize;
            if s >= s0 && s < s1 {
                y[s - s0] += slab.it[j];
            }
        }
        if let Some(area) = trace_area(&y, s0, t.i0, t.i1, dt, p).filter(|&v| v > 0.0) {
            found.push((t.row, area));
        }
        Ok(())
    })?;
    let nf = al.n_files;
    for &(row, area) in &found {
        al.area[row * nf + file] = area;
        al.state[row * nf + file] = CELL_FILLED;
    }
    Ok(found.len())
}
