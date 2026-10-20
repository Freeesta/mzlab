//! The network of the products (`rete/1`): the parent and the accepted products of `tp/1` as nodes in columns by step, and an arc for every rule whose
//! change of formula takes the ion formula of one to that of another, with the MS2 evidence of the arc and the reason in words. A network is not an
//! identification: it raises no level. Used by the Flow step `tp.network` and the view `TP_Mine/lazy/rete-vista.js`; the layout is computed here.

pub mod arcs;
#[cfg(feature = "interp")]
mod interp_in;
mod json;
mod layout;

#[cfg(feature = "interp")]
pub use interp_in::rows_from_interp;
pub use json::SCHEMA;

use crate::hypo::Rules;
use crate::tp::{TpBlock, ST_TP};
use arcs::{best_pairs, counts, neutral_mass};

/// Largest error of the observed difference of m/z against the mass of the change, ppm of the product.
pub const PPM: f64 = 10.0;

/// Thresholds of the network; the values are recorded in the output.
#[derive(Debug, Clone, PartialEq)]
pub struct ReteParams {
    pub ppm: f64,
    /// An arc with MS2 evidence is kept when the product shares at least this many fragments with the parent (equal or shifted); 0 keeps all.
    pub min_fragments: usize,
}

impl Default for ReteParams {
    fn default() -> Self {
        ReteParams {
            ppm: PPM,
            min_fragments: 0,
        }
    }
}

/// A row of the table as the network reads it: the formula of its ion from the best hypothesis (`interp/1`) and the fragments of its MS2 against the
/// parent's. `row` is the index in the table (and in the arrays of `tp/1`), `id` the id of the row.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReteRow {
    pub row: usize,
    pub id: u32,
    pub mz: f64,
    pub rt: f64,
    /// Treatment time of the strongest area, minutes; None without a series.
    pub tmax: Option<f64>,
    pub neutral_formula: String,
    pub ion_formula: String,
    /// Fragments of the MS2 equal to a fragment of the parent and shifted by the change (m/z in the row); both empty without an MS2 compared.
    pub equal: Vec<f64>,
    pub shifted: Vec<f64>,
    pub has_ms2: bool,
}

/// A node of the network. `step` 0 is the parent; `pos` is the place in its column (by m/z, then RT, then id).
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub row: ReteRow,
    pub kinship: u8,
    pub step: u32,
    pub pos: u32,
}

/// An arc: `from` and `to` are row ids.
#[derive(Debug, Clone, PartialEq)]
pub struct Arc {
    pub from: u32,
    pub to: u32,
    pub rule: String,
    /// `literature`, `class` or `generic`: the level of the rule.
    pub scope: &'static str,
    /// Change of the ion formula from `from` to `to` (`+O`, `−2H`), with the true minus.
    pub delta: String,
    /// Change of mass, Da.
    pub dm: f64,
    /// Error of the observed difference of m/z against `dm`, ppm of the product.
    pub ppm: f64,
    pub delta_rt: f64,
    pub kinetic: &'static str,
    /// Evidence of the MS2 of the product against the parent; None when the source is not the parent or no MS2 was compared.
    pub ms2: Option<Ms2Share>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ms2Share {
    pub equal: Vec<f64>,
    pub shifted: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rete {
    pub params: ReteParams,
    /// Id of the row of the parent.
    pub parent: u32,
    pub nodes: Vec<Node>,
    pub arcs: Vec<Arc>,
}

impl Rete {
    /// Columns of the layout: the largest step plus one.
    pub fn columns(&self) -> u32 {
        self.nodes.iter().map(|n| n.step + 1).max().unwrap_or(0)
    }
}

/// The network of the accepted products (`ST_TP`) of `tp` that have a formula, with the parent as node 0. The result does not depend on the order of
/// `rows`. Without rules or without arcs the network has the parent alone.
pub fn build(
    tp: &TpBlock,
    parent: &ReteRow,
    rows: &[ReteRow],
    rules: &Rules,
    params: &ReteParams,
) -> Rete {
    let mut prods: Vec<&ReteRow> = rows
        .iter()
        .filter(|r| {
            r.row != parent.row && !r.ion_formula.is_empty() && tp.status.get(r.row) == Some(&ST_TP)
        })
        .collect();
    prods.sort_by(|a, b| {
        a.mz.total_cmp(&b.mz)
            .then(a.rt.total_cmp(&b.rt))
            .then(a.id.cmp(&b.id))
    });
    let all: Vec<&ReteRow> = std::iter::once(parent).chain(prods).collect();
    let forms: Vec<_> = all.iter().map(|r| counts(&r.ion_formula)).collect();
    let tmax: Vec<Option<f64>> = std::iter::once(Some(0.0))
        .chain(all[1..].iter().map(|r| r.tmax))
        .collect();
    let mut found = Vec::new();
    for p in best_pairs(&forms, &tmax, rules) {
        if let Some(a) = arc_of(&all, &forms, rules, &p, params) {
            found.push((p.from, p.to, a));
        }
    }
    let kept: Vec<(usize, usize)> = found.iter().map(|f| (f.0, f.1)).collect();
    let nodes = layout::nodes(&all, &kept, tp);
    let mut arcs: Vec<Arc> = found.into_iter().map(|f| f.2).collect();
    arcs.sort_by_key(|a| (a.from, a.to));
    Rete {
        params: params.clone(),
        parent: parent.id,
        nodes,
        arcs,
    }
}

fn arc_of(
    all: &[&ReteRow],
    forms: &[crate::hypo::Counts],
    rules: &Rules,
    p: &arcs::Pair,
    params: &ReteParams,
) -> Option<Arc> {
    let (a, b) = (all[p.from], all[p.to]);
    let rule = &rules.rules[p.rule];
    let dm = neutral_mass(&forms[p.to]) - neutral_mass(&forms[p.from]);
    let ppm = ((b.mz - a.mz) - dm) / b.mz * 1e6;
    if ppm.abs() > params.ppm {
        return None;
    }
    let ms2 = (p.from == 0 && b.has_ms2).then(|| Ms2Share {
        equal: b.equal.clone(),
        shifted: b.shifted.clone(),
    });
    if let Some(m) = &ms2 {
        if m.equal.len() + m.shifted.len() < params.min_fragments {
            return None;
        }
    }
    let delta = json::delta_label(&forms[p.from], &forms[p.to]);
    let reason = json::reason(&delta, rule.id.as_str(), p.kinetic, ms2.as_ref(), dm);
    Some(Arc {
        from: a.id,
        to: b.id,
        rule: rule.id.clone(),
        scope: match rule.level {
            2 => "literature",
            3 => "class",
            _ => "generic",
        },
        delta,
        dm,
        ppm,
        delta_rt: b.rt - a.rt,
        kinetic: p.kinetic,
        ms2,
        reason,
    })
}
