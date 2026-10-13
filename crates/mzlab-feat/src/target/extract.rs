//! Targeted extraction in one file: the trace of one ion in a window of RT and the peaks of it that pass the low thresholds.
//! The trace is the sum of the peaks within `ppm` in every scan (two binary searches on the peak table and one slice of it); a peak is accepted
//! when its S/N, shape and number of points reach the limits of `TargetParams` (the S/N takes the place of the minimum height of the detection).

use crate::axis::half;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::peakq::{detect_prominence, ProminenceParams};

use super::TargetParams;
use crate::quality::measure;
use crate::table::{FileFeatures, FLAG_FT_LOBE, FLAG_GOOD};

/// A peak accepted in the trace of an ion; positions are scans of the MS1 section.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cand {
    pub apex: u32,
    pub lo: u32,
    pub hi: u32,
    pub rt: f32,
    pub left: f32,
    pub right: f32,
    pub area: f64,
    pub height: f32,
    pub snr: f32,
    pub shape: f32,
    pub fwhm: f32,
    pub asym: f32,
    pub gaps: u16,
    pub npts: u16,
}

/// The MS1 section of a file: its index and the RT of every scan.
pub(crate) struct Section {
    pub si: usize,
    pub rt: Vec<f64>,
}

impl Section {
    pub fn open<R: ReadAt>(index: &mut Index<R>, polarity: i8) -> Result<Option<Section>> {
        let Some(si) = index.section_of(1, Some(polarity)) else {
            return Ok(None);
        };
        let rt = index.arrays(si)?.rt.clone();
        Ok((!rt.is_empty()).then_some(Section { si, rt }))
    }

    pub fn run(&self) -> (f64, f64) {
        (self.rt[0], self.rt[self.rt.len() - 1])
    }
}

/// The peaks of the ion `mz` whose apex lies in the RT window `win`, highest first.
pub(crate) fn peaks<R: ReadAt>(
    index: &mut Index<R>,
    sec: &Section,
    mz: f64,
    win: (f64, f64),
    p: &TargetParams,
) -> Result<Vec<Cand>> {
    let d = half(mz, p.ppm);
    let a = index.search(sec.si, mz - d, false)?;
    let b = index.search(sec.si, mz + d, true)?;
    if b <= a {
        return Ok(Vec::new());
    }
    let y = index.sum_window(sec.si, a, b)?;
    Ok(trace_peaks(&sec.rt, &y, win, p))
}

fn trace_peaks(rt: &[f64], y: &[f64], win: (f64, f64), p: &TargetParams) -> Vec<Cand> {
    let n = rt.len();
    let i0 = rt.partition_point(|&t| t < win.0);
    let i1 = rt.partition_point(|&t| t <= win.1);
    if i1 <= i0 || y[i0..i1].iter().fold(0.0f64, |m, &v| m.max(v)) < p.floor {
        return Vec::new();
    }
    let (w0, w1) = (i0.saturating_sub(p.pad), (i1 + p.pad).min(n));
    let (t, v) = (&rt[w0..w1], &y[w0..w1]);
    let pp = ProminenceParams {
        snr: p.min_snr,
        min_scans: p.min_points as usize,
        ..Default::default()
    };
    let mut out = Vec::new();
    for pk in detect_prominence(t, v, pp) {
        let apex = w0 + pk.apex_i;
        if apex < i0 || apex >= i1 {
            continue;
        }
        let npts = v[pk.lo..=pk.hi]
            .iter()
            .filter(|&&x| x > 0.0)
            .count()
            .min(u16::MAX as usize) as u16;
        let (snr, shape, fwhm, asym, gaps) = measure(t, v, pk.lo, pk.hi, p.floor);
        if snr < p.min_snr || shape < p.min_shape || npts < p.min_points {
            continue;
        }
        out.push(Cand {
            apex: apex as u32,
            lo: (w0 + pk.lo) as u32,
            hi: (w0 + pk.hi) as u32,
            rt: rt[apex] as f32,
            left: t[pk.lo] as f32,
            right: t[pk.hi] as f32,
            area: pk.area * 60.0,
            height: y[apex] as f32,
            snr: snr as f32,
            shape: shape as f32,
            fwhm: fwhm as f32,
            asym: asym as f32,
            gaps,
            npts,
        });
    }
    out
}

/// A row of the detection table (sorted by m/z) that is the same ion as `mz` at `rt`: within `ppm`, and the apex of one inside the edges of the
/// other or within `rt_tol` minutes. The lowest m/z error wins.
pub(crate) fn detected_row(f: &FileFeatures, mz: f64, c: &Cand, p: &TargetParams) -> Option<usize> {
    let d = half(mz, p.ppm);
    let a = f.mz.partition_point(|&x| x < mz - d);
    let b = f.mz.partition_point(|&x| x <= mz + d);
    (a..b)
        .filter(|&i| f.flags[i] & FLAG_FT_LOBE == 0)
        .filter(|&i| {
            let (apex, rt) = (c.rt, f.rt[i]);
            (apex - rt).abs() as f64 <= p.rt_tol
                || (apex >= f.left[i] && apex <= f.right[i])
                || (rt >= c.left && rt <= c.right)
        })
        .min_by(|&i, &j| (f.mz[i] - mz).abs().total_cmp(&(f.mz[j] - mz).abs()))
}

/// Appends the peak `c` of the ion `mz` as a row of the table of targeted peaks of the file.
pub(crate) fn push_peak(t: &mut FileFeatures, mz: f64, c: &Cand) {
    t.mz.push(mz);
    t.rt.push(c.rt);
    t.left.push(c.left);
    t.right.push(c.right);
    t.apex.push(c.apex);
    t.scan_lo.push(c.lo);
    t.scan_hi.push(c.hi);
    t.area.push(c.area);
    t.height.push(c.height);
    t.npts.push(c.npts);
    t.flags.push(FLAG_GOOD);
    t.iso_group.push(0);
    t.charge.push(0);
    t.shape.push(c.shape);
    t.snr.push(c.snr);
    t.fwhm.push(c.fwhm);
    t.asym.push(c.asym);
    t.gaps.push(c.gaps);
    t.qreason.push(0);
    t.ms2_n.push(0);
    t.purity.push(-1.0);
    t.area_win.push(c.area);
    t.family_id.push(0);
    t.role.push(0);
    t.alt.push(0);
}
