//! The spectra and chromatograms of a sample (a TIC for each full scan or MRM experiment) for the readers module (feature `estesi`): as `run.rs` (SIM experiments as in `run.rs`), and the MRM blocks may be scheduled (sMRM: a transition
//! is a point of its chromatogram only in the scans where it was acquired) and the experiments of a sample may be several (one TIC each).

use super::super::cromatogrammi::Blocks;
use super::super::{scan, Record, Wiff};
use crate::error::Result;
use crate::idxread::ReadAt;
use crate::mzml::{Chromatogram, Spectrum};

impl Wiff {
    /// True when block `r` is one of an experiment that has no scans or transitions to give (a scan experiment's row without data).
    fn is_empty_row(&self, r: &Record) -> bool {
        r.size == 0
            && self
                .experiments
                .get(r.experiment)
                .and_then(|e| e.code)
                .is_some_and(|c| c != super::super::modi::CODE_MRM)
    }

    /// Gives the spectra one at a time to `on` (profile; the empty scans are left out) and returns the chromatograms of the MRM blocks.
    pub fn visit<R: ReadAt>(
        &self,
        scan: &mut R,
        on: &mut dyn FnMut(Spectrum) -> Result<()>,
    ) -> Result<Vec<Chromatogram>> {
        let mut blocks = Blocks::new(self.experiments.len().max(1));
        let mut n = 0;
        for r in self.records.iter().filter(|r| !r.cut(self.scan_len)) {
            if let Some(p) = scan::read_scan(scan, self.scan_len, r.at, r.end)? {
                if p.mz.is_empty() {
                    continue;
                }
                if self.is_full_scan(r.experiment) {
                    blocks.add_tic(r, p.intensity.iter().copied().reduce(f64::max));
                }
                on(self.spectrum_of(n, r, p))?;
                n += 1;
            } else if !self.is_empty_row(r)
                && !self.scan_experiment(r.experiment)
                && let Some(c) = scan::read_mrm_cols(scan, self.scan_len, r.at, r.size as usize)?
            {
                blocks.add(r, c);
            }
        }
        let mut chroms = self.chromatograms_of(blocks);
        if let Some(dad) = &self.dad {
            chroms.push(dad.twc_chromatogram(chroms.len()));
        }
        Ok(chroms)
    }

    /// The chromatograms of the MRM blocks only (no scan is decoded). `counts`: the point count of each record's head when the reader already read them.
    pub fn chromatograms<R: ReadAt>(
        &self,
        scan: &mut R,
        counts: Option<&[Option<u32>]>,
    ) -> Result<Vec<Chromatogram>> {
        let mut blocks = Blocks::new(self.experiments.len().max(1));
        for (i, r) in self
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.cut(self.scan_len))
        {
            let count = match counts.and_then(|c| c.get(i)) {
                Some(&c) => c,
                None => scan::point_count(scan, self.scan_len, r.at, r.end)?,
            };
            if let Some(count) = count {
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
}

impl Blocks {
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
