//! Candidate elemental compositions of one feature from its spectrum: the isotope finder (`isofinder`) gives the charge, the observed pattern and the
//! heavy elements that cannot be ruled out; `mzlab_core::composition::compose` searches the formulas of the exact m/z and scores them against the pattern.
//! This layer only wires them and explains the choice: the program chooses (the candidate with the best score), the caller can always change it and sees why.
//! The confidence of a candidate is its share of the sum of the scores of the list, never an identification. Used by mzFinder (Tabui, administrator mode).

use mzlab_core::composition::{compose, parse_elements, Opts as ComposeOpts, Ranges};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Out, Value};

use super::isofinder::auto::Found;
use super::isofinder::select::{detect, Detection};
use super::isofinder::{Config, Engine};

/// Elements searched when the caller gives none (C H N O P S); Cl, Br, Si and the other heavy ones only enter when the spectrum shows them.
pub const DEFAULT_ELEMENTS: &str = "C:0-50,H:0-100,N:0-8,O:0-15,P:0-2,S:0-3";

/// Candidates within this score of the best one are alternatives.
const ALT_MARGIN: f64 = 0.15;

pub struct Opts {
    /// Ion type of the m/z (`[M+H]+`, `[M-H]-`, ...). Above charge 1 the formulas are those of the ion itself.
    pub ion: String,
    /// Charge: `None` lets the isotope finder decide (and 1 when it finds none).
    pub z: Option<i32>,
    pub tol_ppm: f64,
    /// Element ranges `C:0-50,H:0-100,...`; `None` = [`DEFAULT_ELEMENTS`] plus the heavy elements of the spectrum. The elements written here are searched as written.
    pub elements: Option<String>,
    /// Nitrogen rule of `compose`: `none`, `odd` or `even`.
    pub nitrogen: String,
    pub rdb_min: f64,
    pub max_results: usize,
    pub finder: Config,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            ion: "[M+H]+".into(),
            z: None,
            tol_ppm: 5.0,
            elements: None,
            nitrogen: "none".into(),
            rdb_min: 0.0,
            max_results: 10,
            finder: Config::default(),
        }
    }
}

impl Opts {
    /// Reads `{"ion","z","tol_ppm","elements","nitrogen","rdb_min","max_results"}` (all optional; an empty text is the defaults). A wrong type is an error.
    pub fn from_json(text: &str) -> Result<Opts> {
        let mut o = Opts::default();
        if text.trim().is_empty() {
            return Ok(o);
        }
        let v = json::parse(text.as_bytes())?;
        let bad = |k: &str| Error::new("err.formulaId.param").with("param", k);
        if let Some(x) = v.get("ion") {
            o.ion = x.str().ok_or_else(|| bad("ion"))?.to_string();
        }
        if let Some(x) = v.get("z").filter(|x| !x.is_null()) {
            o.z = Some(x.f64().ok_or_else(|| bad("z"))? as i32);
        }
        if let Some(x) = v.get("tol_ppm") {
            o.tol_ppm = x.f64().filter(|t| *t > 0.0).ok_or_else(|| bad("tol_ppm"))?;
        }
        if let Some(x) = v.get("elements").filter(|x| !x.is_null()) {
            o.elements = Some(x.str().ok_or_else(|| bad("elements"))?.to_string());
        }
        if let Some(x) = v.get("nitrogen") {
            o.nitrogen = x.str().ok_or_else(|| bad("nitrogen"))?.to_string();
        }
        if let Some(x) = v.get("rdb_min") {
            o.rdb_min = x.f64().ok_or_else(|| bad("rdb_min"))?;
        }
        if let Some(x) = v.get("max_results") {
            o.max_results = x
                .f64()
                .filter(|n| *n >= 1.0)
                .ok_or_else(|| bad("max_results"))?
                .min(100.0) as usize;
        }
        Ok(o)
    }
}

/// Where the charge comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChargeSource {
    Given,
    Finder,
    /// The finder found no pattern: the charge of the ion type.
    Assumed,
}

impl ChargeSource {
    fn name(self) -> &'static str {
        match self {
            ChargeSource::Given => "given",
            ChargeSource::Finder => "finder",
            ChargeSource::Assumed => "assumed",
        }
    }
}

/// One reason for the score of a candidate: a stable code and its number (for the interface to word).
#[derive(Clone, Debug, PartialEq)]
pub struct Reason {
    pub code: &'static str,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub formula: String,
    pub neutral: Option<String>,
    pub mz_calc: f64,
    pub error_ppm: f64,
    pub rdb: f64,
    /// Combined score of mass and isotope pattern (mass only without a pattern), 0-1.
    pub score: f64,
    /// Share of the sum of the scores of the list, 0-1.
    pub confidence: f64,
    pub mass_term: f64,
    pub iso_term: Option<f64>,
    pub rules_failed: Vec<String>,
    pub rules_ok: usize,
    pub why: Vec<Reason>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Identified {
    pub charge: i32,
    pub charge_source: ChargeSource,
    /// Probability of the charge among the charges that the finder scored (`None` when it did not decide).
    pub charge_probability: Option<f64>,
    pub observed: Vec<(f64, f64)>,
    pub elements_found: Vec<Found>,
    pub candidates: Vec<Candidate>,
    /// Index in `candidates` of the choice of the program; the caller can change it.
    pub chosen: Option<usize>,
    /// Indexes of the candidates within the margin of the best one (the chosen one excluded).
    pub alternatives: Vec<usize>,
}

/// Ranges of the search: the caller's text as written, or the default one plus the heavy elements of the spectrum (from 0 to their rough count + 1).
fn ranges(o: &Opts, found: &[Found]) -> Result<Ranges> {
    if let Some(t) = &o.elements {
        return parse_elements(t);
    }
    let mut r = parse_elements(DEFAULT_ELEMENTS)?;
    for f in found {
        let top = (f.atoms + 1).min(8);
        match r.iter_mut().find(|(s, _)| *s == f.symbol) {
            Some(x) => x.1 .1 = x.1 .1.max(top),
            None => r.push((f.symbol.clone(), (0, top))),
        }
    }
    Ok(r)
}

/// The charge and the pattern: the caller's charge, else the winner of the finder, else the charge of the ion type.
fn charge_of(o: &Opts, d: &Option<Detection>) -> (i32, ChargeSource, Option<f64>) {
    if let Some(z) = o.z.filter(|z| *z != 0) {
        return (z.abs(), ChargeSource::Given, None);
    }
    match d {
        Some(d) => {
            let p = d
                .probabilities
                .iter()
                .find(|p| p.0 == d.best_charge)
                .map(|p| p.1);
            (d.best_charge, ChargeSource::Finder, p)
        }
        None => (1, ChargeSource::Assumed, None),
    }
}

/// The observed peaks of the pattern of the charge `z`: those of the finder when its winner has that charge, else none.
fn pattern_for(d: &Option<Detection>, z: i32) -> Vec<(f64, f64)> {
    match d {
        Some(d) => d
            .patterns
            .iter()
            .find(|p| p.charge == z)
            .map(|p| p.peaks.clone())
            .unwrap_or_default(),
        None => Vec::new(),
    }
}

/// The mass term of `compose`: a Gaussian of the error in ppm (expected error 3 ppm).
fn mass_term(ppm: f64) -> f64 {
    round((-0.5 * (ppm / 3.0).powi(2)).exp(), 4)
}

/// Runs `compose` and reads its JSON `results` (formula, errors, rules, scores).
fn search(mz: f64, z: i32, obs: Vec<(f64, f64)>, r: &Ranges, o: &Opts) -> Result<Vec<Candidate>> {
    // a charge above 1 has no adduct atoms to name: the formulas are those of the ion
    let ion = if z > 1 { "[M]+" } else { o.ion.as_str() };
    let co = ComposeOpts {
        elements: Some(r.clone()),
        ion,
        z: Some(z),
        tol: o.tol_ppm,
        rdb: (o.rdb_min, 100.0),
        nitrogen: o.nitrogen.as_str(),
        max_results: o.max_results.max(1),
        obs,
        ..ComposeOpts::default()
    };
    let text = compose(mz, &co)?;
    let v = json::parse(text.as_bytes())?;
    let num = |x: &Value, k: &str| x.get(k).and_then(Value::f64);
    Ok(v.get("results")
        .map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .map(|x| Candidate {
            formula: x
                .get("formula")
                .and_then(Value::str)
                .unwrap_or("")
                .to_string(),
            neutral: x.get("neutral").and_then(Value::str).map(String::from),
            mz_calc: num(x, "mz_calc").unwrap_or(0.0),
            error_ppm: num(x, "delta_ppm").unwrap_or(0.0),
            rdb: num(x, "rdb").unwrap_or(0.0),
            score: x.get("score").and_then(|s| num(s, "total")).unwrap_or(0.0),
            confidence: 0.0,
            // without a pattern `compose` writes no score: the mass term is the Gaussian of the error of `compose` (expected error 3 ppm)
            mass_term: x
                .get("score")
                .and_then(|s| num(s, "mass"))
                .unwrap_or_else(|| mass_term(num(x, "delta_ppm").unwrap_or(0.0))),
            iso_term: x.get("score").and_then(|s| num(s, "iso")),
            rules_failed: x
                .get("rules_failed")
                .map(Value::arr)
                .unwrap_or(&[])
                .iter()
                .filter_map(|s| s.str().map(String::from))
                .collect(),
            rules_ok: num(x, "rules_ok").unwrap_or(0.0) as usize,
            why: Vec::new(),
        })
        .collect())
}

/// Confidence (share of the scores), the reasons and the choice. Without a pattern the score is 0 and the mass term ranks and weighs.
fn judge(cands: &mut [Candidate], with_pattern: bool) -> (Option<usize>, Vec<usize>) {
    let key = |c: &Candidate| if with_pattern { c.score } else { c.mass_term };
    cands.sort_by(|a, b| key(b).total_cmp(&key(a)));
    let sum: f64 = cands.iter().map(key).sum();
    for c in cands.iter_mut() {
        c.confidence = if sum > 0.0 { key(c) / sum } else { 0.0 };
        c.why = reasons(c, with_pattern);
    }
    let Some(best) = cands.first().map(key) else {
        return (None, Vec::new());
    };
    let alts = (1..cands.len())
        .filter(|i| key(&cands[*i]) >= best - ALT_MARGIN)
        .collect();
    (Some(0), alts)
}

fn reasons(c: &Candidate, with_pattern: bool) -> Vec<Reason> {
    let mut why = vec![
        Reason {
            code: "massError",
            value: c.error_ppm,
        },
        Reason {
            code: "rdb",
            value: c.rdb,
        },
    ];
    why.push(match with_pattern {
        true => Reason {
            code: "isotopeFit",
            value: c.iso_term.unwrap_or(0.0),
        },
        false => Reason {
            code: "massOnly",
            value: 0.0,
        },
    });
    why.push(Reason {
        code: "rulesOk",
        value: c.rules_ok as f64,
    });
    why.extend(c.rules_failed.iter().map(|_| Reason {
        code: "ruleBroken",
        value: 1.0,
    }));
    why
}

/// Candidate compositions of the feature at `feature_mz` (the m/z of its monoisotopic peak) in the spectrum `(mz, y)`.
pub fn identify(mz: &[f64], y: &[f64], feature_mz: f64, o: &Opts) -> Result<Identified> {
    let engine = Engine::new(o.finder.clone())
        .map_err(|e| Error::new("err.formulaId.finder").with("why", &e))?;
    let sign = if o.ion.ends_with('-') { -1 } else { 1 };
    let detection = detect(&engine, mz, y, feature_mz, sign);
    let (z, source, prob) = charge_of(o, &detection);
    let found = detection
        .as_ref()
        .map(|d| d.composition.clone())
        .unwrap_or_default();
    let observed = pattern_for(&detection, z);
    let r = ranges(o, &found)?;
    let mut candidates = search(feature_mz, z, observed.clone(), &r, o)?;
    let (chosen, alternatives) = judge(&mut candidates, !observed.is_empty());
    Ok(Identified {
        charge: z,
        charge_source: source,
        charge_probability: prob,
        observed,
        elements_found: found,
        candidates,
        chosen,
        alternatives,
    })
}

fn write_candidate(out: &mut Out, c: &Candidate) {
    out.raw("{\"formula\":")
        .str(&c.formula)
        .raw(",\"neutral\":");
    match &c.neutral {
        Some(n) => out.str(n),
        None => out.raw("null"),
    };
    out.raw(",\"mz_calc\":")
        .num(c.mz_calc)
        .raw(",\"error_ppm\":")
        .num(c.error_ppm)
        .raw(",\"rdb\":")
        .num(c.rdb);
    out.raw(",\"score\":")
        .num(round(c.score, 4))
        .raw(",\"confidence\":")
        .num(round(c.confidence, 4));
    out.raw(",\"terms\":{\"mass\":")
        .num(c.mass_term)
        .raw(",\"isotopes\":")
        .opt(c.iso_term)
        .raw("}");
    out.raw(",\"rules_ok\":")
        .int(c.rules_ok as i64)
        .raw(",\"rules_failed\":[");
    for (i, r) in c.rules_failed.iter().enumerate() {
        out.raw(if i > 0 { "," } else { "" }).str(r);
    }
    out.raw("],\"why\":[");
    for (i, w) in c.why.iter().enumerate() {
        out.raw(if i > 0 { "," } else { "" })
            .raw("{\"code\":")
            .str(w.code)
            .raw(",\"value\":")
            .num(w.value)
            .raw("}");
    }
    out.raw("]}");
}

/// The JSON `formula_id/1`: `{"v":"formula_id/1","charge":{value,source,probability},"observed":[[mz,int]...],"elements_found":[{symbol,atoms,confidence}],
/// "candidates":[...],"chosen":i|null,"alternatives":[i...]}`.
pub fn to_json(r: &Identified) -> String {
    let mut out = Out::default();
    out.raw("{\"v\":\"formula_id/1\",\"charge\":{\"value\":")
        .int(r.charge as i64);
    out.raw(",\"source\":")
        .str(r.charge_source.name())
        .raw(",\"probability\":")
        .opt(r.charge_probability)
        .raw("},\"observed\":[");
    for (i, p) in r.observed.iter().enumerate() {
        out.raw(if i > 0 { "," } else { "" })
            .raw("[")
            .num(p.0)
            .raw(",")
            .num(p.1)
            .raw("]");
    }
    out.raw("],\"elements_found\":[");
    for (i, f) in r.elements_found.iter().enumerate() {
        out.raw(if i > 0 { "," } else { "" })
            .raw("{\"symbol\":")
            .str(&f.symbol)
            .raw(",\"atoms\":")
            .int(f.atoms)
            .raw(",\"confidence\":")
            .num(round(f.confidence, 4))
            .raw("}");
    }
    out.raw("],\"candidates\":[");
    for (i, c) in r.candidates.iter().enumerate() {
        out.raw(if i > 0 { "," } else { "" });
        write_candidate(&mut out, c);
    }
    out.raw("],\"chosen\":");
    match r.chosen {
        Some(i) => out.int(i as i64),
        None => out.raw("null"),
    };
    out.raw(",\"alternatives\":[");
    for (i, a) in r.alternatives.iter().enumerate() {
        out.raw(if i > 0 { "," } else { "" }).int(*a as i64);
    }
    out.raw("]}");
    out.0
}
