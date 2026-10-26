//! The BPC of the full scan experiments of a `.wiff` for the readers module (feature `estesi`): the mzML writer reads every scan once and `LettoreWiff::watch`
//! notes its base peak on the way, so the chromatograms are listed with the BPC only after that pass and no scan is ever decoded twice.

use super::lettore::LettoreWiff;
use crate::error::Result;
use crate::lettori::VoceCromatogramma;
use crate::mzml::{ChromKind, Chromatogram, Spectrum};

impl LettoreWiff {
    /// Notes the base peak of scan `k` of injection `c` (the mzML writer reads every scan once, in order; the BPC comes from these).
    pub(super) fn watch(&mut self, c: usize, k: usize, s: &Spectrum) {
        let Some(p) = self.parti.get_mut(c) else { return };
        if p.bpc.len() != p.scans.len() {
            (p.bpc, p.bpc_seen) = (vec![f64::NAN; p.scans.len()], 0);
        }
        let Some(slot) = p.bpc.get_mut(k) else { return };
        p.bpc_seen += slot.is_nan() as usize;
        *slot = (0..s.mz.len()).map(|i| s.intensity_f64(i)).fold(0.0, f64::max);
    }

    /// The BPC of each full scan experiment of injection `c`, in the order and with the times of its TIC; none until every scan went through `watch`.
    pub(super) fn bpc_chroms(&mut self, c: usize) -> Result<Vec<Chromatogram>> {
        self.pronta(c)?;
        let many = self.chromatograms(c)?.iter().filter(|h| h.kind == ChromKind::Tic).count() > 1;
        let p = self.parte(c)?;
        if p.bpc.is_empty() || p.bpc_seen != p.scans.len() {
            return Ok(Vec::new());
        }
        let mut by_experiment: std::collections::BTreeMap<usize, (Vec<f64>, Vec<f64>)> = Default::default();
        for (&i, &base) in p.scans.iter().zip(&p.bpc) {
            let r = &p.w.records[i];
            if p.w.is_full_scan(r.experiment) {
                let (t, v) = by_experiment.entry(r.experiment).or_default();
                (t.push(r.time_min), v.push(base));
            }
        }
        let unit = "number of detector counts";
        Ok(by_experiment
            .into_iter()
            .map(|(e, (time, intensity))| Chromatogram { index: 0, id: p.w.bpc_id(e, many), kind: ChromKind::Bpc, q1: None, q3: None, unit: unit.into(), time, intensity })
            .collect())
    }

    /// Adds the BPC chromatograms of injection `c` to the list `v` (before the optical trace, as `Wiff::visit` orders them).
    pub(super) fn list_bpc(&mut self, c: usize, v: &mut Vec<VoceCromatogramma>) -> Result<()> {
        let bpc = self.bpc_chroms(c)?;
        let at = v.iter().position(|x| x.tipo == ChromKind::Pda).unwrap_or(v.len());
        v.splice(at..at, bpc.iter().map(|h| VoceCromatogramma { id: h.id.clone(), tipo: h.kind, unita: h.unit.clone() }));
        Ok(())
    }
}
