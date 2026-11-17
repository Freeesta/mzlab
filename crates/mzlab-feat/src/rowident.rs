//! Identity columns of the aligned table, computed last (after `sort_rows`, so they never travel with a row): the neutral mass of the hypothesis a row
//! confirms, its adduct and where the row comes from. Either empty (not computed) or one value per row.

use mzlab_core::elements;
use mzlab_core::roles::PROTON;

use crate::aligned::Aligned;
use crate::evid::match_hypotheses_in;
use crate::hypo::Hypothesis;

/// The old path of the identity columns, which live in `aligned_meta` so that the aligned table does not depend on `rowident`.
pub use crate::aligned_meta::{RowIdent, SRC_DETECTED};

/// The adducts that `ion` can name, in the order of the codes (code = position + 1). Written in the meta `adducts` of the `.mzfeat`.
pub fn adduct_names() -> [&'static str; 7] {
    elements::adducts().map(|(a, _)| a)
}

/// Code of the adduct whose shift is `mz - neutral` (the state kept between workers does not carry the name of the adduct, the masses it does).
fn code_of(shift: f64) -> u8 {
    elements::adducts()
        .iter()
        .position(|(_, s)| (s - shift).abs() < SHIFT_TOL)
        .map_or(0, |p| p as u8 + 1)
}

/// Largest difference between the shift of a hypothesis and the one of its adduct, Da.
const SHIFT_TOL: f64 = 1e-3;

impl RowIdent {
    /// The columns of an aligned table in its final order: each row takes the hypothesis `match_hypotheses` gives it; the others are read as the
    /// ion of the polarity of the registry.
    pub fn compute(table: &Aligned, hyps: &[Hypothesis], polarity: i8) -> RowIdent {
        Self::compute_in(table, hyps, polarity, crate::evid::HYPOTHESIS_PPM)
    }

    /// `compute` with the window of m/z of the match (ppm, or minus Da at unit resolution).
    pub fn compute_in(table: &Aligned, hyps: &[Hypothesis], polarity: i8, win: f64) -> RowIdent {
        let matched = match_hypotheses_in(hyps, table, win);
        let sign = if polarity >= 0 { 1.0 } else { -1.0 };
        let own = code_of(sign * PROTON);
        let mut out = RowIdent::default();
        for (r, m) in matched.iter().enumerate() {
            let seen = table.state_counts(r).0 > 0;
            let bit = if seen { SRC_DETECTED } else { 0 };
            let (neutral, ion, src) = match m.map(|h| &hyps[h]) {
                Some(h) => (h.neutral, code_of(h.mz - h.neutral), h.sources << 1 | bit),
                None => (table.mz[r] - sign * PROTON, own, bit),
            };
            out.neutral.push(neutral);
            out.ion.push(ion);
            out.src.push(src);
        }
        out
    }
}
