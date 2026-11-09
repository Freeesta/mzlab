//! The JSON `msngrafo/2` of a graph (`docs/schemi/msngrafo-2.json`; `/1` had the Italian `opzioni` keys): keys in a fixed order, rounded numbers, the same bytes for the same
//! input. `ione` and the formulas are `null` without the formula of the ion of the root.

use super::{Candidate, Edge, GNode, Graph, NodeFormula, Options, SpettroG, STRETTA};
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

fn opzioni(o: &mut Out, op: &Options) {
    o.raw(",\"opzioni\":{\"ppm\":")
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
    o.raw(",\"ione\":");
    match &g.ione {
        Some(i) => o
            .raw("{\"formula\":")
            .str(&i.formula)
            .raw(",\"mz_teorico\":")
            .num(round(i.theoretical_mz, 5))
            .raw(",\"mz_osservato\":")
            .num(round(i.mz_osservato, 5))
            .raw(",\"ppm\":")
            .num(round(i.ppm, 2))
            .raw("}"),
        None => o.raw("null"),
    };
}

fn candidates(o: &mut Out, g: &Graph) {
    o.raw(",\"candidati_radice\":[");
    for (i, c) in g.candidati_radice.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"formula\":")
            .str(&c.formula)
            .raw(",\"mz_teorico\":")
            .num(round(c.theoretical_mz, 5))
            .raw(",\"ppm\":")
            .num(round(c.ppm, 2))
            .raw("}");
    }
    o.raw("]");
}

fn testa(o: &mut Out, g: &Graph, op: &Options, source: Option<(&str, &str)>) {
    o.raw("{\"formato\":\"msngrafo/2\",\"motore\":")
        .str(env!("CARGO_PKG_VERSION"))
        .raw(",\"sorgente\":");
    match source {
        Some((nome, sha)) => o
            .raw("{\"nome\":")
            .str(nome)
            .raw(",\"sha256_idx\":")
            .str(sha)
            .raw("}"),
        None => o.raw("null"),
    };
    opzioni(o, op);
    o.raw(",\"polarita\":")
        .int(g.polarita as i64)
        .raw(",\"radici\":[");
    for (i, r) in g.radici.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(&format!("n{r}"));
    }
    o.raw("]");
    ione(o, g);
    candidates(o, g);
    o.raw(",\"conteggi\":{\"nodi\":").int(g.nodi.len() as i64);
    o.raw(",\"archi\":")
        .int(g.archi.len() as i64)
        .raw(",\"scartati_pesanti\":")
        .int(g.scartati.pesanti as i64);
    o.raw(",\"scartati_isotopi\":")
        .int(g.scartati.isotopi as i64)
        .raw(",\"scartati_deboli\":")
        .int(g.scartati.deboli as i64)
        .raw(",\"scartati_formula\":")
        .int(g.scartati.formula as i64);
    o.raw("},\"avvisi\":[");
    for (i, a) in g.avvisi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"chiave\":")
            .str(a.chiave)
            .raw(",\"albero\":")
            .str(&a.albero)
            .raw(",\"nodo\":");
        id(o, a.nodo);
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
        .raw(",\"livello\":")
        .int(n.livello as i64)
        .raw(",\"isolato\":");
    bool_(o, !n.albero.is_empty());
    o.raw(",\"albero\":[");
    for (j, t) in n.albero.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" }).str(t);
    }
    o.raw("],\"osservazioni\":")
        .int(n.osservazioni as i64)
        .raw(",\"rel_max\":")
        .num(round(n.rel_max, 1));
    o.raw(",\"isotopo_di\":");
    id(o, n.isotope_of);
    o.raw(",\"m1\":");
    opt_num(o, n.m1, 1);
    o.raw(",\"isolamento\":{\"larghezza\":");
    opt_num(o, n.larghezza, 3);
    o.raw(",\"a_monte\":");
    opt_num(o, n.upstream, 3);
    o.raw(",\"stretto\":");
    bool_(o, n.upstream.is_some_and(|w| w < STRETTA));
    o.raw("},\"non_spiegati\":");
    match &n.unexplained {
        Some(x) => {
            o.raw("{\"n\":").int(x.n as i64).raw(",\"picchi\":[");
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
        .raw(",\"riga\":")
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

fn candidato(o: &mut Out, c: &Candidate) {
    o.raw("{\"formula\":")
        .str(&c.formula)
        .raw(",\"mz_teorico\":")
        .num(round(c.theoretical_mz, 5));
    o.raw(",\"ppm\":")
        .num(round(c.ppm, 2))
        .raw(",\"dbe\":")
        .num(c.dbe)
        .raw(",\"elettroni\":")
        .str(if c.even { "pari" } else { "dispari" });
    o.raw(",\"finestra\":").num(c.window).raw(",\"prove\":[");
    for (i, p) in c.prove.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(p);
    }
    o.raw("]}");
}

fn formula(o: &mut Out, f: &NodeFormula) {
    o.raw("{\"candidati\":[");
    for (i, c) in f.candidates.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        candidato(o, c);
    }
    o.raw("],\"scelta\":");
    opt_str(o, f.choice.as_deref());
    o.raw(",\"fonte\":");
    opt_str(o, f.provenance);
    o.raw("}");
}

fn edge(o: &mut Out, a: &Edge) {
    o.raw("{\"da\":")
        .str(&format!("n{}", a.da))
        .raw(",\"a\":")
        .str(&format!("n{}", a.a));
    o.raw(",\"perdita\":{\"da_massa\":")
        .num(round(a.perdita, 5))
        .raw(",\"formula\":");
    opt_str(o, a.formula.as_ref().map(|f| f.0.as_str()));
    o.raw(",\"ppm\":");
    match &a.formula {
        Some((_, p)) => o.num(round(*p, 2)),
        None => o.raw("null"),
    };
    o.raw(",\"compatibile\":[");
    for (j, f) in a.compatibili.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" }).str(f);
    }
    o.raw("]},\"isolamento\":");
    bool_(o, a.isolamento);
    o.raw(",\"misure\":[");
    for (j, m) in a.misure.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" })
            .raw("{\"albero\":")
            .str(&m.albero)
            .raw(",\"livello\":")
            .int(m.level as i64);
        o.raw(",\"att\":")
            .str(&m.att.to_lowercase())
            .raw(",\"ce\":")
            .num(m.ce);
        o.raw(",\"n_scansioni\":")
            .int(m.n_scansioni as i64)
            .raw(",\"rel\":")
            .num(round(m.rel, 1))
            .raw("}");
    }
    o.raw("]}");
}

fn spettro(o: &mut Out, s: &SpettroG) {
    o.raw("{\"albero\":")
        .str(&s.albero)
        .raw(",\"livello\":")
        .int(s.level as i64);
    o.raw(",\"att\":")
        .str(&s.att.to_lowercase())
        .raw(",\"ce\":")
        .num(s.ce)
        .raw(",\"precursore\":");
    id(o, s.precursore);
    o.raw(",\"larghezza\":");
    opt_num(o, s.larghezza, 3);
    o.raw(",\"picchi\":[");
    for (i, (mz, rel, n)) in s.picchi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"mz\":")
            .num(round(*mz, 5))
            .raw(",\"rel\":")
            .num(round(*rel, 1))
            .raw(",\"nodo\":");
        id(o, *n);
        o.raw("}");
    }
    o.raw("]}");
}

/// The graph as `msngrafo/2`; `sorgente` = (name of the file, SHA-256 of its `.mzidx`) when the caller knows them.
pub fn json(g: &Graph, op: &Options, source: Option<(&str, &str)>) -> String {
    let mut o = Out::default();
    testa(&mut o, g, op, source);
    o.raw(",\"nodi\":[");
    for (i, n) in g.nodi.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        node(&mut o, i, n);
    }
    o.raw("],\"archi\":[");
    for (i, a) in g.archi.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        edge(&mut o, a);
    }
    o.raw("],\"spettri\":[");
    for (i, s) in g.spettri.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        spettro(&mut o, s);
    }
    o.raw("]}");
    super::vista::con_vista(o.0)
}
