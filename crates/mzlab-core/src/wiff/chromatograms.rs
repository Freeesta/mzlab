//! The chromatogram list of a sample, the same for the engine (`run.rs`) and the readers module (`run_extended.rs`, `mzlab-cli convert`): one TIC per experiment
//! with data (full scan, precursor-ion, neutral-loss and MS3: the TIC of the index; MRM: the TIC of the blocks), `TIC sample=N period=1 experiment=E` when
//! there are several; after an MRM TIC its BPC (the largest transition of each block) and its SRM / SIM chromatograms; the device traces (`devices.rs`),
//! the BPC of each full scan, precursor-ion and neutral-loss experiment (only when its scans were decoded) and the DAD TWC last, as ProteoWizard lists them.
use super::modes::{CODE_FULL, CODE_MRM, CODE_MS3, CODE_NEUTRAL_LOSS, CODE_PRECURSOR_ION, CODE_Q1};
use super::{Record, Wiff};
use crate::mzml::{ChromKind, Chromatogram};

impl Wiff {
    /// Appends the BPC of each `(experiment, times, intensities)` after the transitions, so that the list a reader that decodes later (`WiffReader`) completes is in
    /// the same order, then the DAD TWC with the index it has after the `later` BPC that reader inserts before it once the scans are read.
    fn push_bpc(
        &self,
        chroms: &mut Vec<Chromatogram>,
        bpcs: Vec<(usize, Vec<f64>, Vec<f64>)>,
        many: bool,
        later: usize,
    ) {
        for (e, time, intensity) in bpcs {
            let id = self.bpc_id(e, many);
            chroms.push(total(chroms.len(), id, ChromKind::Bpc, time, intensity));
        }
        if let Some(dad) = &self.dad {
            chroms.push(dad.twc_chromatogram(chroms.len() + later));
        }
    }

    /// The id of the BPC of experiment `e` (0-based): `BPC`, or with the sample and experiment when several experiments have a TIC.
    pub(crate) fn bpc_id(&self, e: usize, many: bool) -> String {
        match many {
            true => format!("BPC sample={} period=1 experiment={}", self.sample, e + 1),
            false => "BPC".into(),
        }
    }

    fn code(&self, e: usize) -> Option<u32> {
        self.experiments.get(e).and_then(|x| x.code)
    }

    /// True when the spectra of experiment `e` give a TIC chromatogram, as in ProteoWizard: full scan (EMS or Q1), precursor-ion, neutral-loss and MS3; the
    /// product-ion scans give none.
    pub(crate) fn has_tic(&self, e: usize) -> bool {
        matches!(
            self.code(e),
            Some(CODE_FULL | CODE_Q1 | CODE_PRECURSOR_ION | CODE_NEUTRAL_LOSS | CODE_MS3)
        )
    }

    /// True when the spectra of experiment `e` also give a BPC: the experiments with a TIC but MS3.
    pub(crate) fn has_bpc(&self, e: usize) -> bool {
        matches!(
            self.code(e),
            Some(CODE_FULL | CODE_Q1 | CODE_PRECURSOR_ION | CODE_NEUTRAL_LOSS)
        )
    }

    /// One TIC per experiment that has blocks (just `TIC` when there is one), the BPC of an MRM one and one SRM chromatogram per transition that was acquired at least once.
    /// With `trim_scheduled`, a scheduled transition keeps only the points within its window around the target RT, as msconvert does. The DAD TWC comes last,
    /// after the BPC of each full scan experiment (`push_bpc`).
    pub(super) fn chromatograms_of(&self, blocks: Blocks) -> Vec<Chromatogram> {
        let unit = "number of detector counts";
        let mut chroms = Vec::new();
        let (mut bpcs, mut later) = (Vec::new(), 0);
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
            later += (ex.bpc.is_empty() && self.has_bpc(e)) as usize;
            let (time, bpc) = (ex.time.clone(), ex.bpc);
            let full = !bpc.is_empty() && bpc.len() == time.len();
            chroms.push(total(chroms.len(), id, ChromKind::Tic, ex.time, ex.tic));
            if full && self.has_bpc(e) {
                bpcs.push((e, time, bpc));
            } else if full && self.code(e) == Some(CODE_MRM) {
                let id = self.bpc_id(e, many);
                chroms.push(total(chroms.len(), id, ChromKind::Bpc, time, bpc));
            }
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
        for d in &self.devices {
            chroms.push(Chromatogram {
                index: chroms.len(),
                ..d.clone()
            });
        }
        self.push_bpc(&mut chroms, bpcs, many, later);
        chroms
    }
}

/// A TIC or BPC chromatogram (of no transition), in detector counts.
pub(super) fn total(
    index: usize,
    id: String,
    kind: ChromKind,
    time: Vec<f64>,
    intensity: Vec<f64>,
) -> Chromatogram {
    let unit = "number of detector counts".into();
    Chromatogram {
        index,
        id,
        kind,
        q1: None,
        q3: None,
        unit,
        time,
        intensity,
    }
}

/// The MRM blocks of one experiment: the times (min) and TIC of every block, and per transition the times and cps of the scans that acquired it.
#[derive(Default)]
pub(super) struct Experiment {
    pub(super) time: Vec<f64>,
    pub(super) tic: Vec<f64>,
    /// Base peak intensity of each block of `time` (of an MRM block: its largest transition), empty when the blocks were not decoded.
    pub(super) bpc: Vec<f64>,
    pub(super) cols: Vec<(Vec<f64>, Vec<f64>)>,
}

/// The blocks of all the MRM experiments.
pub(super) struct Blocks {
    pub(super) experiments: Vec<Experiment>,
    pub(super) cycles: Cycles,
}

/// The MRM blocks (cycles) of a sample: read, and left out as damaged (a NaN or infinite cps, a sparse block that does not add up), counted by `losses`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cycles {
    pub read: usize,
    pub damaged: usize,
}

/// An MRM block as read: not there (past the end of the file), damaged (left out and counted, never a NaN in a trace), or its values.
pub(crate) enum Mrm<T> {
    Absent,
    Damaged,
    Read(T),
}

impl Blocks {
    pub(super) fn new(experiments: usize) -> Self {
        Blocks {
            experiments: (0..experiments).map(|_| Experiment::default()).collect(),
            cycles: Cycles::default(),
        }
    }

    /// The values of an MRM block, counted as read or damaged.
    pub(super) fn take<T>(&mut self, m: Mrm<T>) -> Option<T> {
        match m {
            Mrm::Absent => None,
            Mrm::Damaged => {
                self.cycles.damaged += 1;
                None
            }
            Mrm::Read(v) => {
                self.cycles.read += 1;
                Some(v)
            }
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
