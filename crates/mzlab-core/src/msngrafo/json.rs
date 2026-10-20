//! The JSON `msngrafo/1` of a graph (`docs/schemi/msngrafo-1.json`): keys in a fixed order, rounded numbers, the same bytes for the same
//! input. `ione` and the formulas are `null` without the formula of the ion of the root.

use super::{Arco, Candidato, FormulaNodo, Grafo, NodoG, Opzioni};
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

fn id(o: &mut Out, i: Option<usize>) {
    match i {
        Some(i) => o.str(&format!("n{i}")),
        None => o.raw("null"),
    };
}

fn opzioni(o: &mut Out, op: &Opzioni) {
    o.raw(",\"opzioni\":{\"ppm\":")
        .num(op.ppm)
        .raw(",\"da_min\":")
        .num(op.da_min);
    o.raw(",\"min_rel\":")
        .num(op.min_rel)
        .raw(",\"max_picchi\":")
        .int(op.max_picchi as i64)
        .raw(",\"isotopi\":");
    bool_(o, op.isotopi);
    o.raw(",\"attivazione\":");
    opt_str(
        o,
        op.attivazione.as_ref().map(|a| a.to_lowercase()).as_deref(),
    );
    o.raw(",\"formula\":");
    opt_str(o, op.formula.as_deref());
    o.raw(",\"smiles\":");
    opt_str(o, op.smiles.as_deref());
    o.raw(",\"ppm_formula\":")
        .num(op.ppm_formula)
        .raw(",\"mz_basso\":")
        .num(op.mz_basso);
    o.raw(",\"ppm_basso\":")
        .num(op.ppm_basso)
        .raw(",\"prove_min\":")
        .int(op.prove_min as i64)
        .raw(",\"scelte\":[");
    for (i, (mz, f)) in op.scelte.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"mz\":")
            .num(round(*mz, 5))
            .raw(",\"formula\":")
            .str(f)
            .raw("}");
    }
    o.raw("]}");
}

fn ione(o: &mut Out, g: &Grafo) {
    o.raw(",\"ione\":");
    match &g.ione {
        Some(i) => o
            .raw("{\"formula\":")
            .str(&i.formula)
            .raw(",\"mz_teorico\":")
            .num(round(i.mz_teorico, 5))
            .raw(",\"mz_osservato\":")
            .num(round(i.mz_osservato, 5))
            .raw(",\"ppm\":")
            .num(round(i.ppm, 2))
            .raw("}"),
        None => o.raw("null"),
    };
}

fn testa(o: &mut Out, g: &Grafo, op: &Opzioni, sorgente: Option<(&str, &str)>) {
    o.raw("{\"formato\":\"msngrafo/1\",\"motore\":")
        .str(env!("CARGO_PKG_VERSION"))
        .raw(",\"sorgente\":");
    match sorgente {
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

fn nodo(o: &mut Out, i: usize, n: &NodoG) {
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
    id(o, n.isotopo_di);
    o.raw(",\"m1\":");
    match n.m1 {
        Some(x) => o.num(round(x, 1)),
        None => o.raw("null"),
    };
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

fn candidato(o: &mut Out, c: &Candidato) {
    o.raw("{\"formula\":")
        .str(&c.formula)
        .raw(",\"mz_teorico\":")
        .num(round(c.mz_teorico, 5));
    o.raw(",\"ppm\":")
        .num(round(c.ppm, 2))
        .raw(",\"dbe\":")
        .num(c.dbe)
        .raw(",\"elettroni\":")
        .str(if c.pari { "pari" } else { "dispari" });
    o.raw(",\"finestra\":").num(c.finestra).raw(",\"prove\":[");
    for (i, p) in c.prove.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(p);
    }
    o.raw("]}");
}

fn formula(o: &mut Out, f: &FormulaNodo) {
    o.raw("{\"candidati\":[");
    for (i, c) in f.candidati.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        candidato(o, c);
    }
    o.raw("],\"scelta\":");
    opt_str(o, f.scelta.as_deref());
    o.raw(",\"fonte\":");
    opt_str(o, f.fonte);
    o.raw("}");
}

fn arco(o: &mut Out, a: &Arco) {
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
            .int(m.livello as i64);
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

/// The graph as `msngrafo/1`; `sorgente` = (name of the file, SHA-256 of its `.mzidx`) when the caller knows them.
pub fn json(g: &Grafo, op: &Opzioni, sorgente: Option<(&str, &str)>) -> String {
    let mut o = Out::default();
    testa(&mut o, g, op, sorgente);
    o.raw(",\"nodi\":[");
    for (i, n) in g.nodi.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        nodo(&mut o, i, n);
    }
    o.raw("],\"archi\":[");
    for (i, a) in g.archi.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        arco(&mut o, a);
    }
    o.raw("]}");
    o.0
}
