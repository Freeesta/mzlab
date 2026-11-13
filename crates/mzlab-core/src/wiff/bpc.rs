//! The BPC of the full scan experiments of a `.wiff`: the mzML writer reads every scan once and `WiffReader::watch`
//! notes its base peak on the way, so the chromatograms are listed with the BPC only after that pass and no scan is ever decoded twice.

use super::chromatograms::total;
use super::reader::WiffReader;
use crate::error::Result;
use crate::mzml::{ChromKind, Chromatogram, Spectrum};
use crate::readers::ChromatogramEntry;

impl WiffReader {
    /// Notes the base peak of scan `k` of injection `c` (the mzML writer reads every scan once, in order; the BPC comes from these).
    pub(super) fn watch(&mut self, c: usize, k: usize, s: &Spectrum) {
        let Some(p) = self.parts.get_mut(c) else {
            return;
        };
        if p.bpc.len() != p.scans.len() {
            (p.bpc, p.bpc_seen) = (vec![f64::NAN; p.scans.len()], 0);
        }
        let Some(slot) = p.bpc.get_mut(k) else { return };
        p.bpc_seen += slot.is_nan() as usize;
        *slot = (0..s.mz.len())
            .map(|i| s.intensity_f64(i))
            .fold(0.0, f64::max);
    }

    /// The BPC of each full scan experiment of injection `c`, in the order and with the times of its TIC; none until every scan went through `watch`.
    pub(super) fn bpc_chroms(&mut self, c: usize) -> Result<Vec<Chromatogram>> {
        self.ready(c)?;
        let many = self
            .chromatograms(c)?
            .iter()
            .filter(|h| h.kind == ChromKind::Tic)
            .count()
            > 1;
        let p = self.part(c)?;
        if p.bpc.is_empty() || p.bpc_seen != p.scans.len() {
            return Ok(Vec::new());
        }
        // indexed by experiment, so the BPCs come in the order of their experiments, as in `Wiff::chromatograms_of`
        let mut by: Vec<(Vec<f64>, Vec<f64>)> = vec![Default::default(); p.w.experiments.len()];
        for (&i, &base) in p.scans.iter().zip(&p.bpc) {
            let Some(r) = p.w.records.get(i) else {
                continue;
            };
            if let (true, Some((t, v))) = (p.w.has_bpc(r.experiment), by.get_mut(r.experiment)) {
                t.push(r.time_min);
                v.push(base);
            }
        }
        Ok(by
            .into_iter()
            .enumerate()
            .filter(|(_, (t, _))| !t.is_empty())
            .map(|(e, (t, v))| total(0, p.w.bpc_id(e, many), ChromKind::Bpc, t, v))
            .collect())
    }

    /// Adds the BPC chromatograms of injection `c` to the list `v` (before the optical trace, as `Wiff::visit` orders them).
    pub(super) fn list_bpc(&mut self, c: usize, v: &mut Vec<ChromatogramEntry>) -> Result<()> {
        let bpc = self.bpc_chroms(c)?;
        let at = v
            .iter()
            .position(|x| x.tipo == ChromKind::Pda)
            .unwrap_or(v.len());
        v.splice(
            at..at,
            bpc.iter().map(|h| ChromatogramEntry {
                id: h.id.clone(),
                tipo: h.kind,
                unita: h.unit.clone(),
            }),
        );
        Ok(())
    }
}
