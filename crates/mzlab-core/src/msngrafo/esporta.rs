//! The spectra of the `msngrafo/1` as CSV, MGF or MSP text (`api/msn_grafo/export`, `mzlab-cli msn-graph`): one block per spectrum of the
//! tree, optionally only its clean peaks (those a node of the graph explains) and only the spectra of one node of the tree. Used by the
//! `.wasm` module of the page and by the command line, so both write the same bytes.

use super::{Grafo, SpettroG};
use crate::error::{Error, Result};

/// The file formats of the export.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Formato {
    Csv,
    Mgf,
    Msp,
}

impl Formato {
    pub fn da_nome(n: &str) -> Result<Formato> {
        match n {
            "csv" => Ok(Formato::Csv),
            "mgf" => Ok(Formato::Mgf),
            "msp" => Ok(Formato::Msp),
            _ => Err(Error::new("err.request.badNumber").with("name", "formato")),
        }
    }
}

/// `formato=csv|mgf|msp&pulito=1&nodo=<id>` of `api/msn_grafo/export`; `get(name)` is the value of a parameter.
pub fn da_richiesta_export(
    get: &dyn Fn(&str) -> Option<String>,
) -> Result<(Formato, bool, Option<String>)> {
    let f = Formato::da_nome(&get("formato").unwrap_or_default())?;
    let pulito = get("pulito").is_some_and(|v| v == "1" || v == "true");
    Ok((f, pulito, get("nodo").filter(|n| !n.is_empty())))
}

/// m/z of the precursor of a spectrum (0 for an MS1-less root).
fn precursore(g: &Grafo, s: &SpettroG) -> f64 {
    s.precursore
        .and_then(|n| g.nodi.get(n))
        .map_or(0.0, |n| n.mz)
}

fn picchi(s: &SpettroG, pulito: bool) -> impl Iterator<Item = &(f64, f64, Option<usize>)> {
    s.picchi.iter().filter(move |p| !pulito || p.2.is_some())
}

fn csv(g: &Grafo, sel: &[&SpettroG], pulito: bool) -> String {
    let mut t = String::from("albero,livello,attivazione,ce,precursore_mz,mz,rel,nodo\n");
    for s in sel {
        let pre = precursore(g, s);
        for (mz, rel, nodo) in picchi(s, pulito) {
            let n = nodo.map_or(String::new(), |n| n.to_string());
            t += &format!(
                "{},{},{},{},{:.5},{:.5},{:.2},{}\n",
                s.albero, s.livello, s.att, s.ce, pre, mz, rel, n
            );
        }
    }
    t
}

fn blocchi(g: &Grafo, sel: &[&SpettroG], pulito: bool, mgf: bool) -> String {
    let mut t = String::new();
    let carica = if g.polarita < 0 { "1-" } else { "1+" };
    for s in sel {
        let pre = precursore(g, s);
        let titolo = format!("{} MS{} {} CE{}", s.albero, s.livello, s.att, s.ce);
        let pk: Vec<_> = picchi(s, pulito).collect();
        if mgf {
            t += &format!("BEGIN IONS\nTITLE={titolo}\nPEPMASS={pre:.5}\nCHARGE={carica}\n");
            t += &format!("MSLEVEL={}\nCOLLISION_ENERGY={}\n", s.livello, s.ce);
        } else {
            let modo = if g.polarita < 0 { "N" } else { "P" };
            t += &format!("Name: {titolo}\nPrecursorMZ: {pre:.5}\nIon_mode: {modo}\n");
            t += &format!("Collision_energy: {}\nNum Peaks: {}\n", s.ce, pk.len());
        }
        for (mz, rel, _) in pk {
            t += &format!("{mz:.5} {rel:.2}\n");
        }
        t += if mgf { "END IONS\n\n" } else { "\n" };
    }
    t
}

/// Whether a spectrum is kept by `nodo`: `nK` (a node of the graph) = the spectra of which it is the precursor; `msn:<id>` or a bare tree id =
/// the spectra of that node of the tree (`SpettroG::albero`); `tutto`, empty or absent = all.
fn voluto(s: &SpettroG, nodo: Option<&str>) -> bool {
    let Some(n) = nodo.filter(|n| !n.is_empty() && *n != "tutto") else {
        return true;
    };
    if let Some(k) = n.strip_prefix('n').and_then(|k| k.parse::<usize>().ok()) {
        return s.precursore == Some(k);
    }
    s.albero == n.strip_prefix("msn:").unwrap_or(n)
}

/// The text of the export; `nodo` as in `voluto`.
pub fn esporta(g: &Grafo, formato: Formato, pulito: bool, nodo: Option<&str>) -> String {
    let sel: Vec<&SpettroG> = g.spettri.iter().filter(|s| voluto(s, nodo)).collect();
    match formato {
        Formato::Csv => csv(g, &sel, pulito),
        Formato::Mgf => blocchi(g, &sel, pulito, true),
        Formato::Msp => blocchi(g, &sel, pulito, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grafo() -> Grafo {
        let mut g = Grafo::default();
        g.spettri.push(SpettroG {
            albero: "195>138".into(),
            livello: 2,
            att: "hcd".into(),
            ce: 30.0,
            precursore: None,
            larghezza: None,
            picchi: vec![(100.0, 100.0, Some(0)), (50.5, 20.0, None)],
        });
        g
    }

    #[test]
    fn the_clean_spectrum_keeps_the_peaks_with_a_node() {
        let g = grafo();
        let c = esporta(&g, Formato::Csv, true, None);
        assert_eq!(c.lines().count(), 2);
        let mgf = esporta(&g, Formato::Mgf, false, None);
        assert!(mgf.contains("50.50000 20.00") && mgf.contains("END IONS"));
        let msp = esporta(&g, Formato::Msp, true, None);
        assert!(msp.contains("Num Peaks: 1"));
        assert!(esporta(&g, Formato::Csv, false, Some("zz")).lines().count() == 1);
    }

    #[test]
    fn the_node_is_a_graph_id_a_tree_id_or_everything() {
        let mut g = grafo();
        g.spettri[0].precursore = Some(2);
        let righe = |n: Option<&str>| esporta(&g, Formato::Csv, false, n).lines().count();
        // graph node: the spectra of which it is the precursor
        assert_eq!((righe(Some("n2")), righe(Some("n1"))), (3, 1));
        // tree node, bare or as `msn:<id>`
        assert_eq!(
            (
                righe(Some("195>138")),
                righe(Some("msn:195>138")),
                righe(Some("msn:zz"))
            ),
            (3, 3, 1)
        );
        // everything
        assert_eq!(
            (righe(Some("tutto")), righe(Some("")), righe(None)),
            (3, 3, 3)
        );
    }

    #[test]
    fn an_unknown_format_is_a_keyed_error() {
        assert!(Formato::da_nome("xls").is_err());
        assert_eq!(Formato::da_nome("msp").unwrap(), Formato::Msp);
    }
}
