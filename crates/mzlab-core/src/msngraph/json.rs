//! The JSON `msn-graph/1` of a graph (`docs/schemi/msn-graph-1.json`): keys in a fixed order, rounded numbers, the same bytes for the same
//! input. `ione` and the formulas are `null` without the formula of the ion of the root.

use super::{Candidate, Edge, GNode, Graph, NodeFormula, Options, SpettroG, NARROW};
use crate::json::{round, Out};

fn bool_(o: &mut Out, b: bool) {
    o.raw(if b { "true" } else { "false" });
}

fn opt_str(o: &mut Out, s: Option<&str>) {
    match s {
        Some(s) => o.str(s),
        None => o.raw("null"),
    };
}

fn opt_num(o: &mut Out, v: Option<f64>, digits: usize) {
    match v {
        Some(x) => o.num(round(x, digits)),
        None => o.raw("null"),
    };
}

fn id(o: &mut Out, i: Option<usize>) {
    match i {
        Some(i) => o.str(&format!("n{i}")),
        None => o.raw("null"),
    };
}

fn options(o: &mut Out, op: &Options) {
    o.raw(",\"options\":{\"ppm\":")
        .num(op.ppm)
        .raw(",\"da_min\":")
        .num(op.da_min);
    o.raw(",\"min_rel\":")
        .num(op.min_rel)
        .raw(",\"max_peaks\":")
        .int(op.max_picchi as i64)
        .raw(",\"isotopes\":");
    bool_(o, op.isotopi);
    o.raw(",\"activation\":");
    opt_str(
        o,
        op.activation.as_ref().map(|a| a.to_lowercase()).as_deref(),
    );
    o.raw(",\"formula\":");
    opt_str(o, op.formula.as_deref());
    o.raw(",\"smiles\":");
    opt_str(o, op.smiles.as_deref());
    // only when given: a graph without a molfile keeps its bytes
    if let Some(m) = &op.molfile {
        o.raw(",\"molfile\":").str(m);
    }
    o.raw(",\"ppm_formula\":")
        .num(op.ppm_formula)
        .raw(",\"low_mz\":")
        .num(op.mz_low);
    o.raw(",\"wide_ppm\":")
        .num(op.ppm_low)
        .raw(",\"min_evidence\":")
        .int(op.evidence_min as i64)
        .raw(",\"pick\":[");
    for (i, (mz, f)) in op.choices.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"mz\":")
            .num(round(*mz, 5))
            .raw(",\"formula\":")
            .str(f)
            .raw("}");
    }
    o.raw("]}");
}

fn ione(o: &mut Out, g: &Graph) {
    o.raw(",\"ion\":");
    match &g.ione {
        Some(i) => o
            .raw("{\"formula\":")
            .str(&i.formula)
            .raw(",\"theoretical_mz\":")
            .num(round(i.theoretical_mz, 5))
            .raw(",\"observed_mz\":")
            .num(round(i.mz_observed, 5))
            .raw(",\"ppm\":")
            .num(round(i.ppm, 2))
            .raw("}"),
        None => o.raw("null"),
    };
}

fn candidates(o: &mut Out, g: &Graph) {
    o.raw(",\"root_candidates\":[");
    for (i, c) in g.candidati_radice.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"formula\":")
            .str(&c.formula)
            .raw(",\"theoretical_mz\":")
            .num(round(c.theoretical_mz, 5))
            .raw(",\"ppm\":")
            .num(round(c.ppm, 2))
            .raw("}");
    }
    o.raw("]");
}

fn testa(o: &mut Out, g: &Graph, op: &Options, source: Option<(&str, &str)>) {
    o.raw("{\"schema\":\"msn-graph/1\",\"engine\":")
        .str(env!("CARGO_PKG_VERSION"))
        .raw(",\"source\":");
    match source {
        Some((name, sha)) => o
            .raw("{\"name\":")
            .str(name)
            .raw(",\"sha256_idx\":")
            .str(sha)
            .raw("}"),
        None => o.raw("null"),
    };
    options(o, op);
    o.raw(",\"polarity\":")
        .int(g.polarita as i64)
        .raw(",\"roots\":[");
    for (i, r) in g.radici.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(&format!("n{r}"));
    }
    o.raw("]");
    ione(o, g);
    candidates(o, g);
    o.raw(",\"counts\":{\"nodes\":").int(g.nodes.len() as i64);
    o.raw(",\"edges\":")
        .int(g.edges.len() as i64)
        .raw(",\"discarded_heavy\":")
        .int(g.scartati.pesanti as i64);
    o.raw(",\"discarded_isotopes\":")
        .int(g.scartati.isotopi as i64)
        .raw(",\"discarded_weak\":")
        .int(g.scartati.deboli as i64)
        .raw(",\"discarded_formula\":")
        .int(g.scartati.formula as i64);
    o.raw("},\"warnings\":[");
    for (i, a) in g.avvisi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"key\":")
            .str(a.key)
            .raw(",\"tree\":")
            .str(&a.tree)
            .raw(",\"node\":");
        id(o, a.node);
        o.raw("}");
    }
    o.raw("]");
}

fn node(o: &mut Out, i: usize, n: &GNode) {
    o.raw("{\"id\":")
        .str(&format!("n{i}"))
        .raw(",\"mz\":")
        .num(round(n.mz, 5));
    o.raw(",\"dev_ppm\":")
        .num(round(n.dev_ppm, 2))
        .raw(",\"level\":")
        .int(n.level as i64)
        .raw(",\"isolated\":");
    bool_(o, !n.tree.is_empty());
    o.raw(",\"tree\":[");
    for (j, t) in n.tree.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" }).str(t);
    }
    o.raw("],\"observations\":")
        .int(n.observations as i64)
        .raw(",\"rel_max\":")
        .num(round(n.rel_max, 1));
    o.raw(",\"isotope_of\":");
    id(o, n.isotope_of);
    o.raw(",\"m1\":");
    opt_num(o, n.m1, 1);
    o.raw(",\"isolation\":{\"width_da\":");
    opt_num(o, n.width, 3);
    o.raw(",\"upstream_da\":");
    opt_num(o, n.upstream, 3);
    o.raw(",\"narrow\":");
    bool_(o, n.upstream.is_some_and(|w| w < NARROW));
    o.raw("},\"unexplained\":");
    match &n.unexplained {
        Some(x) => {
            o.raw("{\"n\":").int(x.n as i64).raw(",\"peaks\":[");
            for (i, (mz, rel)) in x.peaks.iter().enumerate() {
                o.raw(if i > 0 { "," } else { "" })
                    .raw("{\"mz\":")
                    .num(round(*mz, 5))
                    .raw(",\"rel\":")
                    .num(round(*rel, 1))
                    .raw("}");
            }
            o.raw("]}");
        }
        None => {
            o.raw("null");
        }
    }
    o.raw(",\"pos\":{\"col\":")
        .int(n.pos.0 as i64)
        .raw(",\"row\":")
        .int(n.pos.1 as i64)
        .raw("},\"formula\":");
    match &n.formula {
        Some(f) => formula(o, f),
        None => {
            o.raw("null");
        }
    }
    o.raw("}");
}

fn candidate(o: &mut Out, c: &Candidate) {
    o.raw("{\"formula\":")
        .str(&c.formula)
        .raw(",\"theoretical_mz\":")
        .num(round(c.theoretical_mz, 5));
    o.raw(",\"ppm\":")
        .num(round(c.ppm, 2))
        .raw(",\"dbe\":")
        .num(c.dbe)
        .raw(",\"electrons\":")
        .str(if c.even { "even" } else { "odd" });
    o.raw(",\"window\":").num(c.window).raw(",\"proofs\":[");
    for (i, p) in c.prove.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(p);
    }
    o.raw("]}");
}

fn formula(o: &mut Out, f: &NodeFormula) {
    o.raw("{\"candidates\":[");
    for (i, c) in f.candidates.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        candidate(o, c);
    }
    o.raw("],\"choice\":");
    opt_str(o, f.choice.as_deref());
    o.raw(",\"source\":");
    opt_str(o, f.provenance);
    o.raw("}");
}

fn edge(o: &mut Out, a: &Edge) {
    o.raw("{\"from\":")
        .str(&format!("n{}", a.from))
        .raw(",\"to\":")
        .str(&format!("n{}", a.to));
    o.raw(",\"loss\":{\"da_mass\":")
        .num(round(a.perdita, 5))
        .raw(",\"formula\":");
    opt_str(o, a.formula.as_ref().map(|f| f.0.as_str()));
    o.raw(",\"ppm\":");
    match &a.formula {
        Some((_, p)) => o.num(round(*p, 2)),
        None => o.raw("null"),
    };
    o.raw(",\"compatible\":[");
    for (j, f) in a.compatibili.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" }).str(f);
    }
    o.raw("]},\"isolation\":");
    bool_(o, a.isolamento);
    o.raw(",\"measurements\":[");
    for (j, m) in a.misure.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" })
            .raw("{\"tree\":")
            .str(&m.tree)
            .raw(",\"level\":")
            .int(m.level as i64);
        o.raw(",\"activation\":")
            .str(&m.att.to_lowercase())
            .raw(",\"ce\":")
            .num(m.ce);
        o.raw(",\"n_scans\":")
            .int(m.n_scans as i64)
            .raw(",\"rel\":")
            .num(round(m.rel, 1))
            .raw("}");
    }
    o.raw("]}");
}

fn spettro(o: &mut Out, s: &SpettroG) {
    o.raw("{\"tree\":")
        .str(&s.tree)
        .raw(",\"level\":")
        .int(s.level as i64);
    o.raw(",\"activation\":")
        .str(&s.att.to_lowercase())
        .raw(",\"ce\":")
        .num(s.ce)
        .raw(",\"precursor\":");
    id(o, s.precursore);
    o.raw(",\"width_da\":");
    opt_num(o, s.width, 3);
    o.raw(",\"peaks\":[");
    for (i, (mz, rel, n)) in s.picchi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"mz\":")
            .num(round(*mz, 5))
            .raw(",\"rel\":")
            .num(round(*rel, 1))
            .raw(",\"node\":");
        id(o, *n);
        o.raw("}");
    }
    o.raw("]}");
}

/// The graph as `msn-graph/1`; `source` = (name of the file, SHA-256 of its `.mzidx`) when the caller knows them.
pub fn json(g: &Graph, op: &Options, source: Option<(&str, &str)>) -> String {
    let mut o = Out::default();
    testa(&mut o, g, op, source);
    o.raw(",\"nodes\":[");
    for (i, n) in g.nodes.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        node(&mut o, i, n);
    }
    o.raw("],\"edges\":[");
    for (i, a) in g.edges.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        edge(&mut o, a);
    }
    o.raw("],\"spectra\":[");
    for (i, s) in g.spettri.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        spettro(&mut o, s);
    }
    o.raw("]}");
    super::view::with_view(o.0)
}
