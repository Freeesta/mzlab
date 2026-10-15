//! Gap filling: rewritten from scratch after the peak finder of mzmine 3.9.0 `modules/dataprocessing/gapfill_peakfinder/Gap.java` and
//! `multithreaded/MultiThreadPeakFinderTask.java`, with `datamodel/featuredata/FeatureDataUtils.java` for the feature numbers (MIT, (c) 2004-2022 The mzmine
//! Development Team); no code copied. Paper: MZmine 3, Schmid et al., Nat. Biotechnol. 41:447 (2023), doi:10.1038/s41587-023-01690-2.
//!
//! What 3.9.0 does for each empty cell of a row, in the scans of the file that the feature list selected (MS1, one polarity), on the mass list (the
//! peaks above the noise level of the mass detector):
//! 1. m/z range = row m/z +- max(Da, ppm); RT range = row RT +- tolerance (closed ranges, RT a `float`). Each scan inside the RT range gives one point:
//!    the most intense peak inside the m/z range (the first on a tie), or intensity 0 at the centre of the range when the scan has none.
//! 2. The points of consecutive scans form one candidate peak (the "RT shape" test of mzmine is always true for scans already inside the range). Its
//!    apex is the most intense interior point (not the first or last) that is not lower than its two neighbours; the first wins on a tie and a zero
//!    never wins. No apex, no peak.
//! 3. Walking left, a point joins while it is not above the current one by more than `intensity_tol` (`current >= next * (1 - tol)`), and stops after
//!    the first zero (the zero is kept). Walking right it joins while the next point is not above the current one by more than the tolerance
//!    (`next <= current * (1 + tol)`), and stops after the first zero (kept). This is how "the shape stays within `intensityTolerance`" is decided: it
//!    is a test point by point, not a comparison with an expected curve.
//! 4. The peak is accepted when it has at least `min_points` points. (mzmine keeps `bestPeakHeight` at 0, so the last candidate would win; with the
//!    contiguous scans of one window there is a single candidate.)
//! 5. The feature: m/z = mean weighted by intensity, RT and height = the most intense point (the first), area = trapezoid over the RT of the points
//!    with the sums done in `float` like mzmine (intensity x minutes), range = first and last point.
//!
//! The points are read from the `.mzidx` of the file by m/z window (the peak table is sorted by m/z: two binary searches and one pass over the window),
//! never the whole file. The filler of the current pipeline (`crate::gapfill`) is another algorithm and is not touched.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use super::join::Rows;
use super::Feature;

/// Parameters of the filler (the batch of the gold data: intensity tolerance 0.2, 0.0015 Da or 5 ppm, 0.2 min, at least one point).
#[derive(Debug, Clone, Copy)]
pub struct GapParams {
    pub intensity_tol: f64,
    pub tol_da: f64,
    pub tol_ppm: f64,
    pub rt_tol: f64,
    pub min_points: usize,
    /// Peaks under this intensity are not in the mass list (noise level of the mass detector); 0 keeps every peak.
    pub noise: f64,
    /// Polarity of the scans that the feature list used (`None` = all).
    pub polarity: Option<i8>,
}

impl Default for GapParams {
    fn default() -> Self {
        GapParams {
            intensity_tol: 0.2,
            tol_da: 0.0015,
            tol_ppm: 5.0,
            rt_tol: 0.2,
            min_points: 1,
            noise: 0.0,
            polarity: None,
        }
    }
}

/// One point of the candidate peak.
struct Pt {
    rt: f32,
    mz: f64,
    inten: f64,
}

/// The points of the scans inside the RT range: best peak of each scan inside `[mz_lo, mz_hi]`, or a zero.
fn points<R: ReadAt>(
    ix: &mut Index<R>,
    si: usize,
    mz: (f64, f64),
    rt: (f32, f32),
    noise: f64,
) -> Result<Vec<Pt>> {
    let rts = &ix.arrays(si)?.rt;
    let i0 = rts.partition_point(|&r| (r as f32) < rt.0);
    let i1 = rts.partition_point(|&r| (r as f32) <= rt.1);
    if i0 >= i1 {
        return Ok(Vec::new());
    }
    let mut pts: Vec<Pt> = rts[i0..i1]
        .iter()
        .map(|&r| Pt {
            rt: r as f32,
            mz: (mz.0 + mz.1) / 2.0,
            inten: 0.0,
        })
        .collect();
    let (a, b) = (ix.search(si, mz.0, false)?, ix.search(si, mz.1, true)?);
    ix.each_peak(si, a, b, |pos, m, it| {
        if pos >= i0 && pos < i1 && it >= noise && it > pts[pos - i0].inten {
            pts[pos - i0].mz = m;
            pts[pos - i0].inten = it;
        }
    })?;
    Ok(pts)
}

/// Steps 2 and 3: the slice of points `[start, end)` of the peak, or `None` when there is no apex.
fn peak_span(pts: &[Pt], tol: f64) -> Option<(usize, usize)> {
    let n = pts.len();
    let mut apex: Option<usize> = None;
    let mut top = 0.0f64;
    for i in 1..n.saturating_sub(1) {
        let v = pts[i].inten;
        if v >= pts[i + 1].inten && v >= pts[i - 1].inten && v > top {
            top = v;
            apex = Some(i);
        }
    }
    let apex = apex?;
    let mut start = apex;
    let mut cur = pts[start].inten;
    while start > 0 {
        let next = pts[start - 1].inten;
        if cur < next * (1.0 - tol) {
            break;
        }
        start -= 1;
        if next == 0.0 {
            break;
        }
        cur = next;
    }
    let mut stop = apex;
    let mut end = apex;
    cur = pts[stop].inten;
    while stop < n - 1 {
        let next = pts[stop + 1].inten;
        if next > cur * (1.0 + tol) {
            end = n.min(stop + 1);
            break;
        }
        stop += 1;
        end = n.min(stop + 1);
        if next == 0.0 {
            end = stop + 1;
            break;
        }
        cur = next;
    }
    Some((start, end))
}

/// Step 5: the feature of the points `[start, end)`.
fn feature_of(p: &[Pt]) -> Feature {
    let (mut best, mut sw, mut swm) = (0usize, 0.0f64, 0.0f64);
    for (i, x) in p.iter().enumerate() {
        if x.inten > p[best].inten {
            best = i;
        }
        sw += x.inten;
        swm += x.inten * x.mz;
    }
    let mut area = 0.0f32;
    for w in p.windows(2) {
        let sum = (w[1].inten + w[0].inten) as f32;
        area = (area as f64 + ((w[1].rt - w[0].rt) * sum) as f64 / 2.0) as f32;
    }
    Feature {
        mz: if sw > 0.0 { swm / sw } else { p[best].mz },
        rt: p[best].rt as f64,
        rt_start: p[0].rt as f64,
        rt_end: p[p.len() - 1].rt as f64,
        height: p[best].inten,
        area: area as f64,
        filled: true,
    }
}

/// Fills the gaps `(row m/z, row RT)` of one file; `None` = nothing accepted for that gap (or the file has no scans of the polarity).
pub fn fill_file<R: ReadAt>(
    ix: &mut Index<R>,
    gaps: &[(f64, f64)],
    p: &GapParams,
) -> Result<Vec<Option<Feature>>> {
    let Some(si) = ix.section_of(1, p.polarity) else {
        return Ok(vec![None; gaps.len()]);
    };
    let mut out = Vec::with_capacity(gaps.len());
    for &(mz, rt) in gaps {
        let tol = p.tol_da.max(mz / 1e6 * p.tol_ppm);
        let pts = points(
            ix,
            si,
            (mz - tol, mz + tol),
            (rt as f32 - p.rt_tol as f32, rt as f32 + p.rt_tol as f32),
            p.noise,
        )?;
        let found = peak_span(&pts, p.intensity_tol).filter(|&(a, b)| b - a >= p.min_points);
        out.push(found.map(|(a, b)| feature_of(&pts[a..b])));
    }
    Ok(out)
}

/// Fills every empty cell of `rows` from the index of its file (`ixs[k]` = file `k`) and refreshes the row means. Returns the number of cells filled.
pub fn fill<R: ReadAt>(rows: &mut Rows, ixs: &mut [Index<R>], p: &GapParams) -> Result<usize> {
    let mut filled = 0;
    let mut touched = vec![false; rows.rows.len()];
    for (file, ix) in ixs.iter_mut().enumerate().take(rows.n_files) {
        let todo: Vec<usize> = (0..rows.rows.len())
            .filter(|&r| rows.rows[r].cells[file].is_none())
            .collect();
        let gaps: Vec<(f64, f64)> = todo
            .iter()
            .map(|&r| (rows.rows[r].mz, rows.rows[r].rt))
            .collect();
        for (&r, f) in todo.iter().zip(fill_file(ix, &gaps, p)?) {
            if f.is_some() {
                rows.rows[r].cells[file] = f;
                touched[r] = true;
                filled += 1;
            }
        }
    }
    for (r, _) in touched.iter().enumerate().filter(|(_, t)| **t) {
        rows.rows[r].refresh();
    }
    Ok(filled)
}
