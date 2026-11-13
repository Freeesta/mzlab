//! The table of fragmentation rules `fragrules/1` (`docs/studi/regole-frammentazione-schema.md`, schema `docs/schemi/fragrules-1.json`): the strict reader
//! and the embedded table `data/fragrules-1.json`. The evaluation on the graph is `regole_valuta`; the structure conditions (`requires`) are read here and
//! evaluated only when a structure is given (a later step). A rule is evidence with a source, never a verdict.

use std::sync::OnceLock;

use crate::elements::{fmt, mass, parse_formula};
use crate::error::{Error, Result};
use crate::json::{parse, Value};

/// The table shipped with the program.
const TABLE: &str = include_str!("../../data/fragrules-1.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    Loss,
    Ion,
    Cleavage,
}

/// Which activation a rule belongs to: the ion trap (`cid_trap`), the beam type (`beam`: HCD and collision-cell CID) or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activation {
    CidTrap,
    Beam,
    Both,
}

/// A reference of a rule; `check` is `abstract`, `text` or `todo`.
#[derive(Clone, Debug, PartialEq)]
pub struct Rif {
    pub key: String,
    pub pmid: Option<String>,
    pub doi: Option<String>,
    pub check: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub id: String,
    pub tipo: Tipo,
    /// Hill formula of the loss or of the ion (no charge).
    pub formula: String,
    /// Exact mass of `formula`, computed from the elements (the table holds only the formula).
    pub massa: f64,
    pub radicale: bool,
    pub all_groups: Vec<String>,
    pub any_groups: Vec<String>,
    pub site: Option<String>,
    pub excludes: Vec<String>,
    /// Empty = every adduct of the polarity.
    pub ions: Vec<String>,
    /// 1 (`+`), -1 (`-`) or 0 (both).
    pub polarita: i8,
    pub activation: Activation,
    pub energia: String,
    pub load: String,
    pub priorita: u8,
    pub after: Vec<String>,
    pub refs: Vec<Rif>,
    pub perche: String,
}

impl Rule {
    /// The rule has a condition on the structure (groups, excluded groups or a site): without a structure it is not decided.
    pub fn structure_law(&self) -> bool {
        !self.all_groups.is_empty()
            || !self.any_groups.is_empty()
            || self.site.is_some()
            || !self.excludes.is_empty()
            || self.tipo == Tipo::Cleavage
    }

    /// The rule needs no structure: only the formula, the adduct and the polarity decide.
    pub fn formula_only(&self) -> bool {
        self.all_groups.is_empty()
            && self.any_groups.is_empty()
            && self.site.is_none()
            && self.excludes.is_empty()
            && self.after.is_empty()
    }
}

/// A numeric threshold of the global rules with what the table says of it: why, where its value comes from and whether it is still to be calibrated.
#[derive(Clone, Debug, PartialEq)]
pub struct Threshold {
    pub id: String,
    pub value: f64,
    pub perche: String,
    pub origin: Option<String>,
    pub to_calibrate: bool,
    pub refs: Vec<Rif>,
}

/// The global rules with their values (the defaults of the study when the table omits one).
#[derive(Clone, Debug, PartialEq)]
pub struct Globali {
    pub thresholds: Vec<Threshold>,
    pub even_electron: bool,
    pub trap_cutoff_q: f64,
    pub beam_composite: bool,
    pub stepped_sum: bool,
    pub loss_tolerance_mda: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub rules: Vec<Rule>,
    pub globali: Globali,
}

fn error(key: &'static str, detail: impl ToString) -> Error {
    Error::new(key).with("detail", detail)
}

fn only(v: &Value, allowed: &[&str], at: &str) -> Result<()> {
    if let Value::Obj(o) = v {
        if let Some((k, _)) = o.iter().find(|(k, _)| !allowed.contains(&k.as_str())) {
            return Err(error("err.fragrules.badKey", format!("{at}.{k}")));
        }
        return Ok(());
    }
    Err(error("err.fragrules.badFormat", at))
}

fn text(v: &Value, k: &str, at: &str) -> Result<String> {
    let s = v.get(k).and_then(Value::str);
    s.map(str::to_string)
        .ok_or_else(|| error("err.fragrules.badFormat", format!("{at}.{k}")))
}

fn list(v: &Value, k: &str, at: &str) -> Result<Vec<String>> {
    let a = v
        .get(k)
        .ok_or_else(|| error("err.fragrules.badFormat", format!("{at}.{k}")))?;
    a.arr()
        .iter()
        .map(|x| {
            x.str()
                .map(str::to_string)
                .ok_or_else(|| error("err.fragrules.badFormat", format!("{at}.{k}")))
        })
        .collect()
}

fn opzionale(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::str).map(str::to_string)
}

fn references(v: &Value, at: &str) -> Result<Vec<Rif>> {
    let mut out = vec![];
    for r in v.get("refs").map_or(&[][..], Value::arr) {
        only(r, &["key", "pmid", "doi", "check"], at)?;
        let check = text(r, "check", at)?;
        if !["abstract", "text", "todo"].contains(&check.as_str()) {
            return Err(error("err.fragrules.badFormat", format!("{at}.check")));
        }
        out.push(Rif {
            key: text(r, "key", at)?,
            pmid: opzionale(r.get("pmid")),
            doi: opzionale(r.get("doi")),
            check,
        });
    }
    Ok(out)
}

fn rule(v: &Value) -> Result<Rule> {
    let id = text(v, "id", "rules")?;
    only(
        v,
        &[
            "id",
            "kind",
            "formula",
            "radical",
            "requires",
            "excludes",
            "ions",
            "polarity",
            "activation",
            "energy",
            "charge",
            "priority",
            "after",
            "refs",
            "why",
        ],
        &id,
    )?;
    let req = v.get("requires").unwrap_or(&Value::Null);
    if !req.is_null() {
        only(req, &["all_groups", "any_groups", "site", "smarts"], &id)?;
    }
    let tipo = match text(v, "kind", &id)?.as_str() {
        "loss" => Tipo::Loss,
        "ion" => Tipo::Ion,
        "cleavage" => Tipo::Cleavage,
        _ => return Err(error("err.fragrules.badFormat", format!("{id}.kind"))),
    };
    let f = parse_formula(&text(v, "formula", &id)?)
        .map_err(|_| error("err.fragrules.badFormat", format!("{id}.formula")))?;
    let attivazione = match text(v, "activation", &id)?.as_str() {
        "cid_trap" => Activation::CidTrap,
        "beam" => Activation::Beam,
        "both" => Activation::Both,
        _ => return Err(error("err.fragrules.badFormat", format!("{id}.activation"))),
    };
    let polarita = match text(v, "polarity", &id)?.as_str() {
        "+" => 1,
        "-" => -1,
        "both" => 0,
        _ => return Err(error("err.fragrules.badFormat", format!("{id}.polarity"))),
    };
    let priorita = v.get("priority").and_then(Value::f64).unwrap_or(0.0) as u8;
    if !(1..=3).contains(&priorita) {
        return Err(error("err.fragrules.badFormat", format!("{id}.priority")));
    }
    Ok(Rule {
        tipo,
        formula: fmt(&f),
        massa: mass(&f),
        radicale: v.get("radical").and_then(Value::bool).unwrap_or(false),
        all_groups: list(req, "all_groups", &id).unwrap_or_default(),
        any_groups: list(req, "any_groups", &id).unwrap_or_default(),
        site: opzionale(req.get("site")),
        excludes: list(v, "excludes", &id)?,
        ions: list(v, "ions", &id)?,
        polarita,
        activation: attivazione,
        energia: text(v, "energy", &id)?,
        load: text(v, "charge", &id)?,
        priorita,
        after: list(v, "after", &id)?,
        refs: references(v, &id)?,
        perche: text(v, "why", &id)?,
        id,
    })
}

fn globali(v: &Value) -> Result<Globali> {
    let mut g = Globali {
        thresholds: vec![],
        even_electron: true,
        trap_cutoff_q: 0.25,
        beam_composite: true,
        stepped_sum: true,
        loss_tolerance_mda: 3.0,
    };
    for x in v.arr() {
        only(
            x,
            &["id", "value", "refs", "why", "origine", "da_calibrare"],
            "globals",
        )?;
        let valore = x.get("value");
        let (num, flag) = (valore.and_then(Value::f64), valore.and_then(Value::bool));
        let id = text(x, "id", "globals")?;
        if let (true, Some(valore)) = (id == "trap_cutoff" || id == "loss_tolerance_mda", num) {
            g.thresholds.push(Threshold {
                value: valore,
                perche: text(x, "why", &id)?,
                origin: opzionale(x.get("origine")),
                to_calibrate: x.get("da_calibrare").and_then(Value::bool).unwrap_or(false),
                refs: references(x, &id)?,
                id: id.clone(),
            });
        }
        match id.as_str() {
            "even_electron" => g.even_electron = flag.unwrap_or(true),
            "trap_cutoff" => g.trap_cutoff_q = num.unwrap_or(g.trap_cutoff_q),
            "beam_composite" => g.beam_composite = flag.unwrap_or(true),
            "stepped_sum" => g.stepped_sum = flag.unwrap_or(true),
            "loss_tolerance_mda" => g.loss_tolerance_mda = num.unwrap_or(3.0),
            _ => return Err(error("err.fragrules.badKey", "globals.id")),
        }
    }
    Ok(g)
}

/// Reads a `fragrules/1` text: the format, the keys, the kinds and the formulas are all checked, a key that is not in the schema is an error.
pub fn read_table(text_json: &str) -> Result<Table> {
    let v = parse(text_json.as_bytes())?;
    only(&v, &["format", "rules", "globals"], "root")?;
    if v.get("format").and_then(Value::str) != Some("fragrules/1") {
        return Err(error("err.fragrules.badFormat", "format"));
    }
    let regole = v
        .get("rules")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(rule)
        .collect::<Result<Vec<_>>>()?;
    let mut ids: Vec<&str> = regole.iter().map(|r| r.id.as_str()).collect();
    crate::sortc::slice_ord(&mut ids);
    if ids.windows(2).any(|w| w[0] == w[1]) {
        return Err(error("err.fragrules.badFormat", "rules.id"));
    }
    Ok(Table {
        rules: regole,
        globali: globali(v.get("globals").unwrap_or(&Value::Null))?,
    })
}

/// The embedded table.
pub fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| read_table(TABLE).expect("the embedded fragrules table is valid"))
}
