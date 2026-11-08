//! The BPC of an ANDI-MS file: every decode of a scan notes its base peak, so after the index pass (or any full pass) the BPC is
//! answered from those values and no scan is decoded twice; before such a pass the scans are decoded as they always were.

use super::AndiReader;
use mzlab_core::error::Result;

impl AndiReader {
    /// Notes the base peak of scan `k` (points `it`) as `bpc_punti` computes it: the max, from 0, of what `Spectrum::intensity_f64` gives.
    pub(super) fn bpc_note(&mut self, k: usize, it: &[f64], broad: bool) {
        if self.bpc.len() != self.indice.len() {
            (self.bpc, self.bpc_seen) = (vec![f64::NAN; self.indice.len()], 0);
        }
        let Some(slot) = self.bpc.get_mut(k) else {
            return;
        };
        // the fold starts at 0.0 and f64::max drops a NaN, so a noted value is never NaN
        self.bpc_seen += slot.is_nan() as usize;
        *slot = it
            .iter()
            .map(|&x| if broad { x } else { f64::from(x as f32) })
            .fold(0.0, f64::max);
    }

    /// The base peak of every scan: the noted ones when every scan has one, else from decoding each scan.
    pub(super) fn bpc_points(&mut self) -> Result<Vec<f64>> {
        if !self.indice.is_empty() && self.bpc_seen == self.indice.len() {
            return Ok(self.bpc.clone());
        }
        let mut o = Vec::new();
        for k in 0..self.indice.len() {
            let s = self.spettro(k)?;
            o.push(
                (0..s.mz.len())
                    .map(|i| s.intensity_f64(i))
                    .fold(0.0, f64::max),
            );
        }
        Ok(o)
    }
}
