//! The chromatogram list of a sample, the same for the engine (`run.rs`) and the readers module (`run_estesi.rs`, `mzlab-cli convert`): one TIC per experiment
//! with data (full scan: the TIC of the index; MRM: the TIC of the blocks), `TIC sample=N period=1 experiment=E` when there are several, the SRM / SIM
//! chromatograms after the TIC of their experiment and the BPC of each full scan experiment last (only when its scans were decoded).

use super::modi::{CODE_FULL, CODE_Q1};
use super::{Record, Wiff};
use crate::mzml::{ChromKind, Chromatogram};

impl Wiff {
    /// Appends the BPC of each `(experiment, times, intensities)` after the transitions, so that the list a reader that decodes later (`WiffReader`) completes is in
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

    /// True when experiment `e` is a full scan (EMS or Q1): its spectra give a TIC chromatogram (the other scan experiments give none).
    pub(crate) fn is_full_scan(&self, e: usize) -> bool {
        matches!(
            self.experiments.get(e).and_then(|x| x.code),
            Some(CODE_FULL | CODE_Q1)
        )
    }

    /// One TIC per MRM experiment that has blocks (just `TIC` when there is one) and one SRM chromatogram per transition that was acquired at least once.
    /// With `trim_scheduled`, a scheduled transition keeps only the points within its window around the target RT, as msconvert does.
    pub(super) fn chromatograms_of(&self, blocks: Blocks) -> Vec<Chromatogram> {
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
                let (id, kind, q1, q3) = self.trace_of(e, j, t);
                chroms.push(Chromatogram {
                    index: chroms.len(),
                    id,
                    kind,
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
pub(super) struct Experiment {
    pub(super) time: Vec<f64>,
    pub(super) tic: Vec<f64>,
    /// Base peak intensity of each block of `time`, empty when the blocks were not decoded.
    pub(super) bpc: Vec<f64>,
    pub(super) cols: Vec<(Vec<f64>, Vec<f64>)>,
}

/// The blocks of all the MRM experiments.
pub(super) struct Blocks {
    pub(super) experiments: Vec<Experiment>,
}

impl Blocks {
    pub(super) fn new(experiments: usize) -> Self {
        Blocks {
            experiments: (0..experiments).map(|_| Experiment::default()).collect(),
        }
    }

    /// A point of the TIC of a full scan experiment: the TIC the index holds for the block.
    /// `base` is the base peak intensity of the block when its points were decoded.
    pub(super) fn add_tic(&mut self, r: &Record, base: Option<f64>) {
        if let Some(e) = self.experiments.get_mut(r.experiment) {
            e.time.push(r.time_min);
            e.tic.push(r.tic);
            e.bpc.extend(base);
        }
    }
}
