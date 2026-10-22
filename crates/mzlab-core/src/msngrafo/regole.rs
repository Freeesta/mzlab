//! The table of fragmentation rules `fragrules/1` (`docs/studi/regole-frammentazione-schema.md`, schema `docs/schemi/fragrules-1.json`): the strict reader
//! and the embedded table `data/fragrules-1.json`. The evaluation on the graph is `regole_valuta`; the structure conditions (`requires`) are read here and
//! evaluated only when a structure is given (a later step). A rule is evidence with a source, never a verdict.

use std::sync::OnceLock;

use crate::elements::{fmt, mass, parse_formula};
use crate::error::{Error, Result};
use crate::json::{parse, Value};

/// The table shipped with the program.
const TABELLA: &str = include_str!("../../data/fragrules-1.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    Loss,
    Ion,
    Cleavage,
}

/// Which activation a rule belongs to: the ion trap (`cid_trap`), the beam type (`beam`: HCD and collision-cell CID) or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attivazione {
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
pub struct Regola {
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
    pub ioni: Vec<String>,
    /// 1 (`+`), -1 (`-`) or 0 (both).
    pub polarita: i8,
    pub attivazione: Attivazione,
    pub energia: String,
    pub carica: String,
    pub priorita: u8,
    pub after: Vec<String>,
    pub refs: Vec<Rif>,
    pub perche: String,
}

impl Regola {
    /// The rule has a condition on the structure (groups, excluded groups or a site): without a structure it is not decided.
    pub fn legge_struttura(&self) -> bool {
        !self.all_groups.is_empty()
            || !self.any_groups.is_empty()
            || self.site.is_some()
            || !self.excludes.is_empty()
            || self.tipo == Tipo::Cleavage
    }

    /// The rule needs no structure: only the formula, the adduct and the polarity decide.
    pub fn solo_formula(&self) -> bool {
        self.all_groups.is_empty()
            && self.any_groups.is_empty()
            && self.site.is_none()
            && self.excludes.is_empty()
            && self.after.is_empty()
    }
}

/// The global rules with their values (the defaults of the study when the table omits one).
#[derive(Clone, Debug, PartialEq)]
pub struct Globali {
    pub even_electron: bool,
    pub trap_cutoff_q: f64,
    pub beam_composite: bool,
    pub stepped_sum: bool,
    pub loss_tolerance_mda: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tabella {
    pub regole: Vec<Regola>,
    pub globali: Globali,
}

fn errore(chiave: &'static str, dettaglio: impl ToString) -> Error {
    Error::new(chiave).with("detail", dettaglio)
}

fn solo(v: &Value, ammesse: &[&str], dove: &str) -> Result<()> {
    if let Value::Obj(o) = v {
        if let Some((k, _)) = o.iter().find(|(k, _)| !ammesse.contains(&k.as_str())) {
            return Err(errore("err.fragrules.badKey", format!("{dove}.{k}")));
        }
        return Ok(());
    }
    Err(errore("err.fragrules.badFormat", dove))
}

fn testo(v: &Value, k: &str, dove: &str) -> Result<String> {
    let s = v.get(k).and_then(Value::str);
    s.map(str::to_string)
        .ok_or_else(|| errore("err.fragrules.badFormat", format!("{dove}.{k}")))
}

fn lista(v: &Value, k: &str, dove: &str) -> Result<Vec<String>> {
    let a = v
        .get(k)
        .ok_or_else(|| errore("err.fragrules.badFormat", format!("{dove}.{k}")))?;
    a.arr()
        .iter()
        .map(|x| {
            x.str()
                .map(str::to_string)
                .ok_or_else(|| errore("err.fragrules.badFormat", format!("{dove}.{k}")))
        })
        .collect()
}

fn opzionale(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::str).map(str::to_string)
}

fn riferimenti(v: &Value, dove: &str) -> Result<Vec<Rif>> {
    let mut out = vec![];
    for r in v.get("refs").map_or(&[][..], Value::arr) {
        solo(r, &["key", "pmid", "doi", "check"], dove)?;
        let check = testo(r, "check", dove)?;
        if !["abstract", "text", "todo"].contains(&check.as_str()) {
            return Err(errore("err.fragrules.badFormat", format!("{dove}.check")));
        }
        out.push(Rif {
            key: testo(r, "key", dove)?,
            pmid: opzionale(r.get("pmid")),
            doi: opzionale(r.get("doi")),
            check,
        });
    }
    Ok(out)
}

fn regola(v: &Value) -> Result<Regola> {
    let id = testo(v, "id", "rules")?;
    solo(
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
        solo(req, &["all_groups", "any_groups", "site", "smarts"], &id)?;
    }
    let tipo = match testo(v, "kind", &id)?.as_str() {
        "loss" => Tipo::Loss,
        "ion" => Tipo::Ion,
        "cleavage" => Tipo::Cleavage,
        _ => return Err(errore("err.fragrules.badFormat", format!("{id}.kind"))),
    };
    let f = parse_formula(&testo(v, "formula", &id)?)
        .map_err(|_| errore("err.fragrules.badFormat", format!("{id}.formula")))?;
    let attivazione = match testo(v, "activation", &id)?.as_str() {
        "cid_trap" => Attivazione::CidTrap,
        "beam" => Attivazione::Beam,
        "both" => Attivazione::Both,
        _ => {
            return Err(errore(
                "err.fragrules.badFormat",
                format!("{id}.activation"),
            ))
        }
    };
    let polarita = match testo(v, "polarity", &id)?.as_str() {
        "+" => 1,
        "-" => -1,
        "both" => 0,
        _ => return Err(errore("err.fragrules.badFormat", format!("{id}.polarity"))),
    };
    let priorita = v.get("priority").and_then(Value::f64).unwrap_or(0.0) as u8;
    if !(1..=3).contains(&priorita) {
        return Err(errore("err.fragrules.badFormat", format!("{id}.priority")));
    }
    Ok(Regola {
        tipo,
        formula: fmt(&f),
        massa: mass(&f),
        radicale: v.get("radical").and_then(Value::bool).unwrap_or(false),
        all_groups: lista(req, "all_groups", &id).unwrap_or_default(),
        any_groups: lista(req, "any_groups", &id).unwrap_or_default(),
        site: opzionale(req.get("site")),
        excludes: lista(v, "excludes", &id)?,
        ioni: lista(v, "ions", &id)?,
        polarita,
        attivazione,
        energia: testo(v, "energy", &id)?,
        carica: testo(v, "charge", &id)?,
        priorita,
        after: lista(v, "after", &id)?,
        refs: riferimenti(v, &id)?,
        perche: testo(v, "why", &id)?,
        id,
    })
}

fn globali(v: &Value) -> Result<Globali> {
    let mut g = Globali {
        even_electron: true,
        trap_cutoff_q: 0.25,
        beam_composite: true,
        stepped_sum: true,
        loss_tolerance_mda: 3.0,
    };
    for x in v.arr() {
        solo(x, &["id", "value", "refs", "why"], "globals")?;
        let valore = x.get("value");
        let (num, flag) = (valore.and_then(Value::f64), valore.and_then(Value::bool));
        match testo(x, "id", "globals")?.as_str() {
            "even_electron" => g.even_electron = flag.unwrap_or(true),
            "trap_cutoff" => g.trap_cutoff_q = num.unwrap_or(g.trap_cutoff_q),
            "beam_composite" => g.beam_composite = flag.unwrap_or(true),
            "stepped_sum" => g.stepped_sum = flag.unwrap_or(true),
            "loss_tolerance_mda" => g.loss_tolerance_mda = num.unwrap_or(3.0),
            _ => return Err(errore("err.fragrules.badKey", "globals.id")),
        }
    }
    Ok(g)
}

/// Reads a `fragrules/1` text: the format, the keys, the kinds and the formulas are all checked, a key that is not in the schema is an error.
pub fn leggi_tabella(testo_json: &str) -> Result<Tabella> {
    let v = parse(testo_json.as_bytes())?;
    solo(&v, &["format", "rules", "globals"], "root")?;
    if v.get("format").and_then(Value::str) != Some("fragrules/1") {
        return Err(errore("err.fragrules.badFormat", "format"));
    }
    let regole = v
        .get("rules")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(regola)
        .collect::<Result<Vec<_>>>()?;
    let mut ids: Vec<&str> = regole.iter().map(|r| r.id.as_str()).collect();
    crate::sortc::slice_ord(&mut ids);
    if ids.windows(2).any(|w| w[0] == w[1]) {
        return Err(errore("err.fragrules.badFormat", "rules.id"));
    }
    Ok(Tabella {
        regole,
        globali: globali(v.get("globals").unwrap_or(&Value::Null))?,
    })
}

/// The embedded table.
pub fn tabella() -> &'static Tabella {
    static T: OnceLock<Tabella> = OnceLock::new();
    T.get_or_init(|| leggi_tabella(TABELLA).expect("the embedded fragrules table is valid"))
}
