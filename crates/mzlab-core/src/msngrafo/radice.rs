//! Candidate formulas for the ion of the root when the user gave none (`docs/agenti/msn-grafo.md` §2): up to `MAX` formulas of `composition::compose` for
//! the m/z of the first root of the graph, to fill the field `formula` of the options. Candidates only, never an identification.

use super::{Grafo, Opzioni};
use crate::composition::{compose, Opts};
use crate::json::parse;

/// How many formulas are offered.
const MAX: usize = 8;

/// A formula of the ion of the root that fits its m/z.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidatoRadice {
    /// Written as the option `formula` takes it: `C8H11N4O2+`.
    pub formula: String,
    pub mz_teorico: f64,
    pub ppm: f64,
}

/// The candidates for the first root of a graph with no ion formula; none when it has one, or no root.
pub fn candidati_radice(g: &Grafo, o: &Opzioni) -> Vec<CandidatoRadice> {
    let Some(&r) = g.radici.first().filter(|_| g.ione.is_none()) else {
        return vec![];
    };
    let segno = if g.polarita < 0 { '-' } else { '+' };
    let c = Opts {
        ion: if g.polarita < 0 { "[M]-" } else { "[M]+" },
        tol: o.ppm_formula,
        rdb: (-0.5, 100.0),
        max_results: MAX,
        ..Opts::default()
    };
    let Some(v) = compose(g.nodi[r].mz, &c)
        .ok()
        .and_then(|t| parse(t.as_bytes()).ok())
    else {
        return vec![];
    };
    let Some(righe) = v.get("results") else {
        return vec![];
    };
    righe
        .arr()
        .iter()
        .take(MAX)
        .filter_map(|x| {
            let f = x.get("formula")?.str()?.trim_end_matches(['+', '-']);
            Some(CandidatoRadice {
                formula: format!("{f}{segno}"),
                mz_teorico: x.get("mz_calc")?.f64()?,
                ppm: x.get("delta_ppm")?.f64()?,
            })
        })
        .collect()
}
