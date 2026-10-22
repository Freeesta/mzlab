//! The text of the key `regole` of `msngrafo/1` (schema `docs/schemi/msngrafo-1.json`) from an evaluation (`regole_valuta`): fixed key order, rounded numbers.

use super::regole::tabella;
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
        .raw(",\"perche\":")
        .str(r.perche)
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

fn testo_opz(o: &mut Out, v: Option<&str>) {
    match v {
        Some(x) => o.str(x),
        None => o.raw("null"),
    };
}

/// `fonti` (the references the rules of the arcs and the thresholds cite, by key, with their DOI and PMID) and `soglie` (the numeric thresholds of the table
/// with their origin and whether they are still to be calibrated): what the page shows next to a rule, so it does not carry the table.
fn fonti_e_soglie(o: &mut Out, v: &Valutazione) {
    let t = tabella();
    let mut chiavi: Vec<&str> = vec![];
    let citate = v
        .archi
        .iter()
        .flat_map(|a| a.regole.iter().flat_map(|r| r.refs.iter().copied()));
    let delle_soglie = t
        .globali
        .soglie
        .iter()
        .flat_map(|s| s.refs.iter().map(|r| r.key.as_str()));
    for k in citate.chain(delle_soglie) {
        if !chiavi.contains(&k) {
            chiavi.push(k);
        }
    }
    o.raw(",\"fonti\":[");
    let tutte = t
        .regole
        .iter()
        .flat_map(|r| r.refs.iter())
        .chain(t.globali.soglie.iter().flat_map(|s| s.refs.iter()));
    for (i, k) in chiavi.iter().enumerate() {
        let Some(r) = tutte.clone().find(|r| r.key == *k) else {
            continue;
        };
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"key\":")
            .str(k)
            .raw(",\"doi\":");
        testo_opz(o, r.doi.as_deref());
        o.raw(",\"pmid\":");
        testo_opz(o, r.pmid.as_deref());
        o.raw("}");
    }
    o.raw("],\"soglie\":[");
    for (i, s) in t.globali.soglie.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .str(&s.id)
            .raw(",\"valore\":")
            .num(s.valore)
            .raw(",\"perche\":")
            .str(&s.perche)
            .raw(",\"origine\":");
        testo_opz(o, s.origine.as_deref());
        o.raw(",\"da_calibrare\":")
            .raw(if s.da_calibrare { "true" } else { "false" })
            .raw(",\"refs\":");
        let ks: Vec<&str> = s.refs.iter().map(|r| r.key.as_str()).collect();
        lista(o, &ks);
        o.raw("}");
    }
    o.raw("]");
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
        .raw("}");
    fonti_e_soglie(&mut o, v);
    o.raw("}");
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
