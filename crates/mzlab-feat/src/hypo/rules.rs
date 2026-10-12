//! The photocatalytic rules as data: `TP_Mine/data/regole_tp.json`, format `regole_tp/1` (documented in `docs/agenti/mzfeat.md`).
//! A rule is a signed change of formula; the list is the one of mzFinder (`transformation_table`) and can be replaced without touching the code.

use std::collections::BTreeMap;

use mzlab_core::elements::parse_formula;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Value};

pub const FORMAT: &str = "regole_tp";
pub const VERSION: u32 = 1;

/// Signed element counts (a change of formula, or a formula), ordered by element symbol.
pub type Counts = BTreeMap<String, i64>;

/// Where the product elutes against the parent (reversed phase).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    MorePolar,
    LessPolar,
    Free,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub delta: Counts,
    pub delta_text: String,
    pub max_steps: u8,
    /// Prediction level of the rule: 2 literature, 3 class, 4 generic.
    pub level: u8,
    pub source: String,
    pub dir: Dir,
    /// Elements that the parent must have: all of `all` (with at least that count), at least one of `any`.
    pub all: Counts,
    pub any: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Rules {
    pub rules: Vec<Rule>,
}

fn bad(what: &str) -> Error {
    Error::new("error.hypo.rulesFormat").with("what", what)
}

/// '+O', '-H2', '-Cl+H', '+C2H4O' -> signed counts (as `tpmine.chem.parse_delta`).
pub fn parse_delta(text: &str) -> Result<Counts> {
    let t: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let t = if t.starts_with(['+', '-']) {
        t
    } else {
        format!("+{t}")
    };
    let mut out = Counts::new();
    let mut sign = 1i64;
    let mut body = String::new();
    let flush = |sign: i64, body: &mut String, out: &mut Counts| -> Result<()> {
        if body.is_empty() {
            return Ok(());
        }
        for (el, n) in parse_formula(body)? {
            *out.entry(el).or_insert(0) += sign * n as i64;
        }
        body.clear();
        Ok(())
    };
    for c in t.chars() {
        if c == '+' || c == '-' {
            flush(sign, &mut body, &mut out)?;
            sign = if c == '+' { 1 } else { -1 };
        } else {
            body.push(c);
        }
    }
    flush(sign, &mut body, &mut out)?;
    out.retain(|_, n| *n != 0);
    Ok(out)
}

fn dir_of(s: &str) -> Result<Dir> {
    match s {
        "piu_polare" => Ok(Dir::MorePolar),
        "meno_polare" => Ok(Dir::LessPolar),
        "libera" => Ok(Dir::Free),
        _ => Err(bad("direzione_rt")),
    }
}

fn counts_of(v: Option<&Value>) -> Result<Counts> {
    let mut out = Counts::new();
    if let Some(Value::Obj(o)) = v {
        for (k, n) in o {
            out.insert(k.clone(), n.f64().ok_or_else(|| bad("richiede"))? as i64);
        }
    }
    Ok(out)
}

fn rule_of(v: &Value) -> Result<Rule> {
    let text = |k: &str| v.get(k).and_then(Value::str).ok_or_else(|| bad(k));
    let id = text("id")?.to_string();
    let delta_text = text("delta_formula")?.to_string();
    let delta =
        parse_delta(&delta_text).map_err(|_| Error::new("error.hypo.rule").with("id", &id))?;
    let source = text("fonte")?;
    if source != "transformation_table" && !source.starts_with("lit:") {
        return Err(bad("fonte"));
    }
    let num = |k: &str| {
        v.get(k)
            .and_then(Value::f64)
            .map(|x| x as u8)
            .ok_or_else(|| bad(k))
    };
    let need = v.get("richiede");
    let any = need
        .and_then(|n| n.get("almeno_uno"))
        .map(|a| {
            a.arr()
                .iter()
                .filter_map(|x| x.str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(Rule {
        id,
        delta,
        delta_text,
        max_steps: num("passi_max")?.max(1),
        level: num("livello_previsione")?,
        source: source.to_string(),
        dir: dir_of(text("direzione_rt")?)?,
        all: counts_of(need.and_then(|n| n.get("tutti")))?,
        any,
    })
}

impl Rules {
    /// Reads a `regole_tp/1` file; another format or a newer version is refused with an i18n error.
    pub fn parse(bytes: &[u8]) -> Result<Rules> {
        let v = json::parse(bytes).map_err(|_| bad("json"))?;
        if v.get("format").and_then(Value::str) != Some(FORMAT) {
            return Err(bad("format"));
        }
        let found = v.get("version").and_then(Value::f64).unwrap_or(0.0) as u32;
        if found != VERSION {
            return Err(Error::new("error.hypo.rulesVersion")
                .with("found", found)
                .with("need", VERSION));
        }
        let rules = v.get("regole").map(Value::arr).unwrap_or(&[]);
        Ok(Rules {
            rules: rules.iter().map(rule_of).collect::<Result<_>>()?,
        })
    }

    /// Does the parent have what the class rule needs?
    pub fn applies(&self, r: &Rule, parent: &Counts) -> bool {
        let n = |e: &str| parent.get(e).copied().unwrap_or(0);
        r.all.iter().all(|(e, k)| n(e) >= *k)
            && (r.any.is_empty() || r.any.iter().any(|e| n(e) > 0))
    }
}
