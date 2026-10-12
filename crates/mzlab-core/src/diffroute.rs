//! `ref=` on `api/chrom`, `api/xic` and `api/mrm` (the branches of `mzlab/api.py` that subtract a reference file): the options
//! (`_diff_opts`), the scale (`_diff_scale`, `_mrm_area`) and the JSON of the answer. A request Python must word itself (a parameter that
//! does not parse) is `None`; an error with a key is the JSON of `api._fail` (`Err`).

use crate::differenza::{area, cut, norm_factor, subtract_trace};
use crate::error::Result;
use crate::idxread::ReadAt;
use crate::json::{parse, round, Value};
use crate::jsonb::{floats, int, num, text, write, Obj};
use crate::routes::{Query, Routes};

#[derive(Clone, Copy, PartialEq)]
pub enum Norm {
    None,
    Tic,
    Istd,
}

pub struct DiffOpts {
    pub reference: u32,
    pub factor: f64,
    pub norm: Norm,
    pub istd: f64,
    pub rttol: f64,
    pub clip: bool,
}

/// `float(q.get(k, d))`: `None` when the text is not a number (Python words that error itself).
fn number(q: &Query, k: &str, d: f64) -> Option<f64> {
    q.get(k).map_or(Some(d), |v| v.parse().ok())
}

/// The JSON of `api._fail` for a `UserError`.
fn user_error(key: &str, params: Value, msg: &str) -> String {
    write(&Obj::new().put("error", text(msg)).put("error_key", text(key)).put("params", params).done())
}

/// `_diff_opts`: `None` = no reference or a parameter Python words itself; `Some(Err)` = the JSON of the user's error.
pub fn parse_opts(q: &Query) -> Option<std::result::Result<DiffOpts, String>> {
    let reference = q.some("ref")?;
    let word = q.some("norm").unwrap_or("none");
    let norm = match word {
        "none" => Norm::None,
        "tic" => Norm::Tic,
        "istd" => Norm::Istd,
        other => {
            if !other.chars().all(|c| c.is_ascii_alphanumeric() || " _-".contains(c)) {
                return None;                                  // the repr of an unusual text is Python's
            }
            let short: String = other.chars().take(20).collect();
            let msg = format!("norm must be one of none, tic, istd, not '{other}'");
            return Some(Err(user_error("err.diff.norm", Obj::new().put("norm", text(&short)).done(), &msg)));
        }
    };
    let istd = q.some("istd");
    if norm == Norm::Istd && istd.is_none() {
        let msg = "norm=istd needs istd (the m/z of the internal standard)";
        return Some(Err(user_error("err.diff.istd", Obj::new().done(), msg)));
    }
    let reference: u32 = reference.parse().ok()?;
    let (factor, rttol) = (number(q, "factor", 1.0)?, number(q, "rttol", 0.1)?);
    let istd = match istd {
        Some(v) => v.parse().ok()?,
        None => f64::NAN,
    };
    if !((0.0..=100.0).contains(&factor) && (0.0..=10.0).contains(&rttol)) {
        let msg = "factor must be between 0 and 100 and rttol between 0 and 10 minutes";
        return Some(Err(user_error("err.diff.range", Obj::new().done(), msg)));
    }
    Some(Ok(DiffOpts { reference, factor, norm, istd, rttol, clip: q.get("clip") == Some("1") }))
}

/// One trace with the area its normalization needs.
pub struct Tr {
    pub k: u32,
    pub rt: Vec<f64>,
    pub y: Vec<f64>,
    pub area: f64,
}

/// A request of `api/chrom` or `api/xic` with `ref`, ready to be answered from the indexes.
pub struct Plan {
    pub o: DiffOpts,
    pub ks: Vec<u32>,
    pub mz: Option<f64>,
    pub tol: f64,
    pub level: u8,
    chrom: Query,
}

/// The plan of `path` (`/api/chrom` or `/api/xic`); `None` = Python's.
pub fn plan(path: &str, q: &Query) -> Option<std::result::Result<Plan, String>> {
    let o = match parse_opts(q)? {
        Ok(o) => o,
        Err(e) => return Some(Err(e)),
    };
    let level: u8 = q.get("level").map_or(Some(1), |v| v.parse().ok())?;
    let tol = number(q, "tol", 0.35)?;
    let (ks, mz) = match path {
        "/api/xic" => (q.get("k")?.split(',').filter(|x| !x.is_empty()).map(|x| x.parse().ok()).collect::<Option<Vec<u32>>>()?, Some(q.get("mz")?.parse().ok()?)),
        "/api/chrom" => {
            ["mz0", "mz1", "prec"].iter().try_for_each(|k| q.num(k).ok().map(drop))?;
            (vec![q.get("k")?.parse().ok()?], None)
        }
        _ => return None,
    };
    Some(Ok(Plan { o, ks, mz, tol, level, chrom: Query(q.0.clone()) }))
}

impl Plan {
    /// The trace of a file and, when the normalization needs one, its area (total ion current or internal standard); `None` = Python's.
    pub fn fetch<R: ReadAt>(&self, k: u32, r: &mut Routes<R>) -> Result<Option<Tr>> {
        if !r.has_scans() {
            return Ok(None);
        }
        let got = match self.mz {
            Some(mz) => Some(r.xic_raw(mz, self.tol, self.level)?),
            None => r.chrom_raw(&self.chrom)?,
        };
        let Some((rt, y)) = got else { return Ok(None) };
        let area = match self.o.norm {
            Norm::None => 0.0,
            Norm::Tic => {
                let q = Query(vec![("kind".into(), "tic".into()), ("level".into(), self.level.to_string())]);
                r.chrom_raw(&q)?.map_or(0.0, |(t, v)| area(&t, &v))
            }
            Norm::Istd => {
                let (t, v) = r.xic_raw(self.o.istd, self.tol, self.level)?;
                area(&t, &v)
            }
        };
        Ok(Some(Tr { k, rt, y, area }))
    }

    /// `factor` times the normalization of the reference towards `t`.
    fn scale(&self, t: &Tr, reference: &Tr) -> f64 {
        match self.o.norm {
            Norm::None => self.o.factor,
            _ => self.o.factor * norm_factor(true, t.area, reference.area),
        }
    }

    fn difference(&self, t: &Tr, reference: &Tr) -> (Value, Value, f64) {
        let sc = self.scale(t, reference);
        let d = subtract_trace(&t.rt, &t.y, &reference.rt, &reference.y, sc, self.o.rttol, self.o.clip);
        let (rt, y) = cut(&t.rt, &d);
        let round_to = |a: Vec<f64>, nd| floats(&a.into_iter().map(|v| round(v, nd)).collect::<Vec<_>>());
        (round_to(rt, 4), round_to(y, 1), sc)
    }

    /// The JSON Python answers: one trace for `api/chrom`, the list of traces for `api/xic`.
    pub fn finish(&self, reference: &Tr, files: &[Tr]) -> String {
        let r = int(self.o.reference as i64);
        let Some(mz) = self.mz else {
            let (rt, y, sc) = self.difference(&files[0], reference);
            return write(&Obj::new().put("rt", rt).put("y", y).put("ref", r).n("scale", sc).done());
        };
        let traces = files.iter().map(|t| {
            let (rt, y, sc) = self.difference(t, reference);
            Obj::new().put("k", int(t.k as i64)).put("rt", rt).put("y", y).n("scale", sc).done()
        });
        write(&Obj::new().n("mz", mz).n("tol", self.tol).put("ref", r).put("traces", Value::Arr(traces.collect())).done())
    }
}

// ------------------------------------------------------------------------------------------------ MRM
/// `_mrm_area`: the area to normalize an MRM file by (all its transitions, or the ones whose precursor is the internal standard).
fn mrm_area(trs: &[Value], o: &DiffOpts) -> f64 {
    if o.norm == Norm::None {
        return 0.0;
    }
    let col = |t: &Value, k: &str| t.get(k).map_or_else(Vec::new, |a| a.arr().iter().filter_map(Value::f64).collect::<Vec<_>>());
    let mut sum = 0.0;
    for t in trs {
        let q1 = t.get("q1").and_then(Value::f64).unwrap_or(0.0);
        if o.norm == Norm::Tic || (q1 - o.istd).abs() <= 0.5 {
            sum += area(&col(t, "rt"), &col(t, "y"));
        }
    }
    sum
}

fn same_pair(a: &Value, b: &Value) -> bool {
    ["q1", "q3"].iter().all(|k| a.get(k).and_then(Value::f64) == b.get(k).and_then(Value::f64))
}

/// `t` with the keys of `pairs` replaced in place (or appended), as `{**t, ...}` does.
fn with(t: &Value, pairs: Vec<(&str, Value)>) -> Value {
    let mut o = match t {
        Value::Obj(m) => m.clone(),
        _ => Vec::new(),
    };
    for (k, v) in pairs {
        match o.iter_mut().find(|(n, _)| n == k) {
            Some(slot) => slot.1 = v,
            None => o.push((k.to_string(), v)),
        }
    }
    Value::Obj(o)
}

fn column(t: &Value, k: &str) -> Vec<f64> {
    t.get(k).map_or_else(Vec::new, |a| a.arr().iter().filter_map(Value::f64).collect())
}

fn mrm_one(t: &Value, r: &Value, sc: f64, o: &DiffOpts) -> Value {
    let (rt, y) = (column(t, "rt"), column(t, "y"));
    let d = subtract_trace(&rt, &y, &column(r, "rt"), &column(r, "y"), sc, o.rttol, o.clip);
    let (rt, y) = cut(&rt, &d);
    let round_to = |a: Vec<f64>, nd| floats(&a.into_iter().map(|v| round(v, nd)).collect::<Vec<_>>());
    with(t, vec![("rt", round_to(rt, 4)), ("y", round_to(y, 1)), ("scale", num(sc))])
}

/// The answer of `api/mrm` with `ref`: `files` = `[{"k", "transitions"}]`, `reference` = the transitions of the reference file.
pub fn mrm_diff(files: &str, reference: &str, q: &Query) -> Option<std::result::Result<String, String>> {
    let o = match parse_opts(q)? {
        Ok(o) => o,
        Err(e) => return Some(Err(e)),
    };
    let (files, rf) = (parse(files.as_bytes()).ok()?, parse(reference.as_bytes()).ok()?);
    let ref_area = mrm_area(rf.arr(), &o);
    let mut out = Vec::new();
    for f in files.arr() {
        let trs = f.get("transitions").map_or(&[][..], Value::arr);
        let file_area = mrm_area(trs, &o);
        let sc = o.factor * if o.norm == Norm::None { 1.0 } else { norm_factor(true, file_area, ref_area) };
        let done = trs.iter().map(|t| match rf.arr().iter().rev().find(|r| same_pair(t, r)) {
            Some(r) => mrm_one(t, r, sc, &o),
            None => with(t, vec![("nodiff", Value::Bool(true))]),
        });
        out.push(with(f, vec![("transitions", Value::Arr(done.collect()))]));
    }
    Some(Ok(write(&Obj::new().put("files", Value::Arr(out)).put("ref", int(o.reference as i64)).done())))
}
