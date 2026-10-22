//! The interpretation of the arcs of the graph `msngrafo/1`: each neutral loss gets its common name, formula, exact mass and error, with the reason
//! and a confidence, and the activations that observed it. Evidence, never a verdict: the user can always override it. Used by `mzlab-cli msn-graph`.

use super::Grafo;
use crate::elements::{fmt, mass, parse_formula};
use crate::json::{round, Out};

/// Version of the `interpretazione` key.
pub const VERSIONE: u32 = 1;
/// Default window of the loss match by mass, ppm of the m/z of the precursor.
pub const PPM: f64 = 10.0;
/// Smallest window in Da (low m/z, where a few ppm are below the scatter of the peaks).
const DA_MIN: f64 = 0.002;

/// Common neutral losses of ESI CID/HCD (formula, name); the mass is computed from the formula.
const NOTE: [(&str, &str); 34] = [
    ("H2", "hydrogen"),
    ("CH4", "methane"),
    ("NH3", "ammonia"),
    ("H2O", "water"),
    ("HF", "hydrogen fluoride"),
    ("C2H2", "acetylene"),
    ("HCN", "hydrogen cyanide"),
    ("CO", "carbon monoxide"),
    ("C2H4", "ethene"),
    ("CH2O", "formaldehyde"),
    ("CH5N", "methylamine"),
    ("CH4O", "methanol"),
    ("H2S", "hydrogen sulfide"),
    ("HCl", "hydrogen chloride"),
    ("C2H2O", "ketene"),
    ("C3H6", "propene"),
    ("C2H3N", "acetonitrile"),
    ("C2H4O", "acetaldehyde"),
    ("CO2", "carbon dioxide"),
    ("C2H7N", "dimethylamine"),
    ("CH2O2", "formic acid"),
    ("C2H6O", "ethanol"),
    ("C4H8", "butene"),
    ("C3H4O2", "acrylic acid"),
    ("C2H4O2", "acetic acid"),
    ("C3H6O", "acetone"),
    ("C5H8", "isoprene"),
    ("C3H6O2", "propionic acid"),
    ("SO2", "sulfur dioxide"),
    ("HBr", "hydrogen bromide"),
    ("C6H6", "benzene"),
    ("SO3", "sulfur trioxide"),
    ("HPO3", "metaphosphoric acid"),
    ("H3PO4", "phosphoric acid"),
];

/// A neutral loss of the table.
#[derive(Debug, Clone, PartialEq)]
pub struct PerditaNota {
    pub formula: String,
    pub nome: &'static str,
    pub massa: f64,
}

/// The table of the known neutral losses, in the order of the source (Hill formulas).
pub fn perdite_note() -> Vec<PerditaNota> {
    NOTE.iter()
        .filter_map(|(f, nome)| {
            let p = parse_formula(f).ok()?;
            Some(PerditaNota {
                formula: fmt(&p),
                nome,
                massa: mass(&p),
            })
        })
        .collect()
}

/// The interpreted loss of one arc.
#[derive(Debug, Clone, PartialEq)]
pub struct PerditaArco {
    /// Name of the known loss, `None` when the formula is not in the table.
    pub nome: Option<&'static str>,
    pub formula: Option<String>,
    /// Exact mass of the loss from the table, or the observed Δ m/z when it is not listed.
    pub massa: f64,
    /// Error in ppm of the m/z of the precursor (observed Δ against the exact mass).
    pub errore_ppm: f64,
    /// `formula` (from the formulas of the two nodes) or `mass` (only the Δ m/z).
    pub fonte: &'static str,
    /// `high`, `medium`, `low`.
    pub confidenza: &'static str,
    /// Other known losses within the window, by mass only.
    pub alternative: Vec<String>,
}

/// The interpretation of one arc: the loss (if any) and the distinct activations that observed it.
#[derive(Debug, Clone, PartialEq)]
pub struct InterpArco {
    pub da: usize,
    pub a: usize,
    pub perdita: Option<PerditaArco>,
    pub attivazioni: Vec<String>,
}

/// The loss of every arc of the graph, in the order of `g.archi`.
pub fn interpreta(g: &Grafo, ppm: f64) -> Vec<InterpArco> {
    let tab = perdite_note();
    g.archi
        .iter()
        .map(|a| {
            let mz = g.nodi[a.da].mz;
            let mut att: Vec<String> = a.misure.iter().map(|m| m.att.clone()).collect();
            crate::sortc::slice_ord(&mut att);
            att.dedup();
            InterpArco {
                da: a.da,
                a: a.a,
                perdita: perdita_arco(&tab, a.perdita, a.formula.as_ref(), mz, ppm),
                attivazioni: att,
            }
        })
        .collect()
}

fn perdita_arco(
    tab: &[PerditaNota],
    delta: f64,
    formula: Option<&(String, f64)>,
    mz: f64,
    ppm: f64,
) -> Option<PerditaArco> {
    if let Some((f, ppm_f)) = formula {
        let canon = parse_formula(f).ok().map(|p| fmt(&p));
        let nota = tab.iter().find(|n| Some(&n.formula) == canon.as_ref());
        return Some(PerditaArco {
            nome: nota.map(|n| n.nome),
            formula: canon,
            massa: nota.map_or(delta, |n| n.massa),
            errore_ppm: *ppm_f,
            fonte: "formula",
            confidenza: if nota.is_some() && ppm_f.abs() <= ppm {
                "high"
            } else {
                "low"
            },
            alternative: Vec::new(),
        });
    }
    let tol = (mz * ppm * 1e-6).max(DA_MIN);
    let mut v: Vec<(&PerditaNota, f64)> = tab
        .iter()
        .map(|n| (n, delta - n.massa))
        .filter(|(_, d)| d.abs() <= tol)
        .collect();
    crate::sortc::slice_by(&mut v, |x, y| {
        x.1.abs()
            .total_cmp(&y.1.abs())
            .then(x.0.formula.cmp(&y.0.formula))
    });
    let (n, d) = *v.first()?;
    Some(PerditaArco {
        nome: Some(n.nome),
        formula: Some(n.formula.clone()),
        massa: n.massa,
        errore_ppm: d / mz * 1e6,
        fonte: "mass",
        confidenza: if v.len() == 1 { "medium" } else { "low" },
        alternative: v[1..].iter().map(|(n, _)| n.formula.clone()).collect(),
    })
}

fn perdita_json(o: &mut Out, p: &PerditaArco) {
    o.raw("{\"nome\":");
    match p.nome {
        Some(n) => o.str(n),
        None => o.raw("null"),
    };
    o.raw(",\"formula\":");
    match &p.formula {
        Some(f) => o.str(f),
        None => o.raw("null"),
    };
    o.raw(",\"massa\":")
        .num(round(p.massa, 5))
        .raw(",\"errore_ppm\":")
        .num(round(p.errore_ppm, 2))
        .raw(",\"fonte\":")
        .str(p.fonte)
        .raw(",\"confidenza\":")
        .str(p.confidenza)
        .raw(",\"alternative\":[");
    for (i, f) in p.alternative.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(f);
    }
    o.raw("]}");
}

/// The value of the key `interpretazione` (see `docs/schemi/msngrafo-1.json`): keys in a fixed order, rounded numbers.
pub fn interpretazione_json(v: &[InterpArco], ppm: f64) -> String {
    let mut o = Out::default();
    o.raw("{\"versione\":")
        .int(VERSIONE as i64)
        .raw(",\"ppm\":")
        .num(ppm)
        .raw(",\"archi\":[");
    for (i, a) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"da\":")
            .str(&format!("n{}", a.da))
            .raw(",\"a\":")
            .str(&format!("n{}", a.a))
            .raw(",\"attivazioni\":[");
        for (j, t) in a.attivazioni.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).str(t);
        }
        o.raw("],\"perdita\":");
        match &a.perdita {
            Some(p) => perdita_json(&mut o, p),
            None => {
                o.raw("null");
            }
        }
        o.raw("}");
    }
    o.raw("]}");
    o.0
}

/// `grafo` (the text of an `msngrafo/1`) with the key `interpretazione` added at the end; the text of a graph without it does not change.
pub fn con_interpretazione(grafo: &str, g: &Grafo, ppm: f64) -> String {
    let testa = grafo.strip_suffix('}').unwrap_or(grafo);
    format!(
        "{testa},\"interpretazione\":{}}}",
        interpretazione_json(&interpreta(g, ppm), ppm)
    )
}
