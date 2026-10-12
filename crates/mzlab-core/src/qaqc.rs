//! `mzlab/qaqc.py` and `App.qaqc_ion`: QA/QC measures of ONE ion over a series of files (apex RT, area, observed m/z and error in ppm per
//! file, drift, CV, mass accuracy, sample / blank). Numbers only, never verdicts. The request is `api/qaqc`; the answer the JSON of
//! Python (`None` = Python's: a parameter that does not read, a file without index).

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::json::{round, Value};
use crate::jsonb::{int, num, opt, text, write, Obj};
use crate::routes::{Query, Routes};
use crate::stats::psum;

/// The peak of an ion in a window of time.
pub struct Peak {
    pub apex: f64,
    pub area: f64,
    pub height: f64,
    pub i0: usize,
    pub i1: usize,
}

/// `peak_in_window`: apex, area (above the straight line between the edges of the window) and height of the highest point within
/// `expected ± half` minutes; `None` for fewer than 3 scans or no signal.
pub fn peak_in_window(rt: &[f64], y: &[f64], expected: f64, half: f64) -> Option<Peak> {
    let sel: Vec<usize> = (0..rt.len()).filter(|&i| rt[i] >= expected - half && rt[i] <= expected + half).collect();
    if sel.len() < 3 || !sel.iter().any(|&i| y[i] > 0.0) {
        return None;
    }
    let (t, v): (Vec<f64>, Vec<f64>) = sel.iter().map(|&i| (rt[i], y[i])).unzip();
    let n = v.len();
    let step = (v[n - 1] - v[0]) / (n - 1) as f64;
    let net: Vec<f64> = (0..n).map(|i| {
        let base = if i == n - 1 { v[n - 1] } else { i as f64 * step + v[0] };
        (v[i] - base).max(0.0)
    }).collect();
    let mut best = 0;
    for i in 1..n {
        if net[i] > net[best] {
            best = i;
        }
    }
    if net[best] <= 0.0 {
        return None;
    }
    let strips: Vec<f64> = (1..n).map(|i| (net[i] + net[i - 1]) * (t[i] - t[i - 1])).collect();
    Some(Peak { apex: t[best], area: psum(&strips) / 2.0, height: net[best], i0: sel[0], i1: sel[n - 1] })
}

/// `cv_percent`: relative standard deviation (n − 1) in percent; `None` for fewer than two values or a zero mean.
pub fn cv_percent(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mean = psum(values) / values.len() as f64;
    if mean == 0.0 {
        return None;
    }
    let sq: Vec<f64> = values.iter().map(|x| (x - mean) * (x - mean)).collect();
    Some((psum(&sq) / (values.len() - 1) as f64).sqrt() / mean.abs() * 100.0)
}

/// `slope`: least-squares slope per position of the series (`None` below three defined values).
pub fn slope(values: &[Option<f64>]) -> Option<f64> {
    let pts: Vec<(f64, f64)> = values.iter().enumerate().filter_map(|(i, v)| v.map(|v| (i as f64, v))).collect();
    if pts.len() < 3 {
        return None;
    }
    let n = pts.len() as f64;
    let (mx, my) = (pts.iter().map(|p| p.0).sum::<f64>() / n, pts.iter().map(|p| p.1).sum::<f64>() / n);
    let sxy: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let sxx: f64 = pts.iter().map(|p| (p.0 - mx) * (p.0 - mx)).sum();
    Some(sxy / sxx)
}

/// `np.median` of a non-empty list.
fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = s.len();
    if n % 2 == 1 { s[n / 2] } else { (s[n / 2 - 1] + s[n / 2]) / 2.0 }
}

/// What is measured in one file.
pub struct Row {
    pub k: u32,
    pub rt: f64,
    pub area: f64,
    pub height: f64,
    pub mz_obs: Option<f64>,
    pub dppm: Option<f64>,
}

fn live(role: &str) -> bool {
    role != "blank" && role != "dark"
}

fn row_value(r: &Row, expected: f64, med: Option<f64>) -> Value {
    Obj::new().i("k", r.k as i64).n("rt", r.rt).n("area", r.area).n("height", r.height).put("mz_obs", opt(r.mz_obs)).put("dppm", opt(r.dppm))
        .n("drt", r.rt - expected).put("rel", opt(med.map(|m| r.area / m))).done()
}

/// `summarize` (+ the `mass_accuracy` of the rows).
fn summary(rows: &[Option<Row>], roles: &[String], expected: f64) -> Obj {
    let refs: Vec<f64> = rows.iter().zip(roles).filter_map(|(r, ro)| r.as_ref().filter(|r| live(ro) && r.area != 0.0).map(|r| r.area)).collect();
    let med = if refs.is_empty() { None } else { Some(median(&refs)).filter(|m| *m != 0.0) };
    let found: Vec<&Row> = rows.iter().flatten().collect();
    let area_of = |want: &dyn Fn(&str) -> bool| -> Vec<f64> { rows.iter().zip(roles).filter_map(|(r, ro)| r.as_ref().filter(|_| want(ro)).map(|r| r.area)).collect() };
    let errs: Vec<f64> = found.iter().filter_map(|r| r.dppm).collect();
    let accuracy = if errs.is_empty() {
        Value::Null
    } else {
        Obj::new().n("mean", psum(&errs) / errs.len() as f64).n("max_abs", errs.iter().fold(0.0, |m: f64, e| m.max(e.abs()))).i("n", errs.len() as i64).done()
    };
    let series = |f: &dyn Fn(&Row) -> f64| -> Vec<Option<f64>> { rows.iter().map(|r| r.as_ref().map(f)).collect() };
    Obj::new().put("rows", Value::Arr(rows.iter().map(|r| r.as_ref().map_or(Value::Null, |r| row_value(r, expected, med))).collect()))
        .put("median_area", opt(if refs.is_empty() { None } else { Some(median(&refs)) }))
        .put("rt_drift_per_file", opt(slope(&series(&|r| r.rt)))).put("area_drift_per_file", opt(slope(&series(&|r| r.area))))
        .put("cv_area", opt(cv_percent(&area_of(&live)))).put("cv_area_qc", opt(cv_percent(&area_of(&|ro| ro == "qc"))))
        .put("mass_accuracy", accuracy)
}

/// `sample_blank_ratios`: for every file that is not a blank, area / mean area of the blanks (the area of a file without the ion is 0).
fn ratios(rows: &[Option<Row>], roles: &[String], threshold: f64) -> Value {
    let area = |r: &Option<Row>| r.as_ref().map_or(0.0, |r| r.area);
    let bl: Vec<f64> = rows.iter().zip(roles).filter(|(_, ro)| *ro == "blank").map(|(r, _)| area(r)).collect();
    let reference = if bl.is_empty() { None } else { Some(psum(&bl) / bl.len() as f64) };
    let out = rows.iter().zip(roles).map(|(r, ro)| match reference {
        Some(b) if ro != "blank" => {
            let ratio = if b == 0.0 { f64::INFINITY } else { area(r) / b };
            Obj::new().put("ratio", num(ratio)).b("absent_in_blank", ratio.is_infinite()).b("at_least", ratio >= threshold).done()
        }
        _ => Value::Null,
    });
    Obj::new().put("blank_area", opt(reference)).n("threshold", threshold).put("rows", Value::Arr(out.collect())).done()
}

/// A request of `api/qaqc`.
pub struct Plan {
    pub ks: Vec<u32>,
    roles: Vec<String>,
    mz: f64,
    rt: f64,
    win: f64,
    tol: f64,
    unit: String,
    level: u8,
    thr: f64,
    ratio: bool,
}

fn number(q: &Query, k: &str, d: Option<f64>) -> Option<f64> {
    match (q.get(k), d) {
        (Some(v), _) => v.parse().ok(),
        (None, d) => d,
    }
}

/// The plan of the request; `None` = Python's; `Some(Err)` = the JSON of the user's error (one role per file is needed).
pub fn plan(q: &Query) -> Option<std::result::Result<Plan, String>> {
    let ks = q.get("k")?.split(',').filter(|x| !x.is_empty()).map(|x| x.parse().ok()).collect::<Option<Vec<u32>>>()?;
    let roles: Vec<String> = q.get("roles").unwrap_or("").split(',').map(String::from).collect();
    let (mz, rt) = (number(q, "mz", None)?, number(q, "rt", None)?);
    let (win, tol, thr) = (number(q, "win", Some(0.5))?, number(q, "tol", Some(0.5))?, number(q, "thr", Some(3.0))?);
    let level: u8 = q.get("level").map_or(Some(1), |v| v.parse().ok())?;
    if ks.len() != roles.len() {
        let e = Obj::new().put("error", text("one role per file is needed")).put("error_key", text("err.qaqc.roles")).put("params", Obj::new().done()).done();
        return Some(Err(write(&e)));
    }
    Some(Ok(Plan { ks, roles, mz, rt, win, tol, unit: q.get("unit").unwrap_or("da").to_string(), level, thr, ratio: q.get("ratio") == Some("1") }))
}

impl Plan {
    /// The measures of one file (`None` outer = no index scans: Python's; `None` inner = the ion is not found).
    pub fn measure<R: ReadAt>(&self, k: u32, r: &mut Routes<R>) -> Result<Option<Option<Row>>> {
        if !r.has_scans() {
            return Ok(None);
        }
        let tol = if self.unit == "da" { self.tol } else { self.mz * self.tol * 1e-6 };
        let Some(si) = r.idx.section_of(self.level, None) else { return Ok(Some(None)) };
        let (rt, y) = r.xic_raw(self.mz, tol, self.level)?;
        let Some(pk) = peak_in_window(&rt, &y, self.rt, self.win) else { return Ok(Some(None)) };
        let (a, b) = (r.idx.search(si, self.mz - tol, false)?, r.idx.search(si, self.mz + tol, true)?);
        let (mut weighted, mut weights) = (Vec::new(), Vec::new());
        r.idx.each_peak(si, a, b.max(a), |p, mz, inten| {
            if p >= pk.i0 && p <= pk.i1 {
                weighted.push(mz * inten);
                weights.push(inten);
            }
        })?;
        let total = psum(&weights);
        let mz_obs = if total > 0.0 { Some(psum(&weighted) / total) } else { None };
        let dppm = mz_obs.filter(|_| self.unit != "da").map(|mo| round((mo - self.mz) / self.mz * 1e6, 3));
        Ok(Some(Some(Row { k, rt: round(pk.apex, 4), area: pk.area, height: pk.height, mz_obs: mz_obs.map(|m| round(m, 5)), dppm })))
    }

    /// The JSON of the answer from the rows of the files in the order of the request.
    pub fn finish(&self, rows: &[Option<Row>]) -> String {
        let mut o = summary(rows, &self.roles, self.rt);
        if self.ratio {
            o.set("ratio", ratios(rows, &self.roles, self.thr));
        }
        o.set("mz", num(self.mz));
        o.set("rt", num(self.rt));
        o.set("win", num(self.win));
        o.set("tol", num(self.tol));
        o.set("unit", text(&self.unit));
        o.set("level", int(self.level as i64));
        write(&o.done())
    }
}
