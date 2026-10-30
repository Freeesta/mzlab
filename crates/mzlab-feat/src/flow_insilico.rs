//! The text of `spectra.insilico@1`: each query MS2 spectrum against candidate structures, ranked by the peaks their bond-wise fragments explain (`ms2_engine::insilico`).
//! Bytes in, text out: `flow run` (`mzlab-cli`) and the browser (`mzlab-attributio-wasm`) run this one function. The candidates are one of three sources, told apart by
//! their bytes: a CSV with PubChemLite columns, a `screen/1` text (the structures of its hits) or the catalogue of an `insilico/1` package, whose mass-range pieces are
//! read only where a query's `db_ppm` window overlaps them (`pieces_needed` says which, the caller hands each one over inflated).

use crate::attributio::ms2_engine::insilico::{rank, read_candidates, Candidate, InSilicoOpts};
use crate::flow_ms2::num;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Out, Value};
use mzlab_lib::flowmatch::{query_id, read_spectra, Named};

/// Format of the catalogue of the candidate package (`docs/schemi/insilico-1.json`).
pub const FORMAT: &str = "insilico/1";
const PROTON: f64 = 1.00727646677;
/// The window is widened by this fraction so a piece whose edge lies on it is never missed; `rank` applies the exact `db_ppm` anyway.
const SLACK: f64 = 1e-9;

/// One piece of a package: its file and the mass range of its rows.
#[derive(Debug, Clone, PartialEq)]
pub struct PieceRange {
    pub file: String,
    pub mass_min: f64,
    pub mass_max: f64,
}

/// What the text of a catalogue gives the selection: the pieces in mass order.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub pieces: Vec<PieceRange>,
}

fn bad(what: &str) -> Error {
    Error::new("err.flow.insilico.package").with("detail", what)
}

impl Package {
    /// The pieces of an `insilico/1` catalogue; anything else is an error.
    pub fn parse(bytes: &[u8]) -> Result<Package> {
        let v = json::parse(bytes).map_err(|_| bad("json"))?;
        if v.get("format").and_then(Value::str) != Some(FORMAT) {
            return Err(bad("format"));
        }
        let mut pieces = Vec::new();
        for p in v.get("pieces").map(Value::arr).unwrap_or(&[]) {
            let file = p
                .get("file")
                .and_then(Value::str)
                .ok_or_else(|| bad("file"))?;
            let (lo, hi) = (
                p.get("mass_min").and_then(Value::f64),
                p.get("mass_max").and_then(Value::f64),
            );
            let (Some(mass_min), Some(mass_max)) = (lo, hi) else {
                return Err(bad("mass range"));
            };
            if !plain_name(file) || mass_min > mass_max {
                return Err(bad(file));
            }
            pieces.push(PieceRange {
                file: file.to_string(),
                mass_min,
                mass_max,
            });
        }
        if pieces.is_empty() {
            return Err(bad("pieces"));
        }
        Ok(Package { pieces })
    }

    /// Indexes of the pieces whose mass range overlaps `[lo, hi]`.
    pub fn select(&self, lo: f64, hi: f64) -> Vec<usize> {
        let (lo, hi) = (lo * (1.0 - SLACK), hi * (1.0 + SLACK));
        (0..self.pieces.len())
            .filter(|&i| self.pieces[i].mass_max >= lo && self.pieces[i].mass_min <= hi)
            .collect()
    }
}

/// `NNN.bin`: a piece name never leaves the folder of the catalogue.
pub fn plain_name(f: &str) -> bool {
    f.len() == 7 && f.ends_with(".bin") && f[..3].bytes().all(|b| b.is_ascii_digit())
}

/// True when the bytes are the catalogue of a candidate package.
pub fn is_package(bytes: &[u8]) -> bool {
    Package::parse(bytes).is_ok()
}

/// The structures of the hits of a `screen/1` text as candidates (one per skeleton, the first seen), those without a SMILES left out.
fn screen_candidates(v: &Value) -> Vec<Candidate> {
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for h in v.get("hits").map(Value::arr).unwrap_or(&[]) {
        let formula = h.get("formula").and_then(Value::str).unwrap_or("");
        for c in h.get("candidates").map(Value::arr).unwrap_or(&[]) {
            let s = |k: &str| c.get(k).and_then(Value::str).unwrap_or("");
            let (smiles, mass) = (s("smiles"), c.get("neutral_mass").and_then(Value::f64));
            let id = if s("skeleton").is_empty() {
                s("inchikey")
            } else {
                s("skeleton")
            };
            let Some(mass) = mass.filter(|_| !smiles.is_empty()) else {
                continue;
            };
            if seen.iter().any(|x| x == id) && !id.is_empty() {
                continue;
            }
            seen.push(id.to_string());
            out.push(Candidate {
                id: id.to_string(),
                name: s("name").to_string(),
                smiles: smiles.to_string(),
                formula: formula.to_string(),
                mass,
            });
        }
    }
    out
}

/// The candidates of a list that is not a package: a `screen/1` text, else a CSV.
fn list_candidates(bytes: &[u8]) -> Vec<Candidate> {
    if bytes.first() == Some(&b'{') {
        return match json::parse(bytes) {
            Ok(v) if v.get("format").and_then(Value::str) == Some("screen/1") => {
                screen_candidates(&v)
            }
            _ => Vec::new(),
        };
    }
    read_candidates(&String::from_utf8_lossy(bytes))
}

/// The queries of the spectra: `(id, precursor, polarity, peaks)`, those without a precursor or peaks left out.
type Query = (String, f64, &'static str, Vec<(f64, f64)>);

fn queries(spectra: Named) -> Result<Vec<Query>> {
    let mut out = Vec::new();
    for (k, s) in read_spectra(spectra)?.iter().enumerate() {
        if s.prec <= 0.0 || s.mz.is_empty() {
            continue;
        }
        let pol = if s.pol < 0 { "NEG" } else { "POS" };
        let peaks = s.mz.iter().copied().zip(s.it.iter().copied()).collect();
        out.push((query_id(k, s), s.prec, pol, peaks));
    }
    Ok(out)
}

fn neutral(prec: f64, pol: &str) -> f64 {
    if pol == "POS" {
        prec - PROTON
    } else {
        prec + PROTON
    }
}

/// The mass window `[lo, hi]` of the candidates of a precursor: the neutral mass ± `db_ppm`, widened by `SLACK` so a row on its edge is never missed
/// (`rank` applies the exact `db_ppm` anyway).
fn window(q: &Query, db_ppm: f64) -> (f64, f64) {
    let n = neutral(q.1, q.2);
    (
        n * (1.0 - db_ppm * 1e-6) * (1.0 - SLACK),
        n * (1.0 + db_ppm * 1e-6) * (1.0 + SLACK),
    )
}

/// The files of the pieces the queries of `spectra` need, in the order of the package and without repeats: only the pieces whose mass range overlaps the `db_ppm`
/// window of a precursor.
pub fn pieces_needed(
    params: &[(String, Value)],
    spectra: Named,
    catalogue: &[u8],
) -> Result<Vec<String>> {
    let pkg = Package::parse(catalogue)?;
    let db_ppm = num(params, "db_ppm");
    let mut need = vec![false; pkg.pieces.len()];
    for q in queries(spectra)? {
        let (lo, hi) = window(&q, db_ppm);
        for i in pkg.select(lo, hi) {
            need[i] = true;
        }
    }
    Ok((0..need.len())
        .filter(|&i| need[i])
        .map(|i| pkg.pieces[i].file.clone())
        .collect())
}

fn opts(params: &[(String, Value)]) -> InSilicoOpts {
    InSilicoOpts {
        db_ppm: num(params, "db_ppm"),
        frag_ppm: num(params, "frag_ppm"),
        max_bonds: num(params, "max_bonds").max(1.0) as usize,
        top: num(params, "top").max(1.0) as usize,
        terms: params
            .iter()
            .any(|(k, v)| k == "score_terms" && v.bool() == Some(true)),
    }
}

/// The candidates of each query out of the pieces its window overlaps. The pieces are read one at a time, in the order of the package, each once (`piece(file)` gives
/// its inflated CSV) and dropped after the rows of the windows are taken, so only one piece is in memory.
fn package_candidates(
    pkg: &Package,
    qs: &[Query],
    db_ppm: f64,
    piece: &mut dyn FnMut(&str) -> Result<Vec<u8>>,
) -> Result<Vec<Vec<Candidate>>> {
    let wins: Vec<(f64, f64)> = qs.iter().map(|q| window(q, db_ppm)).collect();
    let mut per: Vec<Vec<Candidate>> = vec![Vec::new(); qs.len()];
    for (i, pc) in pkg.pieces.iter().enumerate() {
        let hit: Vec<usize> = (0..qs.len())
            .filter(|&k| pkg.select(wins[k].0, wins[k].1).contains(&i))
            .collect();
        if hit.is_empty() {
            continue;
        }
        let rows = read_candidates(&String::from_utf8_lossy(&piece(&pc.file)?));
        for k in hit {
            per[k].extend(
                rows.iter()
                    .filter(|c| c.mass >= wins[k].0 && c.mass <= wins[k].1)
                    .cloned(),
            );
        }
    }
    Ok(per)
}

/// The text of the `candidates` file: queries in file order, each query's candidates best score first, at most `top` of them. `suspects` is a CSV, a `screen/1` text or
/// the catalogue of a package, whose pieces `piece(file)` gives inflated.
pub fn insilico_text(
    params: &[(String, Value)],
    spectra: Named,
    suspects: &[u8],
    piece: &mut dyn FnMut(&str) -> Result<Vec<u8>>,
) -> Result<String> {
    let o = opts(params);
    let pkg = if suspects.first() == Some(&b'{')
        && json::parse(suspects).is_ok_and(|v| v.get("format").and_then(Value::str) == Some(FORMAT))
    {
        Some(Package::parse(suspects)?)
    } else {
        None
    };
    let list = if pkg.is_none() {
        let l = list_candidates(suspects);
        if l.is_empty() {
            return Err(Error::new("err.flow.insilico.suspects"));
        }
        l
    } else {
        Vec::new()
    };
    let qs = queries(spectra)?;
    let per = match &pkg {
        Some(p) => package_candidates(p, &qs, o.db_ppm, piece)?,
        None => Vec::new(),
    };
    let mut s = Out::default();
    s.raw("{\"format\":\"candidates/1\",\"step\":\"spectra.insilico@1\",\"algorithm\":\"fragmenter\",\"hits\":[");
    let mut first = true;
    for (k, q) in qs.iter().enumerate() {
        let ranked = rank(
            q.1,
            q.2,
            &q.3,
            if pkg.is_some() { &per[k] } else { &list },
            &o,
        );
        for h in ranked {
            if !first {
                s.raw(",");
            }
            first = false;
            let c = &h.candidate;
            s.raw("{\"query\":").str(&q.0);
            s.raw(",\"id\":").str(&c.id);
            s.raw(",\"name\":").str(&c.name);
            s.raw(",\"inchikey\":\"\",\"smiles\":").str(&c.smiles);
            s.raw(",\"formula\":").str(&c.formula);
            s.raw(",\"score\":").num(round(h.score, 6));
            if let Some(t) = &h.terms {
                s.raw(",\"score_terms\":");
                t.write(&mut s);
            }
            s.raw(",\"shared\":").int(h.explained.len() as i64);
            s.raw(",\"explained\":").rounded(&h.explained, 4);
            s.raw(",\"mass_ppm\":").num(round(h.mass_ppm, 3));
            s.raw(",\"precursor_delta\":0,\"source\":\"insilico\",\"algorithm\":\"fragmenter\"}");
        }
    }
    s.raw("]}");
    Ok(s.0)
}

#[cfg(test)]
#[path = "flow_insilico_tests.rs"]
mod tests;
