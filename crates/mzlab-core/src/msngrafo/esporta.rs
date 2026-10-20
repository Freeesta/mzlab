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
    s.precursore.map_or(0.0, |n| g.nodi[n].mz)
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

/// The text of the export; `nodo` keeps the spectra of that node of the tree (`SpettroG::albero`).
pub fn esporta(g: &Grafo, formato: Formato, pulito: bool, nodo: Option<&str>) -> String {
    let sel: Vec<&SpettroG> = g
        .spettri
        .iter()
        .filter(|s| nodo.map_or(true, |n| s.albero == n))
        .collect();
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
            albero: "n1".into(),
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
    fn an_unknown_format_is_a_keyed_error() {
        assert!(Formato::da_nome("xls").is_err());
        assert_eq!(Formato::da_nome("msp").unwrap(), Formato::Msp);
    }
}
