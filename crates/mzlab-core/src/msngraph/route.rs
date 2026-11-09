//! The request of `/api/msn_grafo` and the one function that answers it: the query becomes the options of the graph and the grouping of the
//! tree, the scans and their spectra become the `msngrafo/1`. Used by the `.wasm` module of the page and by the command line, so both
//! write the same bytes.

use super::{build, json, read, with_rules, Expectation, Graph, Input, Options, RuleParams};
use crate::error::{Error, Result};
use crate::idxread::Scan;
use crate::json::parse;
use crate::msntree_build::{Grouping, Tol};
use crate::source::Scansioni;

fn number(get: &dyn Fn(&str) -> Option<String>, k: &str, por: f64) -> Result<f64> {
    match get(k).filter(|v| !v.is_empty()) {
        None => Ok(por),
        Some(v) => v
            .parse::<f64>()
            .ok()
            .filter(|x| x.is_finite() && *x >= 0.0)
            .ok_or_else(|| Error::new("err.request.badNumber").with("name", k)),
    }
}

/// `nom` (or absent), `0.01` (Da) or `10ppm`, as `msn_albero` reads `tol`. A copy of `msnroute::tol_of`: sharing it moves the code of the engine,
/// whose `.wasm` has no room to spare.
fn tol_di(v: Option<&str>) -> Result<Tol> {
    let Some(t) = v.filter(|t| !t.is_empty() && *t != "nom") else {
        return Ok(Tol::Nominale);
    };
    let (num, ppm) = match t.strip_suffix("ppm") {
        Some(n) => (n, true),
        None => (t, false),
    };
    match num.trim().parse::<f64>() {
        Ok(x) if x > 0.0 && x.is_finite() && ppm => Ok(Tol::Ppm(x)),
        Ok(x) if x > 0.0 && x.is_finite() => Ok(Tol::Da(x)),
        _ => Err(Error::new("err.msn.tol").with("tol", t)),
    }
}

/// `[{"mz": 195.0877, "formula": "C10H11N4O2+"}, ...]`: the formulas the user chose among the candidates.
fn choices(text: &str) -> Result<Vec<(f64, String)>> {
    let bad = || Error::new("err.request.badNumber").with("name", "scelte");
    let v = parse(text.as_bytes()).map_err(|_| bad())?;
    v.arr()
        .iter()
        .map(|s| {
            let mz = s.get("mz").and_then(|x| x.f64()).ok_or_else(bad)?;
            let f = s.get("formula").and_then(|x| x.str()).ok_or_else(bad)?;
            Ok((mz, f.to_string()))
        })
        .collect()
}

/// `[["loss_h2o","H2O",0], ...]`: the expectations of the structure (`Expectation`: rule, formula, 1 = ion), computed by the `.wasm` `msnregole`.
fn attese(text: &str) -> Result<Vec<Expectation>> {
    let bad = || Error::new("err.request.badNumber").with("name", "attese");
    let v = parse(text.as_bytes()).map_err(|_| bad())?;
    v.arr()
        .iter()
        .map(|a| {
            let field = |i: usize| a.at(i).and_then(|x| x.str()).map(String::from);
            Ok(Expectation {
                rule: field(0).ok_or_else(bad)?,
                formula: field(1).ok_or_else(bad)?,
                ione: a.at(2).and_then(|x| x.f64()).ok_or_else(bad)? != 0.0,
            })
        })
        .collect()
}

/// The options and the grouping of a request; `get(name)` is the value of a parameter, the defaults are those of `Opzioni::default()`.
pub fn from_request(get: &dyn Fn(&str) -> Option<String>) -> Result<(Options, Grouping)> {
    let d = Options::default();
    let text = |k: &str| get(k).filter(|v| !v.is_empty());
    let o = Options {
        ppm_formula: number(get, "ppm_formula", d.ppm_formula)?,
        mz_low: number(get, "mz_basso", d.mz_low)?,
        ppm_low: number(get, "ppm_basso", d.ppm_low)?,
        evidence_min: number(get, "prove_min", d.evidence_min as f64)? as usize,
        isotopi: get("isotopi").as_deref() == Some("1"),
        activation: text("attivazione"),
        formula: text("formula"),
        smiles: text("smiles"),
        molfile: text("molfile"),
        choices: text("scelte").map_or(Ok(vec![]), |t| choices(&t))?,
        attese: text("attese").map_or(Ok(vec![]), |t| attese(&t))?,
        regole: if get("regole").as_deref() == Some("1") {
            let r = RuleParams::default();
            Some(RuleParams {
                cutoff_q: number(get, "cutoff_q", r.cutoff_q)?,
                toll_mda: number(get, "toll_perdita_mda", r.toll_mda)?,
            })
        } else {
            None
        },
        ..d
    };
    let r = Grouping {
        tol: tol_di(text("tol").as_deref())?,
        energies: get("ce").as_deref() != Some("0"),
    };
    Ok((o, r))
}

/// The graph of the scans of a file and its `msngrafo/1`; `nome` and `sha` (SHA-256 of its `.mzidx`) go in the `sorgente`.
pub fn graph_json<M: Scansioni + ?Sized>(
    scans: &[Scan],
    m: &mut M,
    (nome, sha): (&str, &str),
    (o, r): (&Options, &Grouping),
) -> Result<(Graph, String)> {
    let l = read(scans, m, r, o.budget)?;
    let g = build(
        &Input {
            albero: &l.albero,
            ms1: l.ms1.as_ref(),
            spettri: &l.spettri,
            widths: &l.larghezze,
            polarita: l.polarita,
        },
        o,
    );
    let j = json(&g, o, Some((nome, sha)));
    let j = match &o.regole {
        Some(p) => with_rules(&j, &g, o, p),
        None => j,
    };
    Ok((g, j))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn con(q: &'static str) -> Result<(Options, Grouping)> {
        let q = crate::routes::Query::parse(q);
        from_request(&|k| q.get(k).map(String::from))
    }

    #[test]
    fn an_empty_request_has_the_defaults() {
        let (o, r) = con("").unwrap();
        assert_eq!(o, Options::default());
        assert_eq!(r, Grouping::default());
    }

    #[test]
    fn the_parameters_of_the_page_are_read() {
        let q = "formula=C8H11N4O2%2B&ppm_formula=5&prove_min=2&isotopi=1&attivazione=hcd&tol=10ppm&ce=0\
                 &scelte=%5B%7B%22mz%22%3A195.0877%2C%22formula%22%3A%22C8H11N4O2%2B%22%7D%5D";
        let (o, r) = con(q).unwrap();
        assert_eq!(o.formula.as_deref(), Some("C8H11N4O2+"));
        assert_eq!((o.ppm_formula, o.evidence_min, o.isotopi), (5.0, 2, true));
        assert_eq!(o.activation.as_deref(), Some("hcd"));
        assert_eq!(o.choices, [(195.0877, "C8H11N4O2+".to_string())]);
        assert_eq!((r.tol, r.energies), (Tol::Ppm(10.0), false));
    }

    #[test]
    fn a_bad_number_or_choice_is_a_keyed_error() {
        for q in [
            "ppm_formula=abc",
            "mz_basso=-1",
            "scelte=%5B%7B%22mz%22%3A1%7D%5D",
            "scelte=x",
            "tol=zz",
        ] {
            assert!(con(q).is_err(), "{q}");
        }
        assert_eq!(
            con("ppm_formula=x").err().unwrap().key,
            "err.request.badNumber"
        );
    }
}
