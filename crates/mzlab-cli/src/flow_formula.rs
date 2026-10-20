//! The native executors of `formula.ms1@1` (candidate formulas of the features from the MS1 scan at the apex: `untargeted::formula_id`) and `formula.ms2@1`
//! (sub-formulas of the candidates that explain the peaks of an MS2 spectrum: `review::explain`, `ms2_engine::rearrange`). Both write JSON `formulas/1`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::json::{self, round, Out, Value};
use mzlab_core::mzml::Run;
use mzlab_feat::attributio::ms2_engine::rearrange::attribute;
use mzlab_feat::attributio::review::explain::{explain, fmt_formula, parse_formula, prep_ms2};
use mzlab_feat::attributio::review::settings::ReviewSettings;
use mzlab_feat::untargeted::formula_id::{identify, to_json, Opts};

use crate::flow_match::{num, query_id, read_spectra};

fn io(e: std::io::Error) -> Error {
    Error::new("err.flow.io").with("detail", e)
}

fn param<'a>(job: &'a Job, name: &str) -> Option<&'a Value> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
}

fn opts(job: &Job) -> Opts {
    let mut o = Opts::default();
    if let Some(s) = param(job, "ion").and_then(Value::str) {
        o.ion = s.to_string();
    }
    o.z = param(job, "z").and_then(Value::f64).map(|z| z as i32);
    o.tol_ppm = num(job, "tol_ppm");
    o.elements = param(job, "elements")
        .and_then(Value::str)
        .map(String::from);
    if let Some(s) = param(job, "nitrogen").and_then(Value::str) {
        o.nitrogen = s.to_string();
    }
    o.rdb_min = num(job, "rdb_min");
    o.max_results = num(job, "max_results").max(1.0) as usize;
    o.finder.max_charge = num(job, "max_charge").max(1.0) as i32;
    o.finder.require_c13 = param(job, "require_c13")
        .and_then(Value::bool)
        .unwrap_or(true);
    o
}

/// The feature table as the CSV "full": header and rows; the area columns are `datafile:NAME:area`.
struct Table {
    id: usize,
    mz: usize,
    rt: usize,
    areas: Vec<(String, usize)>,
    rows: Vec<Vec<String>>,
}

fn read_table(path: &Path) -> Result<Table> {
    let text = std::fs::read_to_string(path).map_err(io)?;
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().unwrap_or("").split(',').collect();
    let col = |n: &str| {
        head.iter()
            .position(|h| *h == n)
            .ok_or_else(|| Error::new("err.flow.formula.table").with("column", n))
    };
    let areas = head
        .iter()
        .enumerate()
        .filter_map(|(k, h)| {
            h.strip_prefix("datafile:")?
                .strip_suffix(":area")
                .map(|n| (n.to_string(), k))
        })
        .collect();
    Ok(Table {
        id: col("id")?,
        mz: col("mz")?,
        rt: col("rt")?,
        areas,
        rows: lines
            .map(|l| l.split(',').map(String::from).collect())
            .collect(),
    })
}

fn stem(p: &Path) -> String {
    p.file_stem()
        .map_or(String::new(), |s| s.to_string_lossy().into_owned())
}

/// The MS1 scan of `run` closest in RT to `rt` whose polarity is `sign` (or not said).
fn nearest_ms1(run: &Run, rt: f64, sign: i8) -> Option<(usize, Vec<f64>, Vec<f64>)> {
    let s = run
        .spectra
        .iter()
        .filter(|s| s.level == 1 && (s.polarity == 0 || s.polarity == sign))
        .min_by(|a, b| {
            (a.rt - rt)
                .abs()
                .total_cmp(&(b.rt - rt).abs())
                .then(a.index.cmp(&b.index))
        })?;
    Some((
        s.index,
        s.mz.clone(),
        s.intensity.iter().map(|v| *v as f64).collect(),
    ))
}

/// `formula.ms1`: for each feature the MS1 scan at the apex, in the file where the feature is largest, goes to `formula_id::identify`.
pub fn ms1_text(job: &Job, features: &Path, files: &[PathBuf]) -> Result<String> {
    let t = read_table(features)?;
    let o = opts(job);
    let sign = if o.ion.ends_with('-') { -1 } else { 1 };
    let mut runs: HashMap<String, Run> = HashMap::new();
    let mut out = Out::default();
    out.raw("{\"format\":\"formulas/1\",\"step\":\"formula.ms1@1\",\"algorithm\":\"compose_isotopes\",\"features\":[");
    for (n, row) in t.rows.iter().enumerate() {
        let cell = |k: usize| row.get(k).map(String::as_str).unwrap_or("");
        let (mz, rt) = (
            cell(t.mz).parse::<f64>().unwrap_or(f64::NAN),
            cell(t.rt).parse::<f64>().unwrap_or(f64::NAN),
        );
        let area = |k: usize| cell(k).parse::<f64>().unwrap_or(0.0);
        let best = t.areas.iter().fold(None, |b: Option<&(String, usize)>, a| {
            if b.is_none_or(|b| area(a.1) > area(b.1)) {
                Some(a)
            } else {
                b
            }
        });
        let path = best
            .and_then(|b| files.iter().find(|f| stem(f) == b.0))
            .or(if files.len() == 1 {
                files.first()
            } else {
                None
            })
            .ok_or_else(|| Error::new("err.flow.formula.file"))?;
        if let std::collections::hash_map::Entry::Vacant(e) = runs.entry(stem(path)) {
            let bytes = std::fs::read(path).map_err(io)?;
            e.insert(mzlab_core::mzml::read(&bytes[..])?);
        }
        out.raw(if n > 0 { "," } else { "" })
            .raw("{\"id\":")
            .str(cell(t.id))
            .raw(",\"mz\":")
            .num(mz)
            .raw(",\"rt\":")
            .num(rt);
        out.raw(",\"file\":").str(&stem(path));
        match nearest_ms1(&runs[&stem(path)], rt, sign) {
            None => {
                out.raw(",\"error\":\"err.flow.formula.scan\"}");
            }
            Some((scan, m, y)) => {
                out.raw(",\"scan\":").int(scan as i64);
                match identify(&m, &y, mz, &o) {
                    Ok(r) => {
                        out.raw(",\"result\":").raw(&to_json(&r)).raw("}");
                    }
                    // a wrong parameter stops the step; a feature the search cannot treat is recorded
                    Err(e) if e.key == "err.formulaId.param" || e.key == "err.comp.ion" => {
                        return Err(e)
                    }
                    Err(e) => {
                        out.raw(",\"error\":").str(e.key).raw("}");
                    }
                }
            }
        }
    }
    out.raw("]}");
    Ok(out.0)
}

/// One candidate formula of a `formulas/1` file with the feature it belongs to.
struct Cand {
    feature: String,
    formula: String,
}

fn candidates(files: &[PathBuf]) -> Result<Vec<Cand>> {
    let mut all = Vec::new();
    for f in files {
        let v = json::parse(&std::fs::read(f).map_err(io)?)?;
        for feat in v.get("features").map(Value::arr).unwrap_or(&[]) {
            let id = feat.get("id").and_then(Value::str).unwrap_or("");
            let list = feat
                .get("result")
                .and_then(|r| r.get("candidates"))
                .map(Value::arr)
                .unwrap_or(&[]);
            for c in list {
                let neutral = c.get("neutral").and_then(Value::str);
                if let Some(formula) = neutral.or_else(|| c.get("formula").and_then(Value::str)) {
                    all.push(Cand {
                        feature: id.to_string(),
                        formula: formula.to_string(),
                    });
                }
            }
        }
    }
    Ok(all)
}

/// What one candidate explains of one spectrum.
struct Explained {
    adduct: String,
    ion: String,
    ppm: f64,
    frac: f64,
    decoy: Option<f64>,
    score: f64,
    // m/z and intensity of the peak, formula, theoretical m/z, ppm, loss, hydrogens moved
    frags: Vec<(f64, f64, String, f64, f64, String, i32)>,
}

fn run_candidate(
    alg: &str,
    c: &Cand,
    prec: f64,
    pol: &str,
    peaks: (&[f64], &[f64]),
    s: &ReviewSettings,
) -> Option<Explained> {
    let fd = parse_formula(&c.formula);
    if alg == "h_rearrangement" {
        let a = attribute(&fd, prec, pol, peaks, s)?;
        let ppm = mzlab_feat::attributio::review::explain::match_adduct(&fd, prec, pol)
            .map_or(0.0, |m| m.ppm);
        let frags = a
            .hits
            .iter()
            .enumerate()
            .filter_map(|(i, h)| {
                h.as_ref().map(|h| {
                    (
                        peaks.0[i],
                        peaks.1[i],
                        h.formula.clone(),
                        h.mz_th,
                        h.ppm,
                        h.loss.clone(),
                        h.shift,
                    )
                })
            })
            .collect();
        return Some(Explained {
            adduct: a.adduct,
            ion: a.ion_formula,
            ppm,
            frac: a.frac_explained,
            decoy: None,
            score: a.score,
            frags,
        });
    }
    let e = explain(Some(&fd), prec, pol, peaks, s);
    if !e.fits(s) {
        return None;
    }
    let frags = e
        .frags
        .iter()
        .enumerate()
        .filter_map(|(i, h)| {
            h.as_ref().map(|h| {
                (
                    peaks.0[i],
                    peaks.1[i],
                    h.formula.clone(),
                    h.mz_th,
                    h.ppm,
                    h.loss.clone(),
                    0,
                )
            })
        })
        .collect();
    let ion = e.ion.as_ref().map(fmt_formula).unwrap_or_default();
    Some(Explained {
        adduct: e.adduct.unwrap_or_default(),
        ion,
        ppm: e.ion_ppm,
        frac: e.frac_expl,
        decoy: Some(e.decoy_frac),
        score: e.frac_expl,
        frags,
    })
}

fn write_explained(o: &mut Out, c: &Cand, x: &Explained) {
    o.raw("{\"feature\":")
        .str(&c.feature)
        .raw(",\"formula\":")
        .str(&c.formula)
        .raw(",\"adduct\":")
        .str(&x.adduct);
    o.raw(",\"ion_formula\":")
        .str(&x.ion)
        .raw(",\"ppm_ion\":")
        .num(round(x.ppm, 3));
    o.raw(",\"n_explained\":")
        .int(x.frags.len() as i64)
        .raw(",\"frac_explained\":")
        .num(round(x.frac, 4));
    o.raw(",\"decoy_frac\":")
        .opt(x.decoy.map(|d| round(d, 4)))
        .raw(",\"score\":")
        .num(round(x.score, 4))
        .raw(",\"fragments\":[");
    for (k, f) in x.frags.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"mz\":")
            .num(round(f.0, 5))
            .raw(",\"intensity\":")
            .num(round(f.1, 3));
        o.raw(",\"formula\":")
            .str(&f.2)
            .raw(",\"mz_th\":")
            .num(round(f.3, 5))
            .raw(",\"ppm\":")
            .num(round(f.4, 3));
        o.raw(",\"loss\":")
            .str(&f.5)
            .raw(",\"shift\":")
            .int(f.6 as i64)
            .raw("}");
    }
    o.raw("]}");
}

/// `formula.ms2`: every candidate formula whose ion fits the precursor of a spectrum explains its cleaned peaks with sub-formulas.
pub fn ms2_text(job: &Job, spectra: &Path, formulas: &[PathBuf]) -> Result<String> {
    let alg = job
        .resolved
        .algorithm
        .as_ref()
        .map_or("subformula", |a| a.0.as_str())
        .to_string();
    let s = ReviewSettings {
        ppm_ion: num(job, "ppm_ion"),
        ppm_frag: num(job, "ppm_frag"),
        max_subformulas: num(job, "max_subformulas") as i64,
        ..ReviewSettings::default()
    };
    let cands = candidates(formulas)?;
    let mut out = Out::default();
    out.raw("{\"format\":\"formulas/1\",\"step\":\"formula.ms2@1\",\"algorithm\":")
        .str(&alg)
        .raw(",\"spectra\":[");
    for (k, sp) in read_spectra(spectra)?.iter().enumerate() {
        let pol = if sp.pol < 0 { "NEG" } else { "POS" };
        let (m, y, _) = prep_ms2(&sp.mz, &sp.it, sp.prec, s.rel_min);
        let mut found: Vec<(&Cand, Explained)> = cands
            .iter()
            .filter(|_| !m.is_empty())
            .filter_map(|c| run_candidate(&alg, c, sp.prec, pol, (&m, &y), &s).map(|x| (c, x)))
            .collect();
        found.sort_by(|a, b| {
            b.1.score
                .total_cmp(&a.1.score)
                .then(a.0.formula.cmp(&b.0.formula))
        });
        out.raw(if k > 0 { "," } else { "" })
            .raw("{\"query\":")
            .str(&query_id(k, sp))
            .raw(",\"precursor\":")
            .num(sp.prec);
        out.raw(",\"peaks\":")
            .int(m.len() as i64)
            .raw(",\"candidates\":[");
        for (i, (c, x)) in found.iter().enumerate() {
            out.raw(if i > 0 { "," } else { "" });
            write_explained(&mut out, c, x);
        }
        out.raw("]}");
    }
    out.raw("]}");
    Ok(out.0)
}
