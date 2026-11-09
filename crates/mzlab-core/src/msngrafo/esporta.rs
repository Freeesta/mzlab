//! The spectra of the `msngrafo/1` as CSV, MGF or MSP text (`api/msn_grafo/export`, `mzlab-cli msn-graph`): one block per spectrum of the
//! tree, optionally only its clean peaks (those a node of the graph explains) and only the spectra of one node of the tree. Used by the
//! `.wasm` module of the page and by the command line, so both write the same bytes.

use super::{Graph, SpettroG};
use crate::error::{Error, Result};

/// The file formats of the export.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Format {
    Csv,
    Mgf,
    Msp,
}

impl Format {
    pub fn from_name(n: &str) -> Result<Format> {
        match n {
            "csv" => Ok(Format::Csv),
            "mgf" => Ok(Format::Mgf),
            "msp" => Ok(Format::Msp),
            _ => Err(Error::new("err.request.badNumber").with("name", "formato")),
        }
    }
}

/// `formato=csv|mgf|msp&pulito=1&nodo=<id>` of `api/msn_grafo/export`; `get(name)` is the value of a parameter.
pub fn from_export_request(
    get: &dyn Fn(&str) -> Option<String>,
) -> Result<(Format, bool, Option<String>)> {
    let f = Format::from_name(&get("formato").unwrap_or_default())?;
    let clean = get("pulito").is_some_and(|v| v == "1" || v == "true");
    Ok((f, clean, get("nodo").filter(|n| !n.is_empty())))
}

/// m/z of the precursor of a spectrum (0 for an MS1-less root).
fn precursore(g: &Graph, s: &SpettroG) -> f64 {
    s.precursore
        .and_then(|n| g.nodi.get(n))
        .map_or(0.0, |n| n.mz)
}

fn peaks(s: &SpettroG, clean: bool) -> impl Iterator<Item = &(f64, f64, Option<usize>)> {
    s.picchi.iter().filter(move |p| !clean || p.2.is_some())
}

fn csv(g: &Graph, sel: &[&SpettroG], clean: bool) -> String {
    let mut t = String::from("albero,livello,attivazione,ce,precursore_mz,mz,rel,nodo\n");
    for s in sel {
        let pre = precursore(g, s);
        for (mz, rel, node) in peaks(s, clean) {
            let n = node.map_or(String::new(), |n| n.to_string());
            t += &format!(
                "{},{},{},{},{:.5},{:.5},{:.2},{}\n",
                s.albero, s.level, s.att, s.ce, pre, mz, rel, n
            );
        }
    }
    t
}

fn blocks(g: &Graph, sel: &[&SpettroG], clean: bool, mgf: bool) -> String {
    let mut t = String::new();
    let load = if g.polarita < 0 { "1-" } else { "1+" };
    for s in sel {
        let pre = precursore(g, s);
        let titolo = format!("{} MS{} {} CE{}", s.albero, s.level, s.att, s.ce);
        let pk: Vec<_> = peaks(s, clean).collect();
        if mgf {
            t += &format!("BEGIN IONS\nTITLE={titolo}\nPEPMASS={pre:.5}\nCHARGE={load}\n");
            t += &format!("MSLEVEL={}\nCOLLISION_ENERGY={}\n", s.level, s.ce);
        } else {
            let mode = if g.polarita < 0 { "N" } else { "P" };
            t += &format!("Name: {titolo}\nPrecursorMZ: {pre:.5}\nIon_mode: {mode}\n");
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
fn wanted(s: &SpettroG, node: Option<&str>) -> bool {
    let Some(n) = node.filter(|n| !n.is_empty() && *n != "tutto") else {
        return true;
    };
    if let Some(k) = n.strip_prefix('n').and_then(|k| k.parse::<usize>().ok()) {
        return s.precursore == Some(k);
    }
    s.albero == n.strip_prefix("msn:").unwrap_or(n)
}

/// The text of the export; `nodo` as in `voluto`.
pub fn esporta(g: &Graph, format: Format, clean: bool, node: Option<&str>) -> String {
    let sel: Vec<&SpettroG> = g.spettri.iter().filter(|s| wanted(s, node)).collect();
    match format {
        Format::Csv => csv(g, &sel, clean),
        Format::Mgf => blocks(g, &sel, clean, true),
        Format::Msp => blocks(g, &sel, clean, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph() -> Graph {
        let mut g = Graph::default();
        g.spettri.push(SpettroG {
            albero: "195>138".into(),
            level: 2,
            att: "hcd".into(),
            ce: 30.0,
            precursore: None,
            width: None,
            picchi: vec![(100.0, 100.0, Some(0)), (50.5, 20.0, None)],
        });
        g
    }

    #[test]
    fn the_clean_spectrum_keeps_the_peaks_with_a_node() {
        let g = graph();
        let c = esporta(&g, Format::Csv, true, None);
        assert_eq!(c.lines().count(), 2);
        let mgf = esporta(&g, Format::Mgf, false, None);
        assert!(mgf.contains("50.50000 20.00") && mgf.contains("END IONS"));
        let msp = esporta(&g, Format::Msp, true, None);
        assert!(msp.contains("Num Peaks: 1"));
        assert!(esporta(&g, Format::Csv, false, Some("zz")).lines().count() == 1);
    }

    #[test]
    fn the_node_is_a_graph_id_a_tree_id_or_everything() {
        let mut g = graph();
        g.spettri[0].precursore = Some(2);
        let rows = |n: Option<&str>| esporta(&g, Format::Csv, false, n).lines().count();
        // graph node: the spectra of which it is the precursor
        assert_eq!((rows(Some("n2")), rows(Some("n1"))), (3, 1));
        // tree node, bare or as `msn:<id>`
        assert_eq!(
            (
                rows(Some("195>138")),
                rows(Some("msn:195>138")),
                rows(Some("msn:zz"))
            ),
            (3, 3, 1)
        );
        // everything
        assert_eq!((rows(Some("tutto")), rows(Some("")), rows(None)), (3, 3, 3));
    }

    #[test]
    fn an_unknown_format_is_a_keyed_error() {
        assert!(Format::from_name("xls").is_err());
        assert_eq!(Format::from_name("msp").unwrap(), Format::Msp);
    }
}
