//! Identity columns of the aligned table, computed last (after `sort_rows`, so they never travel with a row): the neutral mass of the hypothesis a row
//! confirms, its adduct and where the row comes from. Either empty (not computed) or one value per row.

use mzlab_core::elements;
use mzlab_core::roles::PROTON;

use crate::aligned::Aligned;
use crate::evid::match_hypotheses;
use crate::hypo::Hypothesis;

/// Bit 0 of `RowIdent::src`: the detection found the row in at least one file (else it was added by the targeted pass).
pub const SRC_DETECTED: u8 = 1;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RowIdent {
    /// Neutral mass of the matched hypothesis; else the one of the protonated (deprotonated) reference ion.
    pub neutral: Vec<f64>,
    /// Adduct of that mass: 1 + the position in `adduct_names()`; 0 = none matched and no adduct assumed.
    pub ion: Vec<u8>,
    /// `Hypothesis::sources << 1 | SRC_DETECTED`; no hypothesis = 0 in the high bits.
    pub src: Vec<u8>,
}

/// The adducts that `ion` can name, in the order of the codes (code = position + 1). Written in the meta `adducts` of the `.mzfeat`.
pub fn adduct_names() -> [&'static str; 7] {
    elements::adducts().map(|(a, _)| a)
}

fn code_of(names: &[&str], adduct: &str) -> u8 {
    names
        .iter()
        .position(|a| *a == adduct)
        .map_or(0, |p| p as u8 + 1)
}

impl RowIdent {
    pub fn len(&self) -> usize {
        self.neutral.len()
    }

    pub fn is_empty(&self) -> bool {
        self.neutral.is_empty()
    }

    pub fn heap_bytes(&self) -> usize {
        self.neutral.capacity() * 8 + self.ion.capacity() + self.src.capacity()
    }

    /// The columns of an aligned table in its final order: each row takes the hypothesis `match_hypotheses` gives it; the others are read as the
    /// ion of the polarity of the registry.
    pub fn compute(table: &Aligned, hyps: &[Hypothesis], polarity: i8) -> RowIdent {
        let matched = match_hypotheses(hyps, table);
        let names = adduct_names();
        let (own, sign) = if polarity >= 0 {
            (code_of(&names, "[M+H]+"), 1.0)
        } else {
            (code_of(&names, "[M-H]-"), -1.0)
        };
        let mut out = RowIdent::default();
        for (r, m) in matched.iter().enumerate() {
            let seen = table.state_counts(r).0 > 0;
            let bit = if seen { SRC_DETECTED } else { 0 };
            let (neutral, ion, src) = match m.map(|h| &hyps[h]) {
                Some(h) => (h.neutral, code_of(&names, h.adduct), h.sources << 1 | bit),
                None => (table.mz[r] - sign * PROTON, own, bit),
            };
            out.neutral.push(neutral);
            out.ion.push(ion);
            out.src.push(src);
        }
        out
    }
}
