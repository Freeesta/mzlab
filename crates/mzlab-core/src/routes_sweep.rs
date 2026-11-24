//! The XIC of many m/z in one pass over the m/z-sorted peaks (`Routes::xic_sweep`): every column is, bit for bit, what `xic_raw` gives for that centre alone
//! (same window edges as `search(lo, false)` / `search(hi, true)`, same summation order per scan), without one search and one pass per ion.
//! The matrix of the last sweep is kept in the session (`Routes::sweep`) and goes out with the same LRU as the scan cells.

use std::sync::Arc;

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::routes::{next_tick, trace, Query, Routes};

/// Most centres of one sweep.
pub const MAX_CENTRES: usize = 20_000;

/// The matrix of a sweep: the RT of the scans and one column of intensities per centre (not rounded).
pub struct Sweep {
    pub rt: Vec<f64>,
    pub y: Vec<Vec<f64>>,
}

/// What a sweep was asked: centres and half-windows (bits), level, polarity, smoothing.
type Key = (Vec<u64>, Vec<u64>, u8, Option<i8>, usize, u64);

pub(crate) struct Memo(Key, Arc<Sweep>, pub(crate) u64);

impl Memo {
    pub(crate) fn bytes(&self) -> usize {
        self.1.y.iter().map(|c| c.len() * 8).sum::<usize>() + self.1.rt.len() * 8
    }
}

/// `LO:HI:STEP` as the centres `LO + i*STEP`, each as the text with the decimals of STEP (or of LO if it has more) read back: the numbers `--mz` takes.
/// `Err` is an `err.xic.sweep.*` key.
pub fn parse_sweep(s: &str) -> Result<Vec<f64>> {
    let spec = || Error::new("err.xic.sweep.spec");
    let v: Vec<&str> = s.split(':').collect();
    let n = |t: &str| t.trim().parse::<f64>().ok().filter(|x| x.is_finite());
    let [Some(lo), Some(hi), Some(step)] = v[..].iter().map(|t| n(t)).collect::<Vec<_>>()[..]
    else {
        return Err(spec());
    };
    if v.len() != 3 || step <= 0.0 || lo >= hi || lo <= 0.0 {
        return Err(spec());
    }
    let count = ((hi - lo) / step + 1e-9).floor() + 1.0;
    if count > MAX_CENTRES as f64 {
        return Err(Error::new("err.xic.sweep.size").with("n", format!("{count}")));
    }
    let dec = |x: f64| format!("{x}").split_once('.').map_or(0, |d| d.1.len());
    let d = dec(step).max(dec(lo));
    Ok((0..count as usize)
        .map(|i| {
            format!("{:.d$}", lo + i as f64 * step)
                .parse()
                .unwrap_or(0.0)
        })
        .collect())
}

impl<R: ReadAt> Routes<R> {
    /// The XIC of each of `centres` (half-window `tol` Da) at one level: `(rt, one column per centre in the order given)`, rounded like `xic_into`
    /// (RT to 4 decimals, y to 1).
    pub fn xic_sweep(
        &mut self,
        centres: &[f64],
        tol: f64,
        level: u8,
    ) -> Result<(Vec<f64>, Vec<Vec<f64>>)> {
        let m = self.sweep_matrix(centres, &vec![tol; centres.len()], level, None)?;
        let r = |v: &[f64], d| v.iter().map(|&x| crate::json::round(x, d)).collect();
        Ok((r(&m.rt, 4), m.y.iter().map(|c| r(c, 1)).collect()))
    }

    /// Drops the matrix the last sweep keeps for a repeat call, so a caller sweeping chunk after chunk holds one matrix at a time.
    pub fn forget_sweep(&mut self) {
        self.sweep = None;
    }

    /// The matrix of the sweep, smoothed by `smooth` points per column, from the memo when the last sweep asked the same.
    pub fn sweep_matrix(
        &mut self,
        centres: &[f64],
        tols: &[f64],
        level: u8,
        smooth: Option<usize>,
    ) -> Result<Arc<Sweep>> {
        // the matrix (centres x scans x 8 B) must fit in half of the budget, as `MemBudget::in_memory` says; checked before anything is built
        let bytes = centres
            .len()
            .saturating_mul(self.xic_len(level))
            .saturating_mul(8);
        let budget = crate::budget::MemBudget::mb((crate::budget::TARGET_BYTES >> 20) as f64);
        if !budget.in_memory(bytes) {
            return Err(budget.refuse("xicSweep", bytes));
        }
        let key: Key = (
            centres.iter().map(|c| c.to_bits()).collect(),
            tols.iter().map(|t| t.to_bits()).collect(),
            level,
            self.pol,
            smooth.unwrap_or(0),
            self.mz_off.to_bits(),
        );
        if let Some(m) = self.sweep.as_mut().filter(|m| m.0 == key) {
            m.2 = next_tick();
            return Ok(m.1.clone());
        }
        let m = Arc::new(self.sweep_walk(centres, tols, level, smooth)?);
        self.sweep = Some(Memo(key, m.clone(), next_tick()));
        Ok(m)
    }

    fn sweep_walk(
        &mut self,
        centres: &[f64],
        tols: &[f64],
        level: u8,
        smooth: Option<usize>,
    ) -> Result<Sweep> {
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok(Sweep {
                rt: Vec::new(),
                y: vec![Vec::new(); centres.len()],
            });
        };
        let n = self.idx.sections[si].n_scans;
        // the windows as `search` compares them (a high-resolution survey keeps m/z in f32), in the order of the centres
        let f4 = self.idx.sections[si].f4;
        let cut = |v: f64| if f4 { v as f32 as f64 } else { v };
        let mut order: Vec<usize> = (0..centres.len()).collect();
        crate::sortc::slice_by(&mut order, |&a, &b| centres[a].total_cmp(&centres[b]));
        let off = self.mz_off;
        let lo: Vec<f64> = order
            .iter()
            .map(|&i| cut(centres[i] + off - tols[i]))
            .collect();
        let hi: Vec<f64> = order
            .iter()
            .map(|&i| cut(centres[i] + off + tols[i]))
            .collect();
        // a column is made (zeroed) when its window opens, just before its first sum: a whole matrix zeroed first is cold in the cache when the sums come
        // (it was checked against the budget above)
        let mut sorted: Vec<Vec<f64>> = vec![Vec::new(); centres.len()];
        // windows that do not open and close in order (a negative half-window) are summed one by one, like `xic_raw`
        let ordered = lo.windows(2).all(|w| w[0] <= w[1])
            && hi.windows(2).all(|w| w[0] <= w[1])
            && lo.iter().zip(&hi).all(|(l, h)| l <= h);
        if ordered {
            walk_runs(&mut self.idx, si, n, &lo, &hi, &mut sorted)?;
        } else {
            for (k, col) in sorted.iter_mut().enumerate() {
                let (a, b) = (
                    self.idx.search(si, lo[k], false)?,
                    self.idx.search(si, hi[k], true)?,
                );
                if b > a {
                    *col = self.idx.sum_window(si, a, b)?;
                }
            }
        }
        // back to the order given, then the filter of each column
        let mut y: Vec<Vec<f64>> = vec![Vec::new(); centres.len()];
        for (k, mut col) in sorted.into_iter().enumerate() {
            // a window that never opened (no peak at or above it) is an all-zero column
            if col.is_empty() {
                col = vec![0.0; n];
            }
            y[order[k]] = match smooth {
                Some(s) => crate::peak1d::smooth(&col, s),
                None => col,
            };
        }
        Ok(Sweep {
            rt: self.idx.arrays(si)?.rt.clone(),
            y,
        })
    }

    /// `{"rt":..,"y":..}` of each centre, in the order given: the text `api/xic` gives for it alone.
    pub fn sweep_traces(
        &mut self,
        centres: &[f64],
        tols: &[f64],
        level: u8,
        smooth: Option<usize>,
    ) -> Result<Vec<String>> {
        let m = self.sweep_matrix(centres, tols, level, smooth)?;
        Ok(m.y.iter().map(|y| trace(&m.rt, y)).collect())
    }

    /// `/api/xic?sweep=LO:HI:STEP[&tol][&level][&smooth]`: `trace-set/1` of the engine, one `{"id","mz","tolerance_da","rt","y"}` per centre (`tol` defaults to STEP/2).
    pub(crate) fn sweep_route(
        &mut self,
        spec: &str,
        q: &Query,
        off: Option<&crate::mzcal::Offset>,
    ) -> Result<Option<String>> {
        if q.some("mz").is_some() {
            return Err(Error::new("err.xic.sweep.mz"));
        }
        let centres = parse_sweep(spec)?;
        let step = spec
            .rsplit(':')
            .next()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(1.0);
        let tol = q.num("tol")?.unwrap_or(step / 2.0);
        // the CLI asks for a half-window above 0; the route is the same call, so it takes no other
        if !(tol.is_finite() && tol > 0.0) {
            return Err(Error::new("err.xic.sweep.tol"));
        }
        let smooth = match q.some("smooth") {
            None => None,
            Some("3") => Some(3),
            Some("5") => Some(5),
            Some(_) => return Ok(None),
        };
        let tols = vec![tol; centres.len()];
        let texts = self.sweep_traces(&centres, &tols, q.level(), smooth)?;
        let mut o = format!(
            "{{\"schema\":\"trace-set/1\",\"n_traces\":{}{},\"traces\":[",
            texts.len(),
            off.map_or(String::new(), |f| f.members().replacen('"', ",\"", 1))
        );
        for (i, (c, t)) in centres.iter().zip(&texts).enumerate() {
            let mut num = crate::json::Out::default();
            num.num(*c).raw(",\"tolerance_da\":").num(tol);
            o += &format!(
                "{}{{\"id\":\"xic:{}\",\"mz\":{},{}",
                if i > 0 { "," } else { "" },
                c,
                num.0,
                &t[1..]
            );
        }
        Ok(Some(o + "]}"))
    }
}

/// Fills `w` (one column per window, windows in order) with the sums of the peaks of each window: each window as the peak range `search` gives a single
/// extraction (so the edges are those of `xic_raw` by construction); the m/z array is never read, only positions and intensities, as `sum_window` does.
fn walk_runs<R: ReadAt>(
    idx: &mut crate::idxread::Index<R>,
    si: usize,
    n: usize,
    lo: &[f64],
    hi: &[f64],
    w: &mut [Vec<f64>],
) -> Result<()> {
    let nc = w.len();
    let mut a = Vec::with_capacity(nc);
    let mut b = Vec::with_capacity(nc);
    for k in 0..nc {
        a.push(idx.search(si, lo[k], false)?);
        b.push(idx.search(si, hi[k], true)?);
    }
    // the windows are cut into runs that overlap (peak ranges of centres in order): each run is one walk, so the peaks between two far-apart
    // ions are never read
    let mut c0 = 0;
    while c0 < nc {
        let (start, mut end, mut c1) = (a[c0], b[c0], c0 + 1);
        while c1 < nc && a[c1] < end {
            end = end.max(b[c1]);
            c1 += 1;
        }
        if c1 == c0 + 1 {
            // a window of its own is the single extraction itself
            if end > start {
                w[c0] = idx.sum_window(si, start, end)?;
            }
        } else if end > start {
            walk_run(
                idx,
                si,
                n,
                (&a[c0..c1], &b[c0..c1]),
                (start, end),
                &mut w[c0..c1],
            )?;
        }
        c0 = c1;
    }
    Ok(())
}

/// One run of overlapping windows (`ab`: their peak ranges, `span`: the range they cover) in one walk: the live windows are `first..next`, since ranges
/// open and close in the order of the centres; the first peak of a window makes its column.
fn walk_run<R: ReadAt>(
    idx: &mut crate::idxread::Index<R>,
    si: usize,
    n: usize,
    ab: (&[u64], &[u64]),
    span: (u64, u64),
    w: &mut [Vec<f64>],
) -> Result<()> {
    let (a, b) = ab;
    let nc = w.len();
    let (mut first, mut next, mut j) = (0, 0, span.0);
    let (mut open_at, mut close_at) = (span.0, 0u64);
    idx.each_inten(si, span.0, span.1, |p, y| {
        if j >= open_at || j >= close_at {
            while next < nc && a[next] <= j {
                w[next] = vec![0.0; n];
                next += 1;
            }
            while first < next && b[first] <= j {
                first += 1;
            }
            open_at = if next < nc { a[next] } else { u64::MAX };
            close_at = if first < next { b[first] } else { 0 };
        }
        j += 1;
        if p < n {
            if next == first + 1 {
                w[first][p] += y;
            } else {
                for c in &mut w[first..next] {
                    c[p] += y;
                }
            }
        }
    })
}
