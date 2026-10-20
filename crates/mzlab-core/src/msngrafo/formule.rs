//! Formulas of the graph from the formula of the ion of the root (`docs/agenti/msn-grafo.md` §3): the ion as typed, the root node that
//! matches it, then `scelta` proposes a sub-formula for every node below it. Only proposals: the reason and the alternatives go with each.

use super::candidati::elementi;
use super::{scelta, Avviso, Grafo, Ione, Opzioni};
use crate::elements::{embedded_adduct, parse_formula, Formula};

/// What an adduct adds to the neutral (`+1`) or takes from it (`-1`), and its polarity.
const ADDOTTI: [(&str, &str, i8, i8); 7] = [
    ("[M+H]+", "H", 1, 1),
    ("[M+NH4]+", "NH4", 1, 1),
    ("[M+Na]+", "Na", 1, 1),
    ("[M+K]+", "K", 1, 1),
    ("[M-H]-", "H", -1, -1),
    ("[M+Cl]-", "Cl", 1, -1),
    ("[M+HCOO]-", "CHO2", 1, -1),
];

/// The root is looked for within this distance when no node is within the tolerance (the isolation window of the MS2).
const FINESTRA: f64 = 0.5;

pub(super) fn avviso(g: &mut Grafo, chiave: &'static str, nodo: Option<usize>) {
    g.avvisi.push(Avviso {
        chiave,
        albero: String::new(),
        nodo,
    });
}

/// The ion of a text: `C8H11N4O2+` (the ion itself), `C8H10N4O2+H` or `C8H10N4O2 [M+H]+` (neutral and adduct). The error is a
/// `warn.msngrafo.*` key: an unreadable formula, or an ion of the other polarity.
pub(super) fn ione(testo: &str, pol: i8) -> Result<Formula, &'static str> {
    let (resto, addotto) = embedded_adduct(testo);
    let segno = |c: char| match c {
        '+' => 1,
        _ => -1,
    };
    let (base, carica) = match addotto {
        Some(a) => (resto, ADDOTTI.iter().find(|x| x.0 == a).map(|x| x.3)),
        None => {
            let t = resto.trim_end_matches(['+', '-']);
            (t.to_string(), resto[t.len()..].chars().next().map(segno))
        }
    };
    if carica.is_some_and(|c| c != pol) {
        return Err("warn.msngrafo.polarity");
    }
    let mut f = parse_formula(&base).map_err(|_| "warn.msngrafo.badFormula")?;
    if let Some((_, atomi, verso, _)) = addotto.and_then(|a| ADDOTTI.iter().find(|x| x.0 == a)) {
        for (el, n) in parse_formula(atomi).expect("adduct formulas are valid") {
            match (f.iter_mut().find(|(e, _)| *e == el), *verso) {
                (Some(x), 1) => x.1 += n,
                (None, 1) => f.push((el, n)),
                (Some(x), _) if x.1 >= n => x.1 -= n,
                _ => return Err("warn.msngrafo.badFormula"),
            }
        }
    }
    Ok(f)
}

/// The root node of the ion: within the tolerance, or else the closest within the isolation window (its m/z is then not the ion's,
/// as when the MS1 is missing and the precursor is the nominal of the method).
fn radice(g: &Grafo, teo: f64, o: &Opzioni) -> Option<(usize, bool)> {
    let r = g
        .radici
        .iter()
        .copied()
        .filter(|&i| (g.nodi[i].mz - teo).abs() <= FINESTRA)
        .min_by(|&a, &b| {
            (g.nodi[a].mz - teo)
                .abs()
                .total_cmp(&(g.nodi[b].mz - teo).abs())
                .then(a.cmp(&b))
        })?;
    Some((r, (g.nodi[r].mz - teo).abs() <= o.tol(teo)))
}

/// Fills `ione`, `calibrazione`, the formulas of the nodes and of the losses when `o.formula` is given.
pub(super) fn assegna(g: &mut Grafo, o: &Opzioni) {
    let Some(testo) = o.formula.as_deref() else {
        return;
    };
    let f = match ione(testo, g.polarita) {
        Ok(f) => f,
        Err(k) => return avviso(g, k, None),
    };
    let e = elementi(&f, g.polarita);
    let rc = e.conta(&f).expect("the root has its own elements");
    let teo = e.mz(&rc);
    let Some((r, entro)) = radice(g, teo, o) else {
        return avviso(g, "warn.msngrafo.rootMissing", None);
    };
    if !entro {
        avviso(g, "warn.msngrafo.rootMissing", Some(r));
    }
    let oss = g.nodi[r].mz;
    g.ione = Some(Ione {
        formula: e.testo(&rc),
        mz_teorico: teo,
        mz_osservato: oss,
        ppm: (oss - teo) / teo * 1e6,
    });
    scelta::scegli(g, &e, r, rc, o);
}
