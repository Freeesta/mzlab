//! The photocatalytic rules as data: `TP_Mine/data/regole_tp_2.json`, format `regole_tp/2` (profiles, mechanistic classes, preconditions on the functional
//! groups of the parent: `docs/schemi/regole_tp-2.json`, `docs/agenti/mzfeat.md`); a `regole_tp/1` file (one flat list, elements only) is still read.
//! A rule is a signed change of formula; the lists can be replaced without touching the code.

use std::collections::BTreeMap;

use mzlab_core::elements::parse_formula;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Value};

pub const FORMAT: &str = "regole_tp";
/// Newest version read (`regole_tp/2`); a `/1` file is read too and says so in `Rules::version`.
pub const VERSION: u32 = 2;

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
    /// Mechanistic class (`/2`; the id in a `/1` file).
    pub class: String,
    /// Functional groups the parent must have (all of `groups_all`, one of `groups_any`); a rule with groups is used only when the structure is known.
    pub groups_all: Vec<String>,
    pub groups_any: Vec<String>,
    /// A rule for when the structure is not known: left out when it is (the rules with groups say it better).
    pub fallback: bool,
}

#[derive(Debug, Clone)]
pub struct Rules {
    pub rules: Vec<Rule>,
    /// Version of the file read (1 or 2).
    pub version: u32,
    /// Profile of the rules (empty in a `/1` file).
    pub profile: String,
}

fn bad(what: &str) -> Error {
    Error::new("error.hypo.rulesFormat").with("what", what)
}

/// '+O', '-H2', '-Cl+H', '+C2H4O' -> signed counts (as the `parse_delta` of mzFinder).
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
    if source != "transformation_table"
        && !source.starts_with("lit:")
        && !source.starts_with("classe:")
    {
        return Err(bad("fonte"));
    }
    let num = |k: &str| {
        v.get(k)
            .and_then(Value::f64)
            .map(|x| x as u8)
            .ok_or_else(|| bad(k))
    };
    let need = v.get("richiede");
    let list = |k: &str| -> Vec<String> {
        need.and_then(|n| n.get(k))
            .map(|a| {
                a.arr()
                    .iter()
                    .filter_map(|x| x.str().map(String::from))
                    .collect()
            })
            // fixed size: a rule text, empty when absent
            .unwrap_or_default()
    };
    Ok(Rule {
        id: id.clone(),
        delta,
        delta_text,
        max_steps: num("passi_max")?.max(1),
        level: num("livello_previsione")?,
        source: source.to_string(),
        dir: dir_of(text("direzione_rt")?)?,
        all: counts_of(need.and_then(|n| n.get("tutti")))?,
        any: list("almeno_uno"),
        class: text("classe").map_or_else(|_| id.clone(), String::from),
        groups_all: list("gruppi"),
        groups_any: list("gruppi_almeno_uno"),
        fallback: v.get("fallback").and_then(Value::bool).unwrap_or(false),
    })
}

fn rules_of(v: &Value, key: &str) -> Result<Vec<Rule>> {
    v.get(key)
        .map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .map(rule_of)
        .collect()
}

impl Rules {
    /// Reads a `regole_tp` file with its default profile; another format or a newer version is refused with an i18n error.
    pub fn parse(bytes: &[u8]) -> Result<Rules> {
        Self::parse_profile(bytes, None)
    }

    /// The same with a profile of a `/2` file (`None` = the default one). A profile that is frozen (no data yet) or missing is refused; a `/1` file has
    /// no profiles. A profile can `eredita` (inherit) the rules of another.
    pub fn parse_profile(bytes: &[u8], profile: Option<&str>) -> Result<Rules> {
        let v = json::parse(bytes).map_err(|_| bad("json"))?;
        if v.get("format").and_then(Value::str) != Some(FORMAT) {
            return Err(bad("format"));
        }
        let found = v.get("version").and_then(Value::f64).unwrap_or(0.0) as u32;
        if found == 1 && profile.is_none() {
            return Ok(Rules {
                rules: rules_of(&v, "regole")?,
                version: 1,
                profile: String::new(),
            });
        }
        if found != VERSION {
            return Err(Error::new("error.hypo.rulesVersion")
                .with("found", found)
                .with("need", VERSION));
        }
        let name = profile
            .or_else(|| v.get("predefinito").and_then(Value::str))
            .unwrap_or("");
        let prof = v
            .get("profili")
            .and_then(|p| p.get(name))
            .ok_or_else(|| Error::new("error.hypo.profile").with("profile", name))?;
        if prof.get("congelato").and_then(Value::bool) == Some(true) {
            return Err(Error::new("error.hypo.profileFrozen").with("profile", name));
        }
        let mut rules = Vec::new();
        if let Some(base) = prof.get("eredita").and_then(Value::str) {
            let b = v
                .get("profili")
                .and_then(|p| p.get(base))
                .ok_or_else(|| Error::new("error.hypo.profile").with("profile", base))?;
            rules = rules_of(b, "regole")?;
        }
        rules.extend(rules_of(prof, "regole")?);
        Ok(Rules {
            rules,
            version: found,
            profile: name.to_string(),
        })
    }

    /// Does the parent have what the class rule needs? Its elements, and its functional groups when the structure is known (`groups`): a rule with
    /// groups is used only then, a `fallback` rule only without.
    pub fn applies(&self, r: &Rule, parent: &Counts, groups: Option<&[String]>) -> bool {
        let n = |e: &str| parent.get(e).copied().unwrap_or(0);
        let has = |g: &String| groups.is_some_and(|l| l.contains(g));
        let by_structure = !r.groups_all.is_empty() || !r.groups_any.is_empty();
        if (groups.is_some() && r.fallback) || (groups.is_none() && by_structure) {
            return false;
        }
        r.all.iter().all(|(e, k)| n(e) >= *k)
            && (r.any.is_empty() || r.any.iter().any(|e| n(e) > 0))
            && r.groups_all.iter().all(has)
            && (r.groups_any.is_empty() || r.groups_any.iter().any(has))
    }
}
