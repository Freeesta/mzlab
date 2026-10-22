//! The text of the key `regole` of `msngrafo/1` (schema `docs/schemi/msngrafo-1.json`) from an evaluation (`regole_valuta`): fixed key order, rounded numbers.

use super::regole_valuta::{valuta, Applicata, ParamRegole, ValArco, Valutazione, VERSIONE};
use super::{Grafo, Opzioni};
use crate::json::{round, Out};

fn lista(o: &mut Out, v: &[&str]) {
    o.raw("[");
    for (i, x) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(x);
    }
    o.raw("]");
}

fn applicata_json(o: &mut Out, r: &Applicata) {
    o.raw("{\"id\":")
        .str(r.id)
        .raw(",\"formula\":")
        .str(&r.formula)
        .raw(",\"massa\":")
        .num(round(r.massa, 5))
        .raw(",\"errore_mda\":")
        .num(round(r.errore_mda, 2))
        .raw(",\"radicale\":")
        .raw(if r.radicale { "true" } else { "false" })
        .raw(",\"priorita\":")
        .int(r.priorita as i64)
        .raw(",\"energia\":")
        .str(r.energia)
        .raw(",\"refs\":");
    lista(o, &r.refs);
    o.raw("}");
}

fn arco_json(o: &mut Out, g: &Grafo, i: usize, a: &ValArco) {
    o.raw("{\"da\":")
        .str(&format!("n{}", g.archi[i].da))
        .raw(",\"a\":")
        .str(&format!("n{}", g.archi[i].a))
        .raw(",\"regole\":[");
    for (j, r) in a.regole.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" });
        applicata_json(o, r);
    }
    o.raw("],\"composta\":[");
    for (j, (x, y)) in a.composta.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" });
        lista(o, &[x, y]);
    }
    o.raw("],\"stepped\":")
        .raw(if a.stepped { "true" } else { "false" })
        .raw(",\"avvisi\":");
    lista(o, &a.avvisi);
    o.raw("}");
}

/// The value of the key `regole` (see `docs/schemi/msngrafo-1.json`): keys in a fixed order, rounded numbers.
pub fn regole_json(g: &Grafo, v: &Valutazione, p: &ParamRegole) -> String {
    let mut o = Out::default();
    o.raw("{\"versione\":")
        .int(VERSIONE as i64)
        .raw(",\"tabella\":\"fragrules/1\",\"cutoff_q\":")
        .num(p.cutoff_q)
        .raw(",\"toll_perdita_mda\":")
        .num(p.toll_mda)
        .raw(",\"addotto\":");
    match v.addotto {
        Some(a) => o.str(a),
        None => o.raw("null"),
    };
    o.raw(",\"archi\":[");
    for (i, a) in v.archi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        arco_json(&mut o, g, i, a);
    }
    o.raw("],\"ioni\":[");
    for (i, x) in v.ioni.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"nodo\":")
            .str(&format!("n{}", x.nodo))
            .raw(",\"regola\":")
            .str(x.regola)
            .raw(",\"errore_mda\":")
            .num(round(x.errore_mda, 2))
            .raw("}");
    }
    o.raw("],\"non_osservabili\":[");
    for (i, x) in v.non_osservabili.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"nodo\":")
            .str(&format!("n{}", x.nodo))
            .raw(",\"regola\":")
            .str(x.regola)
            .raw(",\"mz\":")
            .num(round(x.mz, 4))
            .raw(",\"soglia\":")
            .num(round(x.soglia, 4))
            .raw("}");
    }
    o.raw("],\"conteggi\":{\"archi_spiegati\":")
        .int(v.spiegati as i64)
        .raw(",\"archi_senza_regola\":")
        .int(v.senza_regola as i64)
        .raw(",\"intensita_spiegata\":")
        .num(round(v.intensita_spiegata, 3))
        .raw("}}");
    o.0
}

/// `grafo` (the text of an `msngrafo/1`) with the key `regole` added at the end; the text of a graph without it does not change.
pub fn con_regole(grafo: &str, g: &Grafo, o: &Opzioni, p: &ParamRegole) -> String {
    let testa = grafo.strip_suffix('}').unwrap_or(grafo);
    format!(
        "{testa},\"regole\":{}}}",
        regole_json(g, &valuta(g, o, p), p)
    )
}
