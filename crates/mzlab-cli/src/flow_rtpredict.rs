//! The native executor of `rt.predict@1`: the candidates (`candidates/1`) checked against the retention times of the campaign's own standards, with our own
//! Crippen descriptors (`logp_window`) or a regression forest (`forest`). Written beside each hit: the predicted RT, the residual and, with a window, `rt_ok`;
//! optionally the direction of the RT change against a parent. Evidence only: a hit is never added, a level never set.

use crate::flow::{all_inputs, read_one, write_made, Exec, FileRunner};
use crate::flow_match::num;
use crate::flow_read::load;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::{self, round, Out, Value};
use mzlab_core::step::DataType;
use mzlab_feat::attributio::ms2_engine::rtpredict::RtModel;
use mzlab_feat::attributio::review::rtlogp::{
    calibrate, check, check_terms, CalPoint, LogPValues, MIN_N,
};
use mzlab_feat::hypo::Dir;
use std::collections::HashMap;

/// Keys this step writes; a hit that already has one gets it replaced.
const OWN: [&str; 7] = [
    "rt_pred",
    "rt_resid",
    "rt_ok",
    "score_terms",
    "delta_logp",
    "rt_dir",
    "rt_dir_agrees",
];

/// The executor of the step, read by the runner after its own list.
pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("rt.predict", Exec::File("candidates", rtpredict_job))];

fn bad(key: &'static str) -> Error {
    Error::new(key)
}

fn param<'a>(job: &'a Job, name: &str) -> Option<&'a Value> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .map(|p| &p.1)
}

/// The rows of a CSV (`,`, `;` or tab) as two columns: the ones named `a` and `b` in the header, else the first two. A first row whose second field is not a
/// number is a header.
fn pairs(text: &str, a: &str, b: &str) -> Vec<(String, f64)> {
    let mut rows = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let Some(first) = rows.next() else {
        return Vec::new();
    };
    let sep = [';', '\t']
        .into_iter()
        .find(|c| first.contains(*c))
        .unwrap_or(',');
    let cells = |l: &str| {
        l.split(sep)
            .map(|c| c.trim().trim_matches('"').to_string())
            .collect::<Vec<_>>()
    };
    let head = cells(first);
    let header = head.get(1).is_none_or(|c| c.parse::<f64>().is_err());
    let at = |name: &str| head.iter().position(|c| c.eq_ignore_ascii_case(name));
    let (i, j) = if header {
        (at(a).unwrap_or(0), at(b).unwrap_or(1))
    } else {
        (0, 1)
    };
    let body: Vec<String> = if header {
        Vec::new()
    } else {
        vec![first.to_string()]
    };
    body.iter()
        .map(String::as_str)
        .chain(rows)
        .filter_map(|l| {
            let c = cells(l);
            Some((c.get(i)?.clone(), c.get(j)?.parse().ok()?))
        })
        .collect()
}

fn text_of(job: &Job, name: &str) -> String {
    param(job, name)
        .and_then(Value::str)
        .unwrap_or("")
        .trim()
        .to_string()
}

/// What a hit gets: the check against the standards.
struct Verdict {
    pred: Option<f64>,
    resid: Option<f64>,
    ok: Option<bool>,
    terms: Option<mzlab_core::score_terms::ScoreTerms>,
}

/// The parent of the run: its logP and, when given, its RT.
struct Parent {
    logp: f64,
    rt: Option<f64>,
}

fn parent_of(job: &Job) -> Result<Option<Parent>> {
    let smiles = text_of(job, "parent");
    if smiles.is_empty() {
        return Ok(None);
    }
    let v = LogPValues::from_smiles(&smiles).ok_or_else(|| bad("err.flow.rtpredict.parent"))?;
    let rt = param(job, "parent_rt").and_then(Value::f64);
    Ok(Some(Parent {
        logp: v.crippen.unwrap_or(0.0),
        rt,
    }))
}

/// One hit as it will be written: the object it came as, then the evidence of this step.
type Row = (usize, String, Option<f64>, String);

type Judge = Box<dyn Fn(&str, Option<f64>) -> Verdict>;

/// The check every hit goes through, from the standards: the window of `logp_window` or the forest.
fn judge_of(job: &Job, algorithm: &str, standards: &[u8]) -> Result<Judge> {
    let std_rows = pairs(&String::from_utf8_lossy(standards), "smiles", "rt");
    if algorithm == "forest" {
        let model = RtModel::train(&std_rows, num(job, "seed") as u64)
            .ok_or_else(|| bad("err.flow.rtpredict.standards"))?;
        return Ok(Box::new(move |smiles, obs| {
            let pred = model.predict(smiles);
            Verdict {
                pred,
                resid: pred.zip(obs).map(|(p, o)| o - p),
                ok: None,
                terms: None,
            }
        }));
    }
    let (k, floor) = (
        num(job, "rt_logp_k"),
        Some(num(job, "rt_logp_floor"))
            .filter(|f| *f > 0.0)
            .unwrap_or(1.0),
    );
    let terms = param(job, "rt_logp_terms").and_then(Value::bool) == Some(true);
    let points: Vec<CalPoint> = std_rows
        .iter()
        .filter_map(|(s, rt)| {
            Some(CalPoint {
                logp: LogPValues::from_smiles(s)?,
                y: Some(*rt),
            })
        })
        .collect();
    let model = calibrate(&points, k, floor, MIN_N);
    if model.fit.is_none() {
        return Err(bad("err.flow.rtpredict.standards")
            .with("n", model.n)
            .with("min", MIN_N));
    }
    Ok(Box::new(move |smiles, obs| {
        let Some(lv) = LogPValues::from_smiles(smiles) else {
            return Verdict {
                pred: None,
                resid: None,
                ok: None,
                terms: None,
            };
        };
        let c = check(&model, &lv, obs, k, floor);
        let t = if terms {
            check_terms(&model, &lv, obs)
        } else {
            None
        };
        Verdict {
            pred: c.pred,
            resid: c.resid,
            ok: c.ok,
            terms: t,
        }
    }))
}

fn flag(b: Option<bool>) -> &'static str {
    match b {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    }
}

/// The direction of the RT change against the parent and whether it agrees with the change of logP.
fn direction(o: &mut Out, job: &Job, p: &Parent, delta: f64, rt: Option<f64>) {
    let Some((prt, rt)) = p.rt.zip(rt) else {
        return;
    };
    let seen = Dir::from_delta_logp(rt - prt, num(job, "rt_margin"));
    let name = match seen {
        Dir::MorePolar => "more_polar",
        Dir::LessPolar => "less_polar",
        Dir::Free => "free",
    };
    o.raw(",\"rt_dir\":").str(name).raw(",\"rt_dir_agrees\":");
    o.raw(flag(seen.agrees(delta, num(job, "polarity_margin"))));
}

fn write_hit(o: &mut Out, h: &Value, v: &Verdict, windowed: bool) {
    o.raw("{\"query\":")
        .str(h.get("query").and_then(Value::str).unwrap_or(""));
    for (key, val) in h
        .entries()
        .iter()
        .filter(|(k, _)| k != "query" && !OWN.contains(&k.as_str()))
    {
        o.raw(",").str(key).raw(":");
        json::write_value(o, val);
    }
    let r4 = |x: Option<f64>| x.map(|x| round(x, 4));
    o.raw(",\"rt_pred\":")
        .opt(r4(v.pred))
        .raw(",\"rt_resid\":")
        .opt(r4(v.resid));
    if windowed {
        o.raw(",\"rt_ok\":").raw(flag(v.ok));
    }
    if let Some(t) = &v.terms {
        o.raw(",\"score_terms\":");
        t.write(o);
    }
}

/// Queries in the order they first appear, each one's hits by smallest residual (a hit with none last), ties in file order.
fn sort_by_residual(rows: &mut [Row]) {
    let mut seen: Vec<String> = Vec::new();
    for r in rows.iter() {
        if !seen.contains(&r.1) {
            seen.push(r.1.clone());
        }
    }
    let rank = |q: &str| seen.iter().position(|s| s == q);
    let key = |r: &Row| r.2.unwrap_or(f64::INFINITY);
    rows.sort_by(|a, b| {
        rank(&a.1)
            .cmp(&rank(&b.1))
            .then(key(a).total_cmp(&key(b)))
            .then(a.0.cmp(&b.0))
    });
}

/// Runs the step of `job` on the candidates JSON, the standards CSV (`smiles,rt`) and the optional observed RTs (`query,rt`).
pub fn rtpredict_text(
    job: &Job,
    candidates: &[u8],
    standards: &[u8],
    queries: &[u8],
) -> Result<String> {
    let doc = json::parse(candidates).map_err(|_| bad("err.flow.rtpredict.candidates"))?;
    let hits = doc
        .get("hits")
        .map(Value::arr)
        .ok_or_else(|| bad("err.flow.rtpredict.candidates"))?;
    let observed: HashMap<String, f64> = pairs(&String::from_utf8_lossy(queries), "query", "rt")
        .into_iter()
        .collect();
    let algorithm = job
        .resolved
        .algorithm
        .as_ref()
        .map_or("logp_window", |a| a.0.as_str())
        .to_string();
    let drop = param(job, "drop_outside").and_then(Value::bool) == Some(true);
    if drop && algorithm != "logp_window" {
        return Err(bad("err.flow.rtpredict.window"));
    }
    let (judge, parent) = (judge_of(job, &algorithm, standards)?, parent_of(job)?);
    let mut rows: Vec<Row> = Vec::new();
    for (idx, h) in hits.iter().enumerate() {
        let query = h
            .get("query")
            .and_then(Value::str)
            .unwrap_or("")
            .to_string();
        let smiles = h.get("smiles").and_then(Value::str).unwrap_or("");
        let obs = observed
            .get(&query)
            .copied()
            .or_else(|| h.get("rt").and_then(Value::f64));
        let v = judge(smiles, obs);
        if drop && v.ok == Some(false) {
            continue;
        }
        let mut o = Out::default();
        write_hit(&mut o, h, &v, algorithm == "logp_window");
        if let (Some(p), Some(lv)) = (&parent, LogPValues::from_smiles(smiles)) {
            let delta = lv.crippen.unwrap_or(0.0) - p.logp;
            o.raw(",\"delta_logp\":").num(round(delta, 2));
            direction(&mut o, job, p, delta, obs);
        }
        o.raw("}");
        rows.push((idx, query, v.resid.map(f64::abs), o.0));
    }
    if text_of(job, "order") == "residual" {
        sort_by_residual(&mut rows);
    }
    let mut out = Out::default();
    out.raw("{\"format\":\"candidates/1\",\"step\":\"rt.predict@1\",\"algorithm\":")
        .str(&algorithm)
        .raw(",\"hits\":[");
    for (i, r) in rows.iter().enumerate() {
        out.raw(if i > 0 { "," } else { "" }).raw(&r.3);
    }
    out.raw("]}");
    Ok(out.0)
}

pub(crate) fn rtpredict_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (cands, standards) = (
        read_one(r, job, "candidates")?,
        read_one(r, job, "standards")?,
    );
    let queries = match all_inputs(r, job, "queries").into_iter().next() {
        Some(p) => load(p)?.1,
        None => Vec::new(),
    };
    let text = rtpredict_text(job, &cands.1, &standards.1, &queries)?;
    write_made(r, job, text, DataType::Candidates)
}

#[cfg(test)]
#[path = "flow_rtpredict_tests.rs"]
mod tests;
