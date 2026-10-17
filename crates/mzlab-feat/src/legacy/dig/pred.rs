//! The confirmation of a candidate by its MS2 and its prediction level.

use super::*;

impl Cand {
    /// Does the MS2 confirm the transformation? At least `MIN_SHIFTED` peaks moved by the mass difference (shifted fragments or common neutral
    /// losses) carrying at least `MIN_SHIFTED_COS` of the cosine, and the ion not present in the reference samples. A cleavage has no single
    /// mass difference.
    pub fn confirmation(&self) -> (bool, String) {
        if self
            .hyp
            .as_ref()
            .is_some_and(|h| h.label.starts_with("rottura"))
        {
            return (false, "a cleavage has no single mass difference".into());
        }
        let Some(m) = &self.ms2 else {
            return (false, "no MS2 for this ion".into());
        };
        let (n, share) = (m.sim.n[1] + m.sim.n[2], m.sim.cos[1] + m.sim.cos[2]);
        if n < MIN_SHIFTED || share < MIN_SHIFTED_COS {
            return (
                false,
                format!("{n} shifted fragments, {share:.2} of the cosine: not enough"),
            );
        }
        if self.kin.as_ref().is_some_and(|k| !k.1) {
            return (false, "present in the reference samples".into());
        }
        (
            true,
            format!(
                "{n} fragments shifted by {:+.4} (share of the cosine {share:.2})",
                m.sim.delta
            ),
        )
    }

    /// Prediction level (1 = strongest): the weakest step of the derivation, raised to 1 when the data confirm it; `None` without a rule.
    pub fn pred(&self) -> Option<u8> {
        let h = self.hyp.as_ref().filter(|h| (1..=4).contains(&h.level))?;
        Some(if self.confirmation().0 { 1 } else { h.level })
    }
}
