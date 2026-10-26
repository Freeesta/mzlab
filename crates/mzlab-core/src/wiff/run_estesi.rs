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

    /// True when experiment `e` is a full scan: its spectra give a TIC chromatogram (the other scan experiments give none).
    fn is_full_scan(&self, e: usize) -> bool {
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
                    blocks.add_tic(r);
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
                    blocks.add_tic(r);
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
        chroms
    }
}

/// The MRM blocks of one experiment: the times (min) and TIC of every block, and per transition the times and cps of the scans that acquired it.
#[derive(Default)]
struct Experiment {
    time: Vec<f64>,
    tic: Vec<f64>,
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
    fn add_tic(&mut self, r: &Record) {
        if let Some(e) = self.experiments.get_mut(r.experiment) {
            e.time.push(r.time_min);
            e.tic.push(r.tic);
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
