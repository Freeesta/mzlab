//! Formulas of the graph from the formula of the ion of the root given by the user (`docs/agenti/msn-grafo.md` §3): the ion as typed,
//! the root node that matches it, then `annota` gives every fragment the sub-formulas of the root within its window.

use super::candidates::elementi;
use super::{annotate, Graph, Ione, Options, Warning};
use crate::elements::{embedded_adduct, parse_formula, Formula};

/// What an adduct adds to the neutral (`+1`) or takes from it (`-1`), and its polarity: the adducts of `elements::adducts`, then
/// acetate (written `[M+CH3COO]-` or `+CH3COO`), which that table does not list.
const ADDOTTI: [(&str, &str, i8, i8); 9] = [
    ("[M+H]+", "H", 1, 1),
    ("[M+NH4]+", "NH4", 1, 1),
    ("[M+Na]+", "Na", 1, 1),
    ("[M+K]+", "K", 1, 1),
    ("[M-H]-", "H", -1, -1),
    ("[M+Cl]-", "Cl", 1, -1),
    ("[M+HCOO]-", "CHO2", 1, -1),
    ("[M+CH3COO]-", "C2H3O2", 1, -1),
    ("+CH3COO", "C2H3O2", 1, -1),
];

/// The adduct of a typed ion (`[M+H]+` for `C8H11N4O2+`, `C8H10N4O2+Na`...); `None` for `[M]+`/`[M]-`, the ion itself.
pub(super) fn addotto(text: &str, pol: i8) -> Option<&'static str> {
    let t: String = text.split_whitespace().collect();
    if t.ends_with("[M]+") || t.ends_with("[M]-") {
        return None;
    }
    let found = ADDOTTI[7..].iter().find(|x| t.ends_with(x.0)).or_else(|| {
        let a = embedded_adduct(&t).1?;
        ADDOTTI.iter().find(|x| x.0 == a)
    });
    Some(match (found, pol) {
        (Some(x), _) => x.0,
        (None, 1) => "[M+H]+",
        _ => "[M-H]-",
    })
}

/// The root is looked for within this distance when no node is within the tolerance (the isolation window of the MS2).
const WINDOW: f64 = 0.5;

pub(super) fn warning(g: &mut Graph, key: &'static str, node: Option<usize>) {
    g.avvisi.push(Warning {
        chiave: key,
        albero: String::new(),
        nodo: node,
    });
}

/// The ion of a text: `C8H11N4O2+` or `C8H11N4O2 [M]+` (the ion itself), `C8H10N4O2+H` or `C8H10N4O2 [M+H]+` (neutral and adduct). The error is a
/// `warn.msngrafo.*` key: an unreadable formula, or an ion of the other polarity.
pub(super) fn ione(text: &str, pol: i8) -> Result<Formula, &'static str> {
    // `[M]+` / `[M]-`: the formula is the ion itself
    let t: String = text.split_whitespace().collect();
    let testo = match t.strip_suffix("[M]+").or(t.strip_suffix("[M]-")) {
        Some(f) => format!("{f}{}", &t[t.len() - 1..]),
        None => t,
    };
    let addotto = |a: &str| ADDOTTI.iter().find(|x| x.0 == a);
    let (base, add) = match ADDOTTI[7..].iter().find(|x| testo.ends_with(x.0)) {
        Some(x) => (testo[..testo.len() - x.0.len()].to_string(), Some(x)),
        None => {
            let (rest, a) = embedded_adduct(&testo);
            (rest, a.and_then(addotto))
        }
    };
    let (base, load) = match add {
        Some(x) => (base, Some(x.3)),
        None => {
            let t = base.trim_end_matches(['+', '-']);
            let sign = base[t.len()..]
                .chars()
                .next()
                .map(|c| if c == '+' { 1 } else { -1 });
            (t.to_string(), sign)
        }
    };
    if load.is_some_and(|c| c != pol) {
        return Err("warn.msngrafo.polarity");
    }
    let mut f = parse_formula(&base).map_err(|_| "warn.msngrafo.badFormula")?;
    if let Some((_, atoms, verso, _)) = add {
        for (el, n) in parse_formula(atoms).expect("adduct formulas are valid") {
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

/// The root node of the ion: within `ppm_formula`, or else the closest within the isolation window (its m/z is then not the ion's,
/// as when the MS1 is missing and the precursor is the nominal of the method).
fn root(g: &Graph, teo: f64, o: &Options) -> Option<(usize, bool)> {
    let r = g
        .radici
        .iter()
        .copied()
        .filter(|&i| (g.nodi[i].mz - teo).abs() <= WINDOW)
        .min_by(|&a, &b| {
            (g.nodi[a].mz - teo)
                .abs()
                .total_cmp(&(g.nodi[b].mz - teo).abs())
                .then(a.cmp(&b))
        })?;
    Some((r, ((g.nodi[r].mz - teo) / teo * 1e6).abs() <= o.ppm_formula))
}

/// Fills `ione`, the formulas of the nodes and of the losses when `o.formula` is given; the nodes with no formula leave the graph.
pub(super) fn assign(g: &mut Graph, o: &Options) {
    let Some(text) = o.formula.as_deref() else {
        return;
    };
    let f = match ione(text, g.polarita) {
        Ok(f) => f,
        Err(k) => return warning(g, k, None),
    };
    let e = elementi(&f, g.polarita);
    let rc = e.conta(&f).expect("the root has its own elements");
    let teo = e.mz(&rc);
    let Some((r, entro)) = root(g, teo, o) else {
        return warning(g, "warn.msngrafo.rootMissing", None);
    };
    if !entro {
        warning(g, "warn.msngrafo.rootMissing", Some(r));
    }
    let oss = g.nodi[r].mz;
    g.ione = Some(Ione {
        formula: e.testo(&rc),
        theoretical_mz: teo,
        mz_observed: oss,
        ppm: (oss - teo) / teo * 1e6,
    });
    annotate::annotate(g, &e, r, rc, o);
}
