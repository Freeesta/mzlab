// Ported from Attributio identity.py (the column `untid` is mzLab's own, next to the identity of Attributio).
//! The identity that mzLab gives to the same feature: its neutral mass and adduct as `RowIdent` computes them for a row of the aligned table (`rowident.rs`).
//! It is a proof beside the identity of Attributio, never a verdict: mzLab groups by adducts and in-source fragments of a hypothesis, Attributio by curated
//! name and InChIKey skeleton, so the two can disagree (a name shared by two unrelated masses; two names for one structure). The table of mzMine carries one
//! m/z and one presence per site and no hypothesis registry: without hypotheses every row is read as the ion of its polarity, the rule of a row that no
//! hypothesis confirms; the caller passes the hypotheses of a registry when it has them.

use super::annotate::Feature;
use crate::aligned::{Aligned, CELL_ABSENT, CELL_DETECTED};
use crate::hypo::Hypothesis;
use crate::rowident::{adduct_names, RowIdent};

/// The identity of one feature in mzLab.
#[derive(Clone, Debug, PartialEq)]
pub struct Untid {
    /// Neutral mass of the matched hypothesis, else of the protonated (deprotonated) ion.
    pub neutral: f64,
    /// The adduct of that mass, when one is named.
    pub adduct: Option<&'static str>,
    /// A hypothesis of the registry confirmed the row.
    pub hypothesis: bool,
}

impl Untid {
    /// `199.0947 [M+H]+`: the neutral mass and the adduct.
    pub fn text(&self) -> String {
        match self.adduct {
            Some(a) => format!("{:.4} {a}", self.neutral),
            None => format!("{:.4}", self.neutral),
        }
    }
}

/// One `Untid` per feature (`None`: the feature has no m/z or its polarity is not POS or NEG). `hyps` are the hypotheses of a registry (empty: none).
pub fn untid_column(feats: &[Feature], hyps: &[Hypothesis]) -> Vec<Option<Untid>> {
    let mut out = vec![None; feats.len()];
    for (pol, sign) in [("POS", 1i8), ("NEG", -1i8)] {
        let rows: Vec<usize> = (0..feats.len())
            .filter(|&i| feats[i].pol == pol && feats[i].mz.is_some())
            .collect();
        let mut t = Aligned::with_capacity(1, rows.len());
        for &i in &rows {
            let f = &feats[i];
            t.mz.push(f.mz.unwrap_or(0.0));
            t.rt.push(f.rt.unwrap_or(f64::NAN));
            t.state.push(if f.in_site.iter().any(|(_, v)| *v) {
                CELL_DETECTED
            } else {
                CELL_ABSENT
            });
        }
        let mine: Vec<Hypothesis> = hyps
            .iter()
            .filter(|h| (h.polarity >= 0) == (sign >= 0))
            .cloned()
            .collect();
        let id = RowIdent::compute(&t, &mine, sign);
        let names = adduct_names();
        for (k, &i) in rows.iter().enumerate() {
            out[i] = Some(Untid {
                neutral: id.neutral[k],
                adduct: (id.ion[k] as usize)
                    .checked_sub(1)
                    .and_then(|p| names.get(p))
                    .copied(),
                hypothesis: id.src[k] >> 1 != 0,
            });
        }
    }
    out
}
