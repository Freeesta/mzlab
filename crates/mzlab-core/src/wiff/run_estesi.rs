//! The spectra and chromatograms of a sample (a TIC for each full scan or MRM experiment) for the readers module (feature `estesi`): as `run.rs`, and the MRM blocks may be scheduled (sMRM: a transition
//! is a point of its chromatogram only in the scans where it was acquired) and the experiments of a sample may be several (one TIC each).

use super::super::{scan, Record, Wiff};
use crate::error::Result;
use crate::idxread::ReadAt;
use crate::mzml::{ChromKind, Chromatogram, Spectrum};

impl Wiff {
    /// True when block `r` is one of an experiment that has no scans or transitions to give (a scan experiment's row without data).
    fn is_empty_row(&self, r: &Record) -> bool {
        r.size == 0
            && self
                .experiments
                .get(r.experiment)
                .and_then(|e| e.code)
                .is_some_and(|c| c != super::super::estesi::CODE_MRM)
    }

    /// Appends the BPC of each `(experiment, times, intensities)` after the transitions, so that the list a reader that decodes later (`LettoreWiff`) completes is in
    /// the same order.
    fn push_bpc(
        &self,
        chroms: &mut Vec<Chromatogram>,
        bpcs: Vec<(usize, Vec<f64>, Vec<f64>)>,
        many: bool,
    ) {
        for (e, time, intensity) in bpcs {
            let unit = "number of detector counts".into();
            chroms.push(Chromatogram {
                index: chroms.len(),
                id: self.bpc_id(e, many),
                kind: ChromKind::Bpc,
                q1: None,
                q3: None,
                unit,
                time,
                intensity,
            });
        }
    }

    /// The id of the BPC of experiment `e` (0-based): `BPC`, or with the sample and experiment when several experiments have a TIC.
    pub(crate) fn bpc_id(&self, e: usize, many: bool) -> String {
        match many {
            true => format!("BPC sample={} period=1 experiment={}", self.sample, e + 1),
            false => "BPC".into(),
        }
    }

    /// True when experiment `e` is a full scan: its spectra give a TIC chromatogram (the other scan experiments give none).
    pub(crate) fn is_full_scan(&self, e: usize) -> bool {
        self.experiments.get(e).and_then(|x| x.code) == Some(super::super::estesi::CODE_FULL)
    }

    /// Gives the spectra one at a time to `on` (profile; the empty scans are left out) and returns the chromatograms of the MRM blocks.
    pub fn visit<R: ReadAt>(
        &self,
        scan: &mut R,
        on: &mut dyn FnMut(Spectrum) -> Result<()>,
    ) -> Result<Vec<Chromatogram>> {
        let mut blocks = Blocks::new(self.experiments.len().max(1));
        let mut n = 0;
        for r in &self.records {
            if let Some(p) = scan::read_scan(scan, self.scan_len, r.at, r.end)? {
                if p.mz.is_empty() {
                    continue;
                }
                if self.is_full_scan(r.experiment) {
                    blocks.add_tic(r, p.intensity.iter().copied().reduce(f64::max));
                }
                on(self.spectrum_of(n, r, p))?;
                n += 1;
            } else if !self.is_empty_row(r) && !self.scan_experiment(r.experiment) {
                if let Some(c) = scan::read_mrm_cols(scan, self.scan_len, r.at, r.size as usize)? {
                    blocks.add(r, c);
                }
            }
        }
        let mut chroms = self.chromatograms_of(blocks);
        if let Some(dad) = &self.dad {
            chroms.push(dad.twc_chromatogram(chroms.len()));
        }
        Ok(chroms)
    }

    /// The chromatograms of the MRM blocks only (no scan is decoded).
    pub fn chromatograms<R: ReadAt>(&self, scan: &mut R) -> Result<Vec<Chromatogram>> {
        let mut blocks = Blocks::new(self.experiments.len().max(1));
        for r in &self.records {
            if let Some(count) = scan::point_count(scan, self.scan_len, r.at, r.end)? {
                if count > 0 && self.is_full_scan(r.experiment) {
                    blocks.add_tic(r, None);
                }
                continue;
            }
            if self.is_empty_row(r) || self.scan_experiment(r.experiment) {
                continue;
            }
            if let Some(c) = scan::read_mrm_cols(scan, self.scan_len, r.at, r.size as usize)? {
                blocks.add(r, c);
            }
        }
        let mut chroms = self.chromatograms_of(blocks);
        if let Some(dad) = &self.dad {
            chroms.push(dad.twc_chromatogram(chroms.len()));
        }
        Ok(chroms)
    }

    /// One TIC per MRM experiment that has blocks (just `TIC` when there is one) and one SRM chromatogram per transition that was acquired at least once.
    /// With `trim_scheduled`, a scheduled transition keeps only the points within its window around the target RT, as msconvert does.
    fn chromatograms_of(&self, blocks: Blocks) -> Vec<Chromatogram> {
        let unit = "number of detector counts";
        let mut chroms = Vec::new();
        let mut bpcs = Vec::new();
        let many = blocks
            .experiments
            .iter()
            .filter(|x| !x.time.is_empty())
            .count()
            > 1;
        for (e, ex) in blocks.experiments.into_iter().enumerate() {
            if ex.time.is_empty() {
                continue;
            }
            let id = match many {
                true => format!("TIC sample={} period=1 experiment={}", self.sample, e + 1),
                false => "TIC".into(),
            };
            if !ex.bpc.is_empty() && ex.bpc.len() == ex.time.len() {
                bpcs.push((e, ex.time.clone(), ex.bpc));
            }
            chroms.push(Chromatogram {
                index: chroms.len(),
                id,
                kind: ChromKind::Tic,
                q1: None,
                q3: None,
                unit: unit.into(),
                time: ex.time,
                intensity: ex.tic,
            });
            let known = self.mrm_transitions(e);
            let window = self.experiments.get(e).and_then(|x| x.window_s);
            for (j, (mut time, mut intensity)) in ex
                .cols
                .into_iter()
                .enumerate()
                .filter(|(_, c)| !c.0.is_empty())
            {
                let t = known.get(j);
                if let (true, Some(t), Some(w)) = (self.trim_scheduled, t, window) {
                    let half = f64::from(w) / 120.0;
                    let keep: Vec<bool> =
                        time.iter().map(|x| (x - t.dwell).abs() <= half).collect();
                    let mut k = keep.iter();
                    time.retain(|_| *k.next().unwrap_or(&true));
                    let mut k = keep.iter();
                    intensity.retain(|_| *k.next().unwrap_or(&true));
                    if time.is_empty() {
                        continue;
                    }
                }
                let (q1, q3) = (t.map(|t| t.q1), t.map(|t| t.q3));
                let id = self.srm_id(e, j, t);
                chroms.push(Chromatogram {
                    index: chroms.len(),
                    id,
                    kind: ChromKind::Srm,
                    q1,
                    q3,
                    unit: unit.into(),
                    time,
                    intensity,
                });
            }
        }
        self.push_bpc(&mut chroms, bpcs, many);
        chroms
    }
}

/// The MRM blocks of one experiment: the times (min) and TIC of every block, and per transition the times and cps of the scans that acquired it.
#[derive(Default)]
struct Experiment {
    time: Vec<f64>,
    tic: Vec<f64>,
    /// Base peak intensity of each block of `time`, empty when the blocks were not decoded.
    bpc: Vec<f64>,
    cols: Vec<(Vec<f64>, Vec<f64>)>,
}

/// The blocks of all the MRM experiments.
struct Blocks {
    experiments: Vec<Experiment>,
}

impl Blocks {
    fn new(experiments: usize) -> Self {
        Blocks {
            experiments: (0..experiments).map(|_| Experiment::default()).collect(),
        }
    }

    /// A point of the TIC of a full scan experiment: the TIC the index holds for the block.
    /// `base` is the base peak intensity of the block when its points were decoded.
    fn add_tic(&mut self, r: &Record, base: Option<f64>) {
        if let Some(e) = self.experiments.get_mut(r.experiment) {
            e.time.push(r.time_min);
            e.tic.push(r.tic);
            e.bpc.extend(base);
        }
    }

    fn add(&mut self, r: &Record, c: scan::Cols) {
        let Some(e) = self.experiments.get_mut(r.experiment) else {
            return;
        };
        e.time.push(r.time_min);
        e.tic.push(r.tic);
        if e.cols.len() < c.width {
            e.cols.resize_with(c.width, Default::default);
        }
        for (j, x) in c.points {
            if let Some(col) = e.cols.get_mut(j) {
                col.0.push(r.time_min);
                col.1.push(x);
            }
        }
    }
}
