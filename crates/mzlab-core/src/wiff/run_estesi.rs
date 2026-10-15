//! The spectra and chromatograms of a sample for the readers module (feature `estesi`): as `run.rs`, and the MRM blocks may be scheduled (sMRM: a transition
//! is a point of its chromatogram only in the scans where it was acquired) and the experiments of a sample may be several (the TIC takes the blocks of all).

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
                on(self.spectrum_of(n, r, p))?;
                n += 1;
            } else if !self.is_empty_row(r) {
                if let Some(c) = scan::read_mrm_cols(scan, self.scan_len, r.at, r.size as usize)? {
                    blocks.add(r, c);
                }
            }
        }
        Ok(self.chromatograms_of(blocks))
    }

    /// The chromatograms of the MRM blocks only (no scan is decoded).
    pub fn chromatograms<R: ReadAt>(&self, scan: &mut R) -> Result<Vec<Chromatogram>> {
        let mut blocks = Blocks::new(self.experiments.len().max(1));
        for r in &self.records {
            if scan::point_count(scan, self.scan_len, r.at, r.end)?.is_some()
                || self.is_empty_row(r)
            {
                continue;
            }
            if let Some(c) = scan::read_mrm_cols(scan, self.scan_len, r.at, r.size as usize)? {
                blocks.add(r, c);
            }
        }
        Ok(self.chromatograms_of(blocks))
    }

    /// The TIC of all the MRM blocks and one SRM chromatogram per transition that was acquired at least once.
    fn chromatograms_of(&self, blocks: Blocks) -> Vec<Chromatogram> {
        let unit = "number of detector counts";
        let mut chroms = Vec::new();
        if blocks.time.is_empty() {
            return chroms;
        }
        chroms.push(Chromatogram {
            index: 0,
            id: "TIC".into(),
            kind: ChromKind::Tic,
            q1: None,
            q3: None,
            unit: unit.into(),
            time: blocks.time,
            intensity: blocks.tic,
        });
        for (e, ex) in blocks.experiments.into_iter().enumerate() {
            let known = self.mrm_transitions(e);
            for (j, (time, intensity)) in ex
                .cols
                .into_iter()
                .enumerate()
                .filter(|(_, c)| !c.0.is_empty())
            {
                let (q1, q3) = known.get(j).copied().unwrap_or((None, None));
                let mass = match (q1, q3) {
                    (Some(a), Some(b)) => format!(" Q1={a} Q3={b}"),
                    _ => String::new(),
                };
                let id = format!(
                    "SRM SIC{mass} sample={} period=1 experiment={} transition={j}",
                    self.sample,
                    e + 1
                );
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

/// The MRM blocks of one experiment: per transition, the times (min) and cps of the scans that acquired it.
#[derive(Default)]
struct Experiment {
    cols: Vec<(Vec<f64>, Vec<f64>)>,
}

/// The blocks of all the MRM experiments: the times and TIC of every block, and the experiments.
struct Blocks {
    time: Vec<f64>,
    tic: Vec<f64>,
    experiments: Vec<Experiment>,
}

impl Blocks {
    fn new(experiments: usize) -> Self {
        Blocks {
            time: Vec::new(),
            tic: Vec::new(),
            experiments: (0..experiments).map(|_| Experiment::default()).collect(),
        }
    }

    fn add(&mut self, r: &Record, c: scan::Cols) {
        self.time.push(r.time_min);
        self.tic.push(r.tic);
        let Some(e) = self.experiments.get_mut(r.experiment) else {
            return;
        };
        if e.cols.len() < c.width {
            e.cols.resize_with(c.width, Default::default);
        }
        for (j, x) in c.points {
            e.cols[j].0.push(r.time_min);
            e.cols[j].1.push(x);
        }
    }
}
