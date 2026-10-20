//! The native executor of `annot.level@1`: the identification level of each query from its library hits (`candidates/1`) and its candidate formulas
//! (`formulas/1`). The rule is `mzlab/web/livelli.js` (`assess`) read from the same data (`mzlab-feat/data/livelli-1.json`, profile `mzlab`), so the
//! thresholds are never typed twice; `tests/livelli_casi.json` runs the same cases through both. Level 2b is never given: the chemist decides it.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Out, Value};

const DATA: &str = include_str!("../../mzlab-feat/data/livelli-1.json");

/// A hit of a library on one query: the fields of `assess` (`ent` and `cos` are scores of different algorithms and are never mixed).
#[derive(Clone, Debug, Default)]
pub struct Cand {
    pub name: String,
    pub ik: String,
    pub ent: Option<f64>,
    pub cos: Option<f64>,
    pub shared: Option<f64>,
    pub dprec: Option<f64>,
    pub nlib: Option<f64>,
}

/// A modified-cosine hit with another precursor mass: a structural analogue.
#[derive(Clone, Debug, Default)]
pub struct Analog {
    pub name: String,
    pub mcos: f64,
    pub delta: f64,
}

/// What is known of one query.
#[derive(Clone, Debug, Default)]
pub struct Evidence {
    pub cands: Vec<Cand>,
    pub analogs: Vec<Analog>,
    pub n_scans: f64,
    pub formula_unique: bool,
}

/// A gate of the data file with its outcome.
#[derive(Clone, Debug)]
pub struct GateResult {
    pub id: String,
    pub role: String,
    pub value: Option<f64>,
    pub threshold: f64,
    pub ok: bool,
}

/// The level proposed, why, and the gates of the best hit.
#[derive(Clone, Debug)]
pub struct Assessment {
    pub level: String,
    pub why: &'static str,
    pub best: Option<Cand>,
    pub analog: Option<Analog>,
    pub gates: Vec<GateResult>,
}

fn profile(name: &str) -> Result<Value> {
    json::parse(DATA.as_bytes())?
        .get("profili")
        .and_then(|p| p.get(name))
        .cloned()
        .ok_or_else(|| Error::new("err.flow.level.profile").with("profile", name))
}

fn field(c: &Cand, campo: &str) -> Option<f64> {
    match campo {
        "ent" => c.ent,
        "cos" => c.cos,
        "shared" => c.shared,
        "ppm" => c.dprec,
        "nlib" => c.nlib,
        _ => None,
    }
}

/// The first block of the InChIKey, else the name without punctuation: two hits with the same key are one structure.
fn key(c: &Cand) -> String {
    let k: String = c.ik.chars().take(14).collect::<String>().to_uppercase();
    if !k.is_empty() {
        return k;
    }
    let n: String = c
        .name
        .to_lowercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    format!("n:{n}")
}

/// Lead of the first hit over the first one of another structure (infinite when there is none).
fn margin(list: &[Cand], campo: &str) -> Option<f64> {
    let first = list.first()?;
    let k = key(first);
    Some(match list.iter().find(|c| key(c) != k) {
        Some(o) => field(first, campo)? - field(o, campo)?,
        None => f64::INFINITY,
    })
}

fn threshold(p: &Value, name: &str) -> f64 {
    p.get("soglie")
        .and_then(|s| s.get(name))
        .and_then(Value::f64)
        .unwrap_or(f64::NAN)
}

fn gate(p: &Value, g: &Value, vals: &dyn Fn(&str) -> Option<f64>) -> GateResult {
    let text = |k: &str| g.get(k).and_then(Value::str).unwrap_or("");
    let mut thr = threshold(p, text("soglia"));
    if let Some(r) = g.get("ridotta") {
        let below = r.get("minore_di").and_then(Value::f64).unwrap_or(0.0);
        let field = r.get("se_campo").and_then(Value::str).unwrap_or("");
        if vals(field).is_some_and(|v| v < below) {
            thr = threshold(p, r.get("soglia").and_then(Value::str).unwrap_or(""));
        }
    }
    let value = vals(text("campo")).filter(|v| !v.is_nan());
    let seen = value.map(|v| {
        if g.get("assoluto").and_then(Value::bool) == Some(true) {
            v.abs()
        } else {
            v
        }
    });
    let ok = seen.is_some_and(|v| {
        if text("op") == "<=" {
            v <= thr
        } else {
            v >= thr
        }
    });
    GateResult {
        id: text("id").to_string(),
        role: text("ruolo").to_string(),
        value: seen,
        threshold: thr,
        ok,
    }
}

/// The level that the evidence of one query reaches in `profile` (`mzlab`, or `lr` for unit resolution).
pub fn assess(e: &Evidence, name: &str) -> Result<Assessment> {
    let p = profile(name)?;
    let score = p
        .get("cancelli")
        .map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .find(|g| g.get("id").and_then(Value::str) == Some("score"))
        .and_then(|g| g.get("campo").and_then(Value::str))
        .unwrap_or("ent")
        .to_string();
    let mut list: Vec<Cand> = e
        .cands
        .iter()
        .filter(|c| field(c, &score).is_some())
        .cloned()
        .collect();
    list.sort_by(|a, b| cmp_opt(field(b, &score), field(a, &score)));
    let without = p
        .get("senza_prove")
        .and_then(Value::str)
        .unwrap_or("0")
        .to_string();
    let mut res = Assessment {
        level: without.clone(),
        why: "none",
        best: list.first().cloned(),
        analog: None,
        gates: Vec::new(),
    };
    if by_best(&p, e, &list, &score, &mut res) {
        return Ok(res);
    }
    let min = threshold(&p, "analogo_mcos_min");
    let top = e.analogs.iter().fold(None, |t: Option<&Analog>, a| {
        t.filter(|t| t.mcos >= a.mcos).or(Some(a))
    });
    if let Some(a) = top.filter(|a| a.mcos >= min) {
        res.level = "3b".into();
        res.why = "analogo";
        res.analog = Some(a.clone());
        return Ok(res);
    }
    if e.formula_unique && without == "5" {
        res.level = "4".into();
        res.why = "formula";
        return Ok(res);
    }
    res.why = if res.best.is_some() { "none" } else { "empty" };
    Ok(res)
}

/// The level the best library hit reaches on its own: true when it decides (2a, the cap, 3a), with the gates in `res`.
fn by_best(p: &Value, e: &Evidence, list: &[Cand], score: &str, res: &mut Assessment) -> bool {
    let Some(best) = list.first() else {
        return false;
    };
    let lead = margin(list, score);
    let vals = |f: &str| match f {
        "nScans" => Some(e.n_scans),
        "margin" => lead,
        f => field(best, f),
    };
    res.gates = p
        .get("cancelli")
        .map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .map(|g| gate(p, g, &vals))
        .collect();
    let bad: Vec<&GateResult> = res.gates.iter().filter(|g| !g.ok).collect();
    if bad.is_empty() {
        let cap = p.get("tetto").and_then(Value::str);
        res.level = cap.unwrap_or("2a").to_string();
        res.why = if cap.is_some() { "tetto" } else { "2a" };
        return true;
    }
    if !bad.iter().any(|g| g.role == "source") {
        res.level = "3a".into();
        res.why = "confirm";
        return true;
    }
    let prov = p.get("provvisorio");
    let from = prov.and_then(|v| v.get("campo")).and_then(Value::str);
    let at = prov
        .and_then(|v| v.get("soglia"))
        .and_then(Value::str)
        .map_or(f64::NAN, |s| threshold(p, s));
    let provisional = from.and_then(|f| field(best, f)).is_some_and(|v| v >= at);
    if provisional {
        res.level = "3a".into();
        res.why = "provvisorio";
    }
    provisional
}

fn text(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::str).unwrap_or("").to_string()
}

/// The evidence of every query named by a `candidates/1` file, by query id in file order; replicate spectra whose best hit is the same entry count as scans.
fn from_hits(files: &[&[u8]]) -> Result<Vec<(String, Evidence)>> {
    let mut all: Vec<(String, Evidence)> = Vec::new();
    for f in files {
        let doc = json::parse(f)?;
        for h in doc.get("hits").map(Value::arr).unwrap_or(&[]) {
            let id = text(h, "query");
            let n = match all.iter().position(|(q, _)| *q == id) {
                Some(n) => n,
                None => {
                    all.push((id, Evidence::default()));
                    all.len() - 1
                }
            };
            let get = |k: &str| h.get(k).and_then(Value::f64);
            let algorithm = text(h, "algorithm");
            let delta = get("precursor_delta").unwrap_or(0.0);
            let name = text(h, "name");
            if algorithm == "modified_cosine" && delta != 0.0 {
                all[n].1.analogs.push(Analog {
                    name,
                    mcos: get("score").unwrap_or(0.0),
                    delta,
                });
                continue;
            }
            let score = get("score");
            all[n].1.cands.push(Cand {
                name,
                ik: text(h, "inchikey"),
                ent: score.filter(|_| algorithm == "spectral_entropy"),
                cos: score.filter(|_| algorithm != "spectral_entropy"),
                shared: get("shared"),
                dprec: get("mass_ppm"),
                nlib: get("library_peaks"),
            });
        }
    }
    Ok(all)
}

/// How many candidate formulas each feature or spectrum has in a `formulas/1` file.
fn formula_counts(files: &[&[u8]]) -> Result<Vec<(String, usize)>> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for f in files {
        let doc = json::parse(f)?;
        let rows = doc
            .get("features")
            .or_else(|| doc.get("spectra"))
            .map(Value::arr)
            .unwrap_or(&[]);
        for r in rows {
            let id = if r.get("query").is_some() {
                text(r, "query")
            } else {
                text(r, "id")
            };
            let list = r.get("result").unwrap_or(r).get("candidates");
            let n = list.map_or(0, |c| c.arr().len());
            out.retain(|(q, _)| *q != id);
            out.push((id, n));
        }
    }
    Ok(out)
}

fn scans_of(rows: &[(String, Evidence)], e: &Evidence) -> f64 {
    let key_of = |e: &Evidence| {
        e.cands
            .iter()
            .filter(|c| c.ent.is_some())
            .max_by(|a, b| cmp_opt(a.ent, b.ent))
            .map(key)
    };
    let mine = key_of(e);
    rows.iter()
        .filter(|(_, o)| mine.is_some() && key_of(o) == mine)
        .count()
        .max(1) as f64
}

/// `annot.level`: one row per query of the hits and per feature or spectrum of the formulas, with the level, why and the gates of the best hit.
pub fn level_text(hits: &[&[u8]], formulas: &[&[u8]]) -> Result<String> {
    let mut rows = from_hits(hits)?;
    let counts = formula_counts(formulas)?;
    for (id, _) in &counts {
        if !rows.iter().any(|(q, _)| q == id) {
            rows.push((id.clone(), Evidence::default()));
        }
    }
    let snapshot = rows.clone();
    let mut out = Out::default();
    out.raw("{\"format\":\"annot/1\",\"step\":\"annot.level@1\",\"algorithm\":\"cascade\",\"profile\":\"mzlab\",\"rows\":[");
    for (k, (id, e)) in rows.iter_mut().enumerate() {
        e.n_scans = scans_of(&snapshot, e);
        e.formula_unique = counts.iter().any(|(q, n)| q == id && *n == 1);
        let a = assess(e, "mzlab")?;
        out.raw(if k > 0 { "," } else { "" })
            .raw("{\"id\":")
            .str(id)
            .raw(",\"level\":")
            .str(&a.level)
            .raw(",\"why\":")
            .str(a.why);
        if let Some(b) = &a.best {
            out.raw(",\"name\":")
                .str(&b.name)
                .raw(",\"entry\":")
                .str(&b.ik);
        }
        if let Some(n) = &a.analog {
            out.raw(",\"analog\":")
                .str(&n.name)
                .raw(",\"analog_delta\":")
                .num(round(n.delta, 4));
        }
        out.raw(",\"gates\":[");
        for (i, g) in a.gates.iter().enumerate() {
            out.raw(if i > 0 { "," } else { "" })
                .raw("{\"id\":")
                .str(&g.id)
                .raw(",\"ok\":")
                .raw(if g.ok { "true" } else { "false" })
                .raw(",\"value\":")
                .opt(g.value.filter(|v| v.is_finite()).map(|v| round(v, 4)))
                .raw(",\"threshold\":")
                .num(round(g.threshold, 4))
                .raw("}");
        }
        out.raw("]}");
    }
    out.raw("]}");
    Ok(out.0)
}

fn cmp_opt(a: Option<f64>, b: Option<f64>) -> std::cmp::Ordering {
    a.unwrap_or(f64::NAN).total_cmp(&b.unwrap_or(f64::NAN))
}
