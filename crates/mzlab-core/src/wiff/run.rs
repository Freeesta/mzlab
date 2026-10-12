//! A sample as the `Run` that the rest of the program reads: scans become spectra (profile, empty ones left out as msconvert does)
//! and the MRM blocks become a TIC and one SRM chromatogram per transition.

use super::{scan, Wiff};
use crate::error::Result;
use crate::idxread::ReadAt;
use crate::methodinfo::Kind;
use crate::mzml::{ChromKind, Chromatogram, Run, Spectrum};

/// The MRM blocks of one experiment: times (min), TIC, and one column per transition.
#[derive(Default)]
struct Mrm {
    time: Vec<f64>,
    tic: Vec<f64>,
    cols: Vec<Vec<f64>>,
}

impl Wiff {
    fn mrm_transitions(&self, experiment: usize) -> Vec<(Option<f64>, Option<f64>)> {
        let kind = self.method.as_ref().and_then(|m| m.experiments.get(experiment)).map(|e| &e.kind);
        match kind {
            Some(Kind::Mrm(t)) => t.iter().map(|t| (Some(t.q1), Some(t.q3))).collect(),
            _ => Vec::new(),
        }
    }

    /// Spectra and chromatograms of the sample. Reads every block of the `.wiff.scan` once.
    pub fn run<R: ReadAt>(&self, scan: &mut R) -> Result<Run> {
        let mut run = Run::default();
        let mut mrm: Vec<Mrm> = (0..self.experiments.len().max(1)).map(|_| Mrm::default()).collect();
        for r in &self.records {
            if let Some(p) = scan::read_scan(scan, self.scan_len, r.at, r.end)? {
                if p.mz.is_empty() {
                    continue;
                }
                let e = self.experiments.get(r.experiment).cloned().unwrap_or_default();
                run.spectra.push(Spectrum {
                    index: run.spectra.len(),
                    id: format!("sample={} period=1 cycle={} experiment={}", self.sample, r.cycle + 1, r.experiment + 1),
                    level: e.level.max(1),
                    rt: r.time_min,
                    polarity: self.polarity,
                    profile: true,
                    filter: String::new(),
                    precursor: e.precursor,
                    mz: p.mz,
                    intensity: p.intensity.iter().map(|&v| v as f32).collect(),
                    wide: Some(p.intensity),
                });
            } else if let Some(v) = scan::read_mrm(scan, self.scan_len, r.at, r.size as usize / 4)? {
                let m = &mut mrm[r.experiment];
                m.time.push(r.time_min);
                m.tic.push(r.tic);
                m.cols.resize(v.len(), Vec::new());
                v.iter().zip(&mut m.cols).for_each(|(x, col)| col.push(*x));
            }
        }
        self.add_chromatograms(&mut run, mrm);
        Ok(run)
    }

    fn add_chromatograms(&self, run: &mut Run, mrm: Vec<Mrm>) {
        let unit = "number of detector counts";
        for (e, Mrm { time, tic, cols }) in mrm.into_iter().enumerate().filter(|(_, m)| !m.time.is_empty()) {
            if run.chromatograms.is_empty() {
                run.chromatograms.push(Chromatogram { index: 0, id: "TIC".into(), kind: ChromKind::Tic, q1: None, q3: None, unit: unit.into(), time: time.clone(), intensity: tic });
            }
            let known = self.mrm_transitions(e);
            for (j, col) in cols.into_iter().enumerate() {
                let (q1, q3) = known.get(j).copied().unwrap_or((None, None));
                let mass = match (q1, q3) {
                    (Some(a), Some(b)) => format!(" Q1={a} Q3={b}"),
                    _ => String::new(),
                };
                let id = format!("SRM SIC{mass} sample={} period=1 experiment={} transition={j}", self.sample, e + 1);
                run.chromatograms.push(Chromatogram { index: run.chromatograms.len(), id, kind: ChromKind::Srm, q1, q3, unit: unit.into(), time: time.clone(), intensity: col });
            }
        }
    }
}
