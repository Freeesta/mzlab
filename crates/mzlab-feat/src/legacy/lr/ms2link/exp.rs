//! The experiments of the series: the level-2 scans of the MS2 files and of the full-scan files, grouped by polarity, precursor and collision
//! energy, with their trace `(rt, TIC)` taken from the index (no peak is read) and the peaks of it.

use mzlab_core::idxread::{Index, ReadAt};

use super::{runs, small, FEW_OUTSIDE, SN_PEAK};
use crate::legacy::lr::LrDig;
use crate::legacy::Res;
use mzlab_core::peaks::{detect_peaks, Params};

/// A peak of the trace of an experiment (minutes).
#[derive(Debug, Clone, Copy, Default)]
pub struct PeakL {
    pub apex: f64,
    pub lo: f64,
    pub hi: f64,
    pub sn: f64,
    pub scans: usize,
}

/// One product-ion experiment of one file.
#[derive(Debug, Clone, Default)]
pub struct Exp {
    pub file: String,
    /// Sampling time of the file, minutes.
    pub time: Option<f64>,
    pub polarity: i8,
    pub precursor: f64,
    pub ce: Option<f64>,
    /// Index of the file in the order of `open` (the columns, then the MS2 files).
    pub k: usize,
    /// Rows of the index of the file, by time.
    pub scans: Vec<usize>,
    pub rt: Vec<f64>,
    pub tic: Vec<f64>,
    pub drift: f64,
    pub peaks: Vec<PeakL>,
    /// The trace has peaks, all of them before `rt_min`.
    pub front: bool,
    /// Half the isolation window written in the file (median of the scans), Da; `None` when the file does not say it.
    pub half: Option<f64>,
}

type Key = (i8, i64, i64);

fn tenth(v: f64) -> i64 {
    (v * 10.0).round() as i64
}

/// The experiments of every file and the polarity of the series (the majority of its level-1 scans; 0 when none says).
pub fn experiments<R: ReadAt>(
    d: &LrDig,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<(Vec<Exp>, i8)> {
    let ms2: Vec<_> = d.files.iter().filter(|f| f.kind == "ms2").collect();
    let (mut pos, mut neg) = (0usize, 0usize);
    let mut out = Vec::new();
    for k in 0..d.lc.len() + ms2.len() {
        let ix = open(k)?;
        let (name, time) = match d.lc.get(k) {
            Some(f) => (f.name.clone(), f.time),
            None => (ms2[k - d.lc.len()].name.clone(), ms2[k - d.lc.len()].time),
        };
        let mut tagged: Vec<(Key, usize)> = Vec::new();
        for (i, s) in ix.scans.iter().enumerate() {
            if s.level == 1 && k < d.lc.len() {
                pos += usize::from(s.polarity > 0);
                neg += usize::from(s.polarity < 0);
            }
            if let (2, Some(p)) = (s.level, s.precursor) {
                let ce = s
                    .collision_energy
                    .filter(|c| *c != 0.0)
                    .map_or(i64::MIN, tenth);
                tagged.push(((s.polarity, tenth(p), ce), i));
            }
        }
        mzlab_core::sortc::by(&mut tagged, |a, b| a.cmp(b));
        for ((polarity, _, ce), mut scans) in runs(&tagged) {
            small(&mut scans, |a, b| {
                ix.scans[*a].rt.total_cmp(&ix.scans[*b].rt)
            });
            let mut precs: Vec<f64> = scans
                .iter()
                .filter_map(|i| ix.scans[*i].precursor)
                .collect();
            small(&mut precs, |a, b| a.total_cmp(b));
            let mut halves: Vec<f64> = scans
                .iter()
                .filter_map(|i| ix.scans[*i].iso)
                .map(|(a, b)| (b - a) / 2.0)
                .filter(|h| *h > 0.0)
                .collect();
            small(&mut halves, |a, b| a.total_cmp(b));
            let mut e = Exp {
                half: halves.get(halves.len() / 2).copied(),
                file: name.clone(),
                time,
                polarity,
                precursor: mzlab_core::json::round(precs[precs.len() / 2], 2),
                ce: (ce != i64::MIN).then(|| ce as f64 / 10.0),
                k,
                rt: scans.iter().map(|i| ix.scans[*i].rt).collect(),
                tic: scans.iter().map(|i| ix.scans[*i].tic).collect(),
                scans,
                ..Exp::default()
            };
            list_peaks(&mut e, d.settings.rt_min, d.settings.rt_tol_min);
            out.push(e);
        }
    }
    let series = if pos > neg {
        1
    } else if neg > pos {
        -1
    } else {
        0
    };
    Ok((out, series))
}

/// The peaks of the trace: the ones above S/N `SN_PEAK` after `rt_min`; a trace acquired only around one time is one peak, all of it.
fn list_peaks(e: &mut Exp, rt_min: f64, rt_tol: f64) {
    let n = e.rt.len();
    if n == 0 {
        return;
    }
    let mid = e.rt[n / 2];
    if e.rt.iter().filter(|t| (**t - mid).abs() > rt_tol).count() < FEW_OUTSIDE {
        let top = (0..n).fold(0, |b, i| if e.tic[i] > e.tic[b] { i } else { b });
        e.peaks.push(PeakL {
            apex: e.rt[top],
            lo: e.rt[0],
            hi: e.rt[n - 1],
            sn: 0.0,
            scans: n,
        });
        return;
    }
    if n < 10 {
        return;
    }
    let par = Params {
        snr: SN_PEAK,
        ..Params::default()
    };
    let found = detect_peaks(&e.rt, &e.tic, par);
    e.front = found.iter().any(|p| e.rt[p.apex_i] < rt_min)
        && found.iter().all(|p| e.rt[p.apex_i] < rt_min);
    for p in found.iter().filter(|p| e.rt[p.apex_i] >= rt_min) {
        e.peaks.push(PeakL {
            apex: e.rt[p.apex_i],
            lo: e.rt[p.lo],
            hi: e.rt[p.hi],
            sn: p.snr,
            scans: p.n_scans,
        });
    }
    small(&mut e.peaks, |a, b| a.apex.total_cmp(&b.apex));
}

impl Exp {
    /// The m/z tolerance for the precursor of this experiment (`ms2assoc::tolerance` with the Q1 half-window of a unit-resolution instrument).
    pub fn tol(&self) -> super::super::ms2assoc::Tol {
        super::super::ms2assoc::tolerance(self.half, super::PREC_TOL, self.precursor)
    }

    /// Positions (in `scans`) of the scans whose time lies in `[lo, hi]`.
    pub fn within(&self, lo: f64, hi: f64) -> Vec<usize> {
        (0..self.rt.len())
            .filter(|&i| self.rt[i] >= lo && self.rt[i] <= hi)
            .map(|i| self.scans[i])
            .collect()
    }
}
