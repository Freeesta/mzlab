//! The JSON of the MSn tree as mzFinder gives it: `MsnTree.summary()`, `engine._tree_summary` (`msn_tree`, `tree` of the run), the calibration,
//! `fragtree._hr_msn` (`frag_tree`), `fragtree.compare_hr` (`frag_tree_compare`) and the localisation of the detail.

use crate::sortc::SortC;
use mzlab_core::json::{round, Out};

use super::library::Library;
use super::localize::Localization;
use super::space::Calibration;
use super::{MsnTree, Node, Peak};

mod frag;
pub use frag::{compare, frag_tree};

fn opt_str(o: &mut Out, s: Option<&str>) {
    match s {
        Some(x) => o.str(x),
        None => o.raw("null"),
    };
}

fn ints(o: &mut Out, v: &[usize]) {
    o.raw("[");
    for (i, x) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).int(*x as i64);
    }
    o.raw("]");
}

fn rounded(v: Option<f64>, nd: usize) -> Option<f64> {
    v.map(|x| round(x, nd))
}

fn peak_full(o: &mut Out, p: &Peak) {
    o.raw("{\"mz\":")
        .num(p.mz)
        .raw(",\"mz_raw\":")
        .num(p.mz_raw)
        .raw(",\"rel\":")
        .num(p.rel);
    o.raw(",\"formula\":")
        .str(&p.formula)
        .raw(",\"ppm\":")
        .num(p.ppm);
    o.raw(",\"n_formulas\":")
        .int(p.n_formulas as i64)
        .raw(",\"n_scans\":")
        .int(p.n_scans as i64);
    if let Some(n) = p.n_atom_candidates {
        o.raw(",\"n_atom_candidates\":").int(n as i64);
    }
    o.raw("}");
}

fn path(o: &mut Out, n: &Node) {
    o.raw("[");
    for (i, (m, act, ce)) in n.path.iter().enumerate() {
        o.raw(if i > 0 { ",[" } else { "[" })
            .int(*m)
            .raw(",")
            .str(act)
            .raw(",")
            .num(f64::from_bits(*ce))
            .raw("]");
    }
    o.raw("]");
}

fn node_full(o: &mut Out, n: &Node) {
    o.raw("{\"id\":").str(&n.id).raw(",\"path\":");
    path(o, n);
    o.raw(",\"level\":")
        .int(n.level as i64)
        .raw(",\"ce\":")
        .opt(n.ce);
    o.raw(",\"n_scans\":")
        .int(n.n_scans as i64)
        .raw(",\"prec_mz\":")
        .opt(n.prec_mz)
        .raw(",\"formula\":");
    opt_str(o, n.formula.as_deref());
    o.raw(",\"ppm\":")
        .opt(n.ppm)
        .raw(",\"ghost\":")
        .raw(if n.ghost { "true" } else { "false" })
        .raw(",\"parent\":");
    opt_str(o, n.parent.as_deref());
    o.raw(",\"via\":").int(n.via as i64).raw(",\"peaks\":[");
    for (i, p) in n.peaks.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        peak_full(o, p);
    }
    o.raw("],\"atom_candidates\":");
    match &n.atom_candidates {
        Some((k, c, p)) => {
            o.raw("{\"n\":").int(*k as i64).raw(",\"certain\":");
            ints(o, c);
            o.raw(",\"possible\":");
            ints(o, p);
            o.raw("}");
        }
        None => {
            o.raw("null");
        }
    }
    if let Some([h, s, f]) = n.removed {
        o.raw(",\"removed\":{\"heavy\":")
            .int(h as i64)
            .raw(",\"not_subformula\":")
            .int(s as i64)
            .raw(",\"no_formula\":")
            .int(f as i64)
            .raw("}");
    }
    if let Some(t) = n.note {
        o.raw(",\"note\":").str(t);
    }
    if let Some(k) = n.n_formulas {
        o.raw(",\"n_formulas\":").int(k as i64);
    }
    if let Some(b) = n.base_peak_removed {
        o.raw(",\"base_peak_removed\":")
            .raw(if b { "true" } else { "false" });
    }
    o.raw("}");
}

/// `MsnTree.summary()`: every node with every key.
pub fn summary(t: &MsnTree) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, n) in t.nodes.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        node_full(&mut o, n);
    }
    o.raw("]");
    o.0
}

/// `engine._tree_summary`: the nodes, rounded, with their eight strongest peaks.
pub fn tree_summary(t: &MsnTree) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, n) in t.nodes.iter().enumerate() {
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
            .str(&n.id)
            .raw(",\"level\":")
            .int(n.level as i64)
            .raw(",\"parent\":");
        opt_str(&mut o, n.parent.as_deref());
        o.raw(",\"formula\":");
        opt_str(&mut o, n.formula.as_deref());
        o.raw(",\"prec_mz\":")
            .opt(rounded(n.prec_mz, 4))
            .raw(",\"ppm\":")
            .opt(rounded(n.ppm, 1));
        o.raw(",\"n_scans\":")
            .int(n.n_scans as i64)
            .raw(",\"ghost\":")
            .raw(if n.ghost { "true" } else { "false" });
        o.raw(",\"ce\":").opt(n.ce).raw(",\"peaks\":[");
        let mut ps: Vec<&Peak> = n.peaks.iter().collect();
        ps.sortc_by(|a, b| b.rel.total_cmp(&a.rel));
        for (j, p) in ps.iter().take(8).enumerate() {
            o.raw(if j > 0 { ",{\"mz\":" } else { "{\"mz\":" })
                .num(round(p.mz, 4))
                .raw(",\"rel\":")
                .num(round(p.rel, 1));
            o.raw(",\"formula\":")
                .str(&p.formula)
                .raw(",\"ppm\":")
                .num(round(p.ppm, 1))
                .raw("}");
        }
        o.raw("]}");
    }
    o.raw("]");
    o.0
}

/// `Calibration.as_dict()`.
pub fn calibration(c: &Calibration) -> String {
    let mut o = Out::default();
    o.raw("{\"a_ppm\":")
        .num(c.a)
        .raw(",\"b_ppm_per_mz\":")
        .num(c.b)
        .raw(",\"mz0\":")
        .num(c.mz0)
        .raw(",\"n\":")
        .int(c.n as i64)
        .raw("}");
    o.0
}

/// The entries of the library (for the parity test): formula, m/z, sources, strongest intensity, atom sets.
pub fn library(l: &Library) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, (f, e)) in l.entries.iter().enumerate() {
        o.raw(if i > 0 {
            ",{\"formula\":"
        } else {
            "{\"formula\":"
        })
        .str(f)
        .raw(",\"mz\":")
        .num(e.mz)
        .raw(",\"source\":[");
        let mut s = e.source.clone();
        s.sortc();
        for (j, x) in s.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).str(x);
        }
        o.raw("],\"rel\":").num(e.rel).raw(",\"idx\":");
        match &e.cand {
            Some((idx, w)) => {
                ints(&mut o, idx);
                o.raw(",\"w\":").floats(w);
            }
            None => {
                o.raw("null,\"w\":null");
            }
        }
        o.raw("}");
    }
    o.raw("]");
    o.0
}

/// `detail.localization` (`localize` plus `n_atoms`, and `n_groups` when it found a region).
pub fn localization(l: &Localization) -> String {
    let mut o = Out::default();
    o.raw("{\"ok\":")
        .raw(if l.found.is_some() { "true" } else { "false" })
        .raw(",\"type\":")
        .str(l.kind);
    o.raw(",\"delta\":")
        .str(&l.delta)
        .raw(",\"delta_sign\":")
        .str(l.delta_sign)
        .raw(",\"n_sites\":")
        .int(l.n_sites as i64);
    o.raw(",\"note\":")
        .str(l.note)
        .raw(",\"n_atoms\":")
        .int(l.n_atoms as i64);
    if let Some(f) = &l.found {
        o.raw(",\"region_atoms\":");
        ints(&mut o, &f.region_atoms);
        o.raw(",\"region\":")
            .str(&f.region)
            .raw(",\"ll_best\":")
            .num(f.ll_best)
            .raw(",\"n_inside\":")
            .int(f.n_inside as i64);
        o.raw(",\"top_sites\":[");
        for (i, (s, ll, at)) in f.top_sites.iter().enumerate() {
            o.raw(if i > 0 { ",{\"site\":" } else { "{\"site\":" })
                .str(s)
                .raw(",\"ll\":")
                .num(*ll)
                .raw(",\"atoms\":");
            ints(&mut o, at);
            o.raw("}");
        }
        o.raw("],\"margin\":")
            .num(f.margin)
            .raw(",\"fraction_explained\":")
            .num(f.fraction_explained);
        if let Some((at, d)) = &f.retained {
            o.raw(",\"retained_atoms\":");
            ints(&mut o, at);
            o.raw(",\"retained\":").str(d);
        }
        let s = l.evidence.iter().filter(|e| e.shifted).count();
        o.raw(",\"n_shifted\":")
            .int(s as i64)
            .raw(",\"n_unshifted\":")
            .int((l.evidence.len() - s) as i64);
        o.raw(",\"n_groups\":").int(f.n_groups as i64);
    }
    o.raw(",\"evidence\":[");
    for (i, e) in l.evidence.iter().enumerate() {
        o.raw(if i > 0 { ",{\"mz\":" } else { "{\"mz\":" })
            .num(e.mz)
            .raw(",\"rel\":")
            .num(e.rel)
            .raw(",\"formula\":")
            .str(&e.formula);
        o.raw(",\"kind\":")
            .str(if e.shifted { "S" } else { "U" })
            .raw(",\"library\":")
            .str(&e.library)
            .raw(",\"ppm\":")
            .num(e.ppm)
            .raw("}");
    }
    o.raw("]}");
    o.0
}
