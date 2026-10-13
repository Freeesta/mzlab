//! Regions of interest and ion traces from the `.mzidx`: a port of `ionfamily.build_rois` and `ionfamily.extract_trace` of the Python engine
//! (the reference: same assignments, same running means, same sums in the same order). The peaks of the window are read from the index in
//! m/z order, put in order of scan by a stable counting sort (so inside a scan they stay in m/z order, like `np.argsort(kind="stable")`)
//! and walked scan after scan, as centWave does.

use crate::error::{Error, Result};
use crate::idxread::{Index, ReadAt};

/// Dense extraction of an RT window: `x` is `n_roi` rows of `n_scans` intensities.
#[derive(Debug, Clone, Default)]
pub struct Window {
    pub rt: Vec<f64>,
    pub mz: Vec<f64>,
    pub x: Vec<f64>,
    pub n_scans: usize,
    pub scan_lo: usize,
    pub mz_lo: Option<f64>,
    pub mz_hi: Option<f64>,
    pub n_peaks: u64,
}

impl Window {
    pub fn n_roi(&self) -> usize {
        self.mz.len()
    }
}

fn lower(a: &[f64], v: f64) -> usize {
    a.partition_point(|x| *x < v)
}
fn upper(a: &[f64], v: f64) -> usize {
    a.partition_point(|x| *x <= v)
}

/// `ionfamily.build_rois` on the section `si` of the index (the table of one level and polarity).
#[allow(clippy::too_many_arguments)]
pub fn build_rois<R: ReadAt>(
    idx: &mut Index<R>,
    si: usize,
    rt0: f64,
    rt1: f64,
    tol: f64,
    min_scans: i64,
    max_gap: i64,
    mz_range: Option<(f64, f64)>,
    min_int: f64,
) -> Result<Window> {
    let rt_all = idx.arrays(si)?.rt.clone();
    let f4 = idx.sections[si].f4;
    let lo = lower(&rt_all, rt0);
    let hi = upper(&rt_all, rt1).max(lo);
    let n = hi - lo;
    let rt = rt_all[lo..hi].to_vec();
    let n_peaks_all = idx.sections[si].n_peaks;
    if n == 0 || n_peaks_all == 0 {
        return Ok(Window {
            rt,
            n_scans: n,
            scan_lo: lo,
            ..Default::default()
        });
    }
    // the peaks of scans lo..hi, by scan (stable): two passes over the index, count and place
    let mut off = vec![0usize; n + 1];
    idx.each_peak(si, 0, n_peaks_all, |p, _m, _y| {
        if p >= lo && p < hi {
            off[p - lo + 1] += 1;
        }
    })?;
    for i in 0..n {
        off[i + 1] += off[i];
    }
    let total = off[n];
    let mut mz = Vec::new();
    let mut y = Vec::new();
    mz.try_reserve_exact(total)
        .map_err(|_| Error::memory_budget())?;
    y.try_reserve_exact(total)
        .map_err(|_| Error::memory_budget())?;
    mz.resize(total, 0.0);
    y.resize(total, 0.0);
    let mut fill = off.clone();
    idx.each_peak(si, 0, n_peaks_all, |p, m, it| {
        if p >= lo && p < hi {
            let k = fill[p - lo];
            mz[k] = m;
            y[k] = it;
            fill[p - lo] += 1;
        }
    })?;
    drop(fill);
    if total == 0 {
        return Ok(Window {
            rt,
            n_scans: n,
            scan_lo: lo,
            ..Default::default()
        });
    }
    let (rlo, rhi) = match mz_range {
        // a Python float against a float32 array is read as float32
        Some((a, b)) if f4 => (Some(a as f32 as f64), Some(b as f32 as f64)),
        Some((a, b)) => (Some(a), Some(b)),
        None => (None, None),
    };
    let keep: Vec<bool> = (0..total)
        .map(|i| y[i] > min_int && rlo.is_none_or(|a| mz[i] >= a) && rhi.is_none_or(|b| mz[i] <= b))
        .collect();
    let n_kept = keep.iter().filter(|k| **k).count() as u64;

    let mut r_mz: Vec<f64> = Vec::new();
    let mut r_cnt: Vec<i64> = Vec::new();
    let mut r_last: Vec<i64> = Vec::new();
    // assignments: (roi, scan, y, mz) in the order they were made
    let mut a_roi: Vec<i64> = Vec::new();
    let mut a_scan: Vec<usize> = Vec::new();
    let mut a_y: Vec<f64> = Vec::new();
    let mut a_mz: Vec<f64> = Vec::new();
    for s in 0..n {
        let (a, b) = (off[s], off[s + 1]);
        if b <= a {
            continue;
        }
        let (pm, py): (Vec<f64>, Vec<f64>) =
            (a..b).filter(|&i| keep[i]).map(|i| (mz[i], y[i])).unzip();
        let m = pm.len();
        if m == 0 {
            continue;
        }
        let mut assigned = vec![-1i64; m];
        let mut extra = vec![false; m];
        let n_roi = r_mz.len();
        let min_last = s as i64 - max_gap - 1;
        let mut act: Vec<usize> = (0..n_roi).filter(|&r| r_last[r] >= min_last).collect();
        let mut near = vec![false; m];
        if !act.is_empty() {
            crate::sortc::by(&mut act, |&p, &q| r_mz[p].total_cmp(&r_mz[q]));
            let am: Vec<f64> = act.iter().map(|&r| r_mz[r]).collect();
            let last = am.len() - 1;
            let mut jb = vec![0usize; m];
            for i in 0..m {
                let j = lower(&am, pm[i]);
                let jl = j.saturating_sub(1).min(last);
                let jr = j.min(last);
                let (dl, dr) = ((pm[i] - am[jl]).abs(), (pm[i] - am[jr]).abs());
                jb[i] = if dl <= dr { jl } else { jr };
                near[i] = dl.min(dr) <= tol;
            }
            let mut order: Vec<usize> = (0..m).filter(|&i| near[i]).collect();
            if !order.is_empty() {
                crate::sortc::by(&mut order, |&p, &q| py[q].total_cmp(&py[p])); // stable: -py ascending
                                                                                // the first (strongest) centroid of each target ROI
                let mut seen: Vec<usize> = Vec::new();
                let mut strong = vec![false; m];
                for &i in &order {
                    let t = act[jb[i]];
                    if !seen.contains(&t) {
                        seen.push(t);
                        strong[i] = true;
                    }
                }
                for &i in &order {
                    assigned[i] = act[jb[i]] as i64;
                    if !strong[i] {
                        extra[i] = true;
                    }
                }
            }
        }
        // a centroid within tol of an active ROI never starts a new ROI
        let mut next = n_roi as i64;
        for i in 0..m {
            if !near[i] {
                r_mz.push(pm[i]);
                r_cnt.push(0);
                r_last.push(-1_000_000_000);
                assigned[i] = next;
                next += 1;
            }
        }
        for i in 0..m {
            if !extra[i] {
                let r = assigned[i] as usize;
                r_mz[r] = (r_mz[r] * r_cnt[r] as f64 + pm[i]) / (r_cnt[r] as f64 + 1.0);
                r_cnt[r] += 1;
            }
        }
        for i in 0..m {
            r_last[assigned[i] as usize] = s as i64;
            a_roi.push(assigned[i]);
            a_scan.push(s);
            a_y.push(py[i]);
            a_mz.push(pm[i]);
        }
    }
    let n_roi = r_mz.len();
    let mz_min = mz.iter().cloned().fold(f64::INFINITY, f64::min);
    let mz_max = mz.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if n_roi == 0 || a_roi.is_empty() {
        return Ok(Window {
            rt,
            n_scans: n,
            scan_lo: lo,
            ..Default::default()
        });
    }
    let mut remap = vec![-1i64; n_roi];
    let mut m_good = 0usize;
    for r in 0..n_roi {
        if r_cnt[r] >= min_scans {
            remap[r] = m_good as i64;
            m_good += 1;
        }
    }
    let mut x = Vec::new();
    x.try_reserve_exact(m_good * n)
        .map_err(|_| Error::memory_budget())?;
    x.resize(m_good * n, 0.0);
    let mut wsum = vec![0.0f64; m_good];
    for k in 0..a_roi.len() {
        let g = remap[a_roi[k] as usize];
        if g >= 0 {
            x[g as usize * n + a_scan[k]] += a_y[k];
            // the weight of the centre is a product of two stored values: float32 for a float32 table
            wsum[g as usize] += if f4 {
                ((a_mz[k] as f32) * (a_y[k] as f32)) as f64
            } else {
                a_mz[k] * a_y[k]
            };
        }
    }
    let mut centre: Vec<f64> = (0..m_good)
        .map(|g| wsum[g] / x[g * n..(g + 1) * n].iter().sum::<f64>().max(1e-12))
        .collect();
    let mut ord: Vec<usize> = (0..m_good).collect();
    crate::sortc::by(&mut ord, |&p, &q| centre[p].total_cmp(&centre[q]));
    let mut xs = Vec::with_capacity(x.len());
    for &g in &ord {
        xs.extend_from_slice(&x[g * n..(g + 1) * n]);
    }
    centre = ord.iter().map(|&g| centre[g]).collect();
    Ok(Window {
        rt,
        mz: centre,
        x: xs,
        n_scans: n,
        scan_lo: lo,
        mz_lo: Some(mz_min),
        mz_hi: Some(mz_max),
        n_peaks: n_kept,
    })
}

/// `ionfamily.extract_trace`: `(rt, intensity)` of one ion, the sum of the centroids within `mz ± tol` per scan, in the window (all the run if `None`).
pub fn extract_trace<R: ReadAt>(
    idx: &mut Index<R>,
    si: usize,
    mz: f64,
    tol: f64,
    window: Option<(f64, f64)>,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let a = idx.search(si, mz - tol, false)?;
    let b = idx.search(si, mz + tol, true)?;
    let n = idx.sections[si].n_scans;
    let y = if b <= a {
        vec![0.0; n]
    } else {
        idx.sum_window(si, a, b)?
    };
    let rt = idx.arrays(si)?.rt.clone();
    Ok(match window {
        None => (rt, y),
        Some((t0, t1)) => {
            let (i0, i1) = (lower(&rt, t0), upper(&rt, t1));
            (rt[i0..i1.max(i0)].to_vec(), y[i0..i1.max(i0)].to_vec())
        }
    })
}
