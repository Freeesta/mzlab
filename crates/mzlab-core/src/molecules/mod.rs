//! The molecules database, `molecules/1` (`docs/schemi/molecules-1.json`): one record per molecule (names, formula, SMILES, InChIKey, classes, properties),
//! shared by the tools of the app. The mass is always computed here from formula and charge. `csv.rs` and `json.rs` read and write the two forms (an error
//! for every wrong row, canonical JSON), `livelli.rs` joins catalogue < imported < user, `cerca.rs` is the one search, `rotte.rs` the bindings.

mod cerca;
mod csv;
mod json;
mod livelli;
mod rotte;

pub use cerca::cerca;
pub use csv::{from_csv, to_csv};
pub use json::{digest, errors_json, from_json, hits_json, record_json, to_json};
pub use livelli::unisci;
pub use rotte::route;

use crate::budget::sha256_hex;
use crate::elements::{fmt, mass, parse_formula, ELECTRON};
use crate::json::round;

pub const SCHEMA: &str = "molecules/1";
/// Levels of a record, from the lowest: a later one replaces the same `id` of an earlier one.
pub const LIVELLI: [&str; 3] = ["catalogo", "importata", "utente"];
/// Largest difference (Da) between the control mass of a CSV row and the computed one.
pub const MASS_TOL: f64 = 0.0005;

/// An experimental or calculated datum of a molecule; unit (`1` when it has none) and source are always there.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Proprieta {
    pub chiave: String,
    pub valore: f64,
    pub unita: String,
    pub fonte: String,
    pub condizioni: String,
}

/// Where a record comes from; `catalogo` only for the catalogue, `file` for an imported file, the dates are ISO.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Origine {
    pub tipo: String,
    pub catalogo: Vec<String>,
    pub file: Option<String>,
    pub creato: Option<String>,
    pub modificato: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Molecola {
    pub id: String,
    pub nome: String,
    pub sinonimi: Vec<String>,
    /// Hill order, no sign.
    pub formula: String,
    pub carica: i32,
    /// Monoisotopic, Da, 6 decimals: computed by `finalizza`, never read.
    pub massa: f64,
    pub smiles: Option<String>,
    pub inchikey: Option<String>,
    pub addotti: Vec<String>,
    pub raccolte: Vec<String>,
    pub classi: Vec<String>,
    pub cas: Option<String>,
    pub pubchem_cid: Option<String>,
    pub proprieta: Vec<Proprieta>,
    pub note: String,
    pub origine: Origine,
}

/// A wrong row (CSV: the 1-based line; JSON: the 1-based position of the record), an `error.molecules.*` key and the text that explains it.
#[derive(Debug, Clone, PartialEq)]
pub struct RowError {
    pub line: usize,
    pub key: &'static str,
    pub detail: String,
}

type Bad = (&'static str, String);

fn text_or_none(s: Option<String>) -> Option<String> {
    s.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

/// Trimmed, no empty items, no repeated item (case ignored, the first spelling stays); sorted when `sorted`.
fn tidy(v: &mut Vec<String>, sorted: bool, skip: &str) -> Result<(), Bad> {
    let mut out: Vec<String> = Vec::new();
    for s in v.iter().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if s.contains('|') {
            return Err(("error.molecules.badList", s.to_string()));
        }
        let low = s.to_lowercase();
        if low != skip && !out.iter().any(|o| o.to_lowercase() == low) {
            out.push(s.to_string());
        }
    }
    if sorted {
        crate::sortc::slice_by_key(&mut out, |s| (s.to_lowercase(), s.clone()));
    }
    *v = out;
    Ok(())
}

/// A valid InChIKey: 14 + 10 capitals and a final one, separated by hyphens.
pub fn is_inchikey(s: &str) -> bool {
    let p: Vec<&str> = s.split('-').collect();
    p.len() == 3
        && [14, 10, 1]
            .iter()
            .zip(&p)
            .all(|(n, x)| x.len() == *n && x.bytes().all(|b| b.is_ascii_uppercase()))
}

fn id_ok(id: &str) -> bool {
    match id.split_once(':') {
        Some(("ik", k)) => is_inchikey(k),
        Some(("u", h)) => {
            h.len() == 12
                && h.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }
        _ => false,
    }
}

fn properties(m: &mut Molecola) -> Result<(), Bad> {
    for p in &mut m.proprieta {
        for s in [&mut p.chiave, &mut p.unita, &mut p.fonte, &mut p.condizioni] {
            *s = s.trim().to_string();
        }
        if p.chiave.is_empty() || p.unita.is_empty() || p.fonte.is_empty() || !p.valore.is_finite()
        {
            return Err(("error.molecules.badProperty", p.chiave.clone()));
        }
    }
    crate::sortc::slice_by(&mut m.proprieta, |a, b| {
        (&a.chiave, &a.unita).cmp(&(&b.chiave, &b.unita))
    });
    match m.proprieta.windows(2).find(|w| w[0].chiave == w[1].chiave) {
        Some(w) => Err(("error.molecules.dupProperty", w[0].chiave.clone())),
        None => Ok(()),
    }
}

/// Puts a record in its one form: trimmed text, lists tidy, formula in Hill order, mass computed from formula and charge (and checked against `control`
/// when the file had one), the InChIKey valid, the `id` assigned when empty (never changed after).
pub fn finalizza(m: &mut Molecola, control: Option<f64>) -> Result<(), Bad> {
    m.nome = m.nome.trim().to_string();
    if m.nome.is_empty() {
        return Err(("error.molecules.noName", String::new()));
    }
    let f =
        parse_formula(&m.formula).map_err(|_| ("error.molecules.badFormula", m.formula.clone()))?;
    m.formula = fmt(&f);
    m.massa = round(mass(&f) - f64::from(m.carica) * ELECTRON, 6);
    if let Some(c) = control.filter(|c| (c - m.massa).abs() > MASS_TOL) {
        return Err(("error.molecules.massMismatch", format!("{c} / {}", m.massa)));
    }
    m.inchikey = text_or_none(m.inchikey.take()).map(|k| k.to_uppercase());
    if let Some(k) = m.inchikey.as_deref().filter(|k| !is_inchikey(k)) {
        return Err(("error.molecules.badInchikey", k.to_string()));
    }
    let nome = m.nome.to_lowercase();
    tidy(&mut m.sinonimi, true, &nome)?;
    tidy(&mut m.addotti, false, "")?;
    tidy(&mut m.raccolte, true, "")?;
    tidy(&mut m.classi, true, "")?;
    tidy(&mut m.origine.catalogo, false, "")?;
    m.smiles = text_or_none(m.smiles.take());
    m.cas = text_or_none(m.cas.take());
    m.pubchem_cid = text_or_none(m.pubchem_cid.take());
    m.note = m.note.trim().to_string();
    properties(m)?;
    if m.origine.tipo.is_empty() {
        m.origine.tipo = "utente".into();
    }
    if !LIVELLI.contains(&m.origine.tipo.as_str()) {
        return Err(("error.molecules.badOrigin", m.origine.tipo.clone()));
    }
    if m.id.is_empty() {
        m.id = match &m.inchikey {
            Some(k) => format!("ik:{k}"),
            None => format!("u:{}", &sha256_hex(record_json(m).as_bytes())[..12]),
        };
    }
    id_ok(&m.id)
        .then_some(())
        .ok_or(("error.molecules.badId", m.id.clone()))
}

/// Adds to `errors` (as `error.molecules.dupId`, at its line or position) every record whose `id` an earlier one already has, and puts the errors in the order
/// of the lines.
pub fn repeated(found: &[(usize, Molecola)], errors: &mut Vec<RowError>) {
    let mut seen = std::collections::HashSet::new();
    for (line, m) in found {
        if !seen.insert(m.id.as_str()) {
            errors.push(RowError {
                line: *line,
                key: "error.molecules.dupId",
                detail: m.id.clone(),
            });
        }
    }
    crate::sortc::slice_by_key(errors, |e| e.line);
}
