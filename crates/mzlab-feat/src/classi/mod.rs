//! Chemical classes from rules (`classi/1`): diagnostic fragments and neutral losses of an HR MS/MS spectrum, read from the data table `data/classi-1.json`.
//! Every m/z and Δm is computed from a formula with the masses of `mzlab-core`; a class is a hypothesis with a score and a level of at most 3, never an identification.

mod valuta;

pub use valuta::{evaluate, to_json, ClassHit, Evidence, Report, Unmet};

use mzlab_core::elements::{self, ELECTRON};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Value};

pub const FORMAT: &str = "classi/1";
pub const TABLE_FORMAT: &str = "classi-regole/1";
/// The table committed in the crate.
pub const BUNDLED: &str = include_str!("../../data/classi-1.json");
/// A class is listed only from this score (matched weight over the strongest diagnostic rule plus all supporting rules of the polarity, at most 1).
pub const SCORE_MIN: f64 = 0.3;
/// Peaks below this fraction of the base peak are not evidence.
pub const MIN_REL: f64 = 0.01;
/// Score from which two diagnostic matches earn level 3a (one diagnostic match, or an ambiguous one, stays 3b).
pub const SCORE_3A: f64 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    Pos,
    Neg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Frag,
    Loss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Specific: one match lets the class appear.
    Diagnostic,
    /// Not specific (H2O, CO2, shared ions): raises the score only when a diagnostic rule matched.
    Support,
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub id: String,
    pub class: usize,
    pub kind: Kind,
    pub formula: String,
    /// `None`: both polarities (losses only).
    pub polarity: Option<Polarity>,
    pub role: Role,
    pub weight: f64,
    pub why: String,
    /// Theoretical m/z of the fragment ion, or Δm of the loss.
    pub value: f64,
}

#[derive(Clone, Debug)]
pub struct Table {
    pub classes: Vec<(String, String)>,
    pub rules: Vec<Rule>,
}

fn bad(what: impl ToString) -> Error {
    Error::new("error.classi.table").with("what", what)
}

fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key).and_then(Value::str).ok_or_else(|| bad(key))
}

impl Table {
    pub fn bundled() -> Result<Table> {
        Table::parse(BUNDLED)
    }

    pub fn parse(src: &str) -> Result<Table> {
        let v = json::parse(src.as_bytes())?;
        if text(&v, "format")? != TABLE_FORMAT {
            return Err(bad("format"));
        }
        let mut t = Table {
            classes: vec![],
            rules: vec![],
        };
        for (ci, c) in v
            .get("classes")
            .map(Value::arr)
            .unwrap_or(&[])
            .iter()
            .enumerate()
        {
            t.classes
                .push((text(c, "id")?.to_string(), text(c, "name")?.to_string()));
            for r in c.get("rules").map(Value::arr).unwrap_or(&[]) {
                t.rules.push(parse_rule(r, ci)?);
            }
        }
        if t.rules.is_empty() {
            return Err(bad("rules"));
        }
        Ok(t)
    }
}

fn parse_rule(r: &Value, class: usize) -> Result<Rule> {
    let id = text(r, "id")?.to_string();
    let kind = match text(r, "kind")? {
        "frag" => Kind::Frag,
        "loss" => Kind::Loss,
        _ => return Err(bad(format!("{id}: kind"))),
    };
    let polarity = match text(r, "polarity")? {
        "+" => Some(Polarity::Pos),
        "-" => Some(Polarity::Neg),
        "both" if kind == Kind::Loss => None,
        _ => return Err(bad(format!("{id}: polarity"))),
    };
    let role = match text(r, "role")? {
        "diagnostic" => Role::Diagnostic,
        "support" => Role::Support,
        _ => return Err(bad(format!("{id}: role"))),
    };
    let weight = r
        .get("weight")
        .and_then(Value::f64)
        .filter(|w| *w > 0.0)
        .ok_or_else(|| bad(format!("{id}: weight")))?;
    let formula = text(r, "formula")?.to_string();
    let m = elements::mass(&elements::parse_formula(&formula)?);
    // An anion has gained an electron, a cation has lost one.
    let value = match (kind, polarity) {
        (Kind::Loss, _) => m,
        (Kind::Frag, Some(Polarity::Pos)) => m - ELECTRON,
        (Kind::Frag, _) => m + ELECTRON,
    };
    Ok(Rule {
        id,
        class,
        kind,
        formula,
        polarity,
        role,
        weight,
        why: text(r, "why")?.to_string(),
        value,
    })
}
