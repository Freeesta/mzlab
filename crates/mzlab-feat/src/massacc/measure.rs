//! The trace of each reference ion in the MS1 scans of one index: per scan, the most intense peak inside the search window (its m/z, its intensity) and the summed intensity of the
//! window. Only the peak table of the index is read; nothing is written. Used by `massacc::run`.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Value;

use super::refs::RefIon;
use super::Params;

/// A reference needs at least this many scans over `min_intensity` to have a trace.
pub const MIN_SCANS: usize = 3;
/// A listed background ion must be seen in at least this share of the MS1 scans of the file: background is there all along, an analyte is not.
pub const BACKGROUND_SHARE: f64 = 0.5;

/// The scans of one reference, in scan order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Trace {
    /// Position of the reference in the list that was searched.
    pub r: usize,
    pub rt: Vec<f64>,
    /// m/z of the most intense peak of each scan.
    pub mz: Vec<f64>,
    pub inten: Vec<f64>,
    /// Summed intensity of the peaks in the window.
    pub sum: Vec<f64>,
}

/// What one index gave.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileMeasure {
    /// Polarity of the main MS1 group: 1, -1, or 0 when the index does not split it.
    pub pol: i8,
    pub n_scans: usize,
    /// SHA-256 of the file the index was made from (empty when it does not say).
    pub sha256: String,
    /// References that could be looked for in this file (their polarity is in it).
    pub searched: usize,
    pub traces: Vec<Trace>,
    /// What the method of the file says of its lock mass, when the index carries it.
    pub lock_mass: Option<Value>,
}

/// The MS1 group of a polarity: its position and its scans.
fn group<R: ReadAt>(index: &Index<R>, pol: Option<i8>) -> Option<(usize, usize)> {
    let si = index.section_of(1, pol)?;
    Some((si, index.sections[si].n_scans))
}

/// The lock-mass description of the method (`ms.calibrazione.lock_mass`) when the index holds the method; it is read, never applied.
fn lock_mass<R: ReadAt>(index: &Index<R>) -> Option<Value> {
    [&index.run, &index.source].into_iter().find_map(|v| {
        v.get("metodo")?
            .get("calibrazione")?
            .get("lock_mass")
            .filter(|l| !l.is_null())
            .cloned()
    })
}

/// The traces of `refs` in `index`.
pub fn measure<R: ReadAt>(
    index: &mut Index<R>,
    refs: &[RefIon],
    p: &Params,
) -> Result<FileMeasure> {
    let (pos, neg, all) = (
        group(index, Some(1)),
        group(index, Some(-1)),
        group(index, None),
    );
    let main = match (pos, neg) {
        (Some(a), Some(b)) => Some(if b.1 > a.1 { (b, -1) } else { (a, 1) }),
        (Some(a), None) => Some((a, 1)),
        (None, Some(b)) => Some((b, -1)),
        (None, None) => all.map(|a| (a, 0)),
    };
    let mut out = FileMeasure {
        sha256: crate::series::source_sha256(index),
        lock_mass: lock_mass(index),
        ..FileMeasure::default()
    };
    let Some(((_, n_main), pol)) = main else {
        return Ok(out);
    };
    (out.pol, out.n_scans) = (pol, n_main);
    let mut rts: [Option<Vec<f64>>; 3] = [None, None, None];
    for (r, reference) in refs.iter().enumerate() {
        let (slot, found) = match reference.pol {
            1 => (0, pos),
            -1 => (1, neg),
            _ => (2, main.map(|(g, _)| g)),
        };
        let Some((si, n_scans)) = found else {
            continue;
        };
        out.searched += 1;
        let (lo, hi) = (
            reference.mz * (1.0 - p.search_ppm * 1e-6),
            reference.mz * (1.0 + p.search_ppm * 1e-6),
        );
        let (a, b) = (index.search(si, lo, false)?, index.search(si, hi, true)?);
        if b <= a {
            continue;
        }
        let mut peaks: Vec<(u32, f64, f64)> = Vec::new();
        index.each_peak(si, a, b, |scan, mz, y| {
            if y >= p.min_intensity {
                peaks.push((scan as u32, mz, y));
            }
        })?;
        if peaks.len() < MIN_SCANS {
            continue;
        }
        if rts[slot].is_none() {
            rts[slot] = Some(index.arrays(si)?.rt.clone());
        }
        let rt = rts[slot].as_deref().unwrap_or_default();
        // by scan, the most intense peak first (the lower m/z on a tie)
        crate::sortc::by(&mut peaks, |x, y| {
            x.0.cmp(&y.0)
                .then(y.2.total_cmp(&x.2))
                .then(x.1.total_cmp(&y.1))
        });
        let mut t = Trace {
            r,
            ..Trace::default()
        };
        let mut i = 0;
        while i < peaks.len() {
            let j = peaks[i..]
                .iter()
                .position(|q| q.0 != peaks[i].0)
                .map_or(peaks.len(), |k| i + k);
            let scan_rt = rt.get(peaks[i].0 as usize).copied().unwrap_or(f64::NAN);
            let inside = reference
                .window
                .is_none_or(|(w0, w1)| scan_rt >= w0 && scan_rt <= w1);
            if inside && scan_rt.is_finite() {
                t.rt.push(scan_rt);
                t.mz.push(peaks[i].1);
                t.inten.push(peaks[i].2);
                t.sum.push(peaks[i..j].iter().map(|q| q.2).sum());
            }
            i = j;
        }
        let needed = if reference.background {
            ((BACKGROUND_SHARE * n_scans as f64).ceil() as usize).max(MIN_SCANS)
        } else {
            MIN_SCANS
        };
        if t.rt.len() >= needed {
            out.traces.push(t);
        }
    }
    Ok(out)
}
