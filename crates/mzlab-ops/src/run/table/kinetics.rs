//! The text of `quant.kinetics@1`: the areas of a time series (fixed RT windows, from `quant.targets`) joined to the times of `samples/1`, divided by the
//! UV area of a reference peak per file when a UV table is wired (the injection check), and a first-order decay `y = A exp(-k t)` fitted per ion and
//! series with its bootstrap interval (`mzlab_core::kinetics::fit_decay`). Every choice is logged; nothing is left out silently. Bytes in, text out:
//! `flow run` (`mzlab-cli`) and the browser module `flowkin` both call it, so they write the same bytes.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;
use mzlab_core::jsonb::{num, opt, text, Obj};
use mzlab_core::kinetics::fit_decay;
use mzlab_core::stats::wspearman;
use mzlab_core::tracestat::percentile;
use mzlab_feat::samples::samples_of;

use super::kinetics_read::{parse_areas, parse_uv, settings, stem, Area, Set, Uv};
use super::kinetics_write::{document, uv_json};

/// Fewest distinct times a decay is fitted on; the engine calls a fit reliable from five points.
const MIN_TIMES: usize = 3;
/// Spearman rho of a flat-fitted series from which it is called rising (a formation, not a decay).
const RISING: f64 = 0.8;
/// |Spearman rho| of the UV area against time from which the reference looks like it reacts, not like an injection check.
const UV_TREND: f64 = 0.8;

/// One file of the series: its role, series, time and what the UV reference says.
pub(super) struct File {
    pub(super) stem: String,
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) series: String,
    pub(super) time: Option<f64>,
    pub(super) uv: Option<(f64, f64)>,
    pub(super) factor: Option<f64>,
    /// `None` = its areas enter the fits, else why not.
    pub(super) out: Option<&'static str>,
}

/// An automatic choice, written in the log.
pub(super) struct Choice(
    pub(super) String,
    pub(super) String,
    pub(super) String,
    pub(super) String,
);

fn choice(log: &mut Vec<Choice>, key: &str, subject: &str, value: String, reason: &str) {
    log.push(Choice(key.into(), subject.into(), value, reason.into()));
}

fn median(v: &[f64]) -> f64 {
    percentile(v, 50.0)
}

/// The files of `samples/1`: sample and t0 files with a time are used (t0 files join every series), the others are listed with the reason.
fn files_of(samples: &[u8], areas: &[Area], set: &Set, log: &mut Vec<Choice>) -> Result<Vec<File>> {
    let mut out = Vec::new();
    for s in samples_of(samples)? {
        let st = stem(&s.file);
        let (time, series, why) = match s.kind.as_str() {
            "sample" if s.time_min.is_none() => (None, s.group.clone(), Some("no_time")),
            "sample" => (s.time_min, s.group.clone(), None),
            "t0" if !set.use_t0 => (s.time_min, String::new(), Some("t0_not_used")),
            "t0" => {
                if s.time_min.is_none() {
                    choice(
                        log,
                        "t0_time",
                        &s.file,
                        "0".into(),
                        "t0 file with no time in samples",
                    );
                }
                (Some(s.time_min.unwrap_or(0.0)), String::new(), None)
            }
            _ => (s.time_min, String::new(), Some("role_not_used")),
        };
        let why = why.or_else(|| (!areas.iter().any(|a| a.stem == st)).then_some("no_areas"));
        out.push(File {
            stem: st,
            name: s.file,
            kind: s.kind,
            series,
            time,
            uv: None,
            factor: None,
            out: why,
        });
    }
    Ok(out)
}

/// The UV reference per file: the peak nearest to the reference RT (given, else the median apex of the largest peak of each used file) and its factor
/// against the median area of the used files. Returns the reference RT and the median area, `None` without a UV table.
fn uv_factors(
    files: &mut [File],
    uv: &[Uv],
    set: &Set,
    log: &mut Vec<Choice>,
) -> Option<(f64, f64)> {
    if uv.is_empty() {
        return None;
    }
    let used = |f: &File| f.out.is_none();
    let rt_ref = if set.uv_rt > 0.0 {
        set.uv_rt
    } else {
        let tops: Vec<f64> = files
            .iter()
            .filter(|f| used(f))
            .filter_map(|f| {
                uv.iter()
                    .filter(|p| p.stem == f.stem)
                    .max_by(|a, b| a.area.total_cmp(&b.area))
                    .map(|p| p.rt)
            })
            .collect();
        let m = if tops.is_empty() {
            f64::NAN
        } else {
            median(&tops)
        };
        choice(
            log,
            "uv_rt",
            "",
            format!("{m}"),
            "median apex of the largest UV peak of each used file",
        );
        m
    };
    for f in files.iter_mut().filter(|f| f.out.is_none()) {
        let near = uv
            .iter()
            .filter(|p| p.stem == f.stem && (p.rt - rt_ref).abs() <= set.uv_rt_tol);
        f.uv = near
            .min_by(|a, b| (a.rt - rt_ref).abs().total_cmp(&(b.rt - rt_ref).abs()))
            .map(|p| (p.rt, p.area));
    }
    let areas: Vec<f64> = files
        .iter()
        .filter(|f| used(f))
        .filter_map(|f| f.uv.map(|u| u.1))
        .collect();
    let reference = if areas.is_empty() {
        f64::NAN
    } else {
        median(&areas)
    };
    for f in files.iter_mut().filter(|f| f.out.is_none()) {
        f.factor =
            f.uv.map(|u| u.1 / reference)
                .filter(|v| v.is_finite() && *v > 0.0);
        if f.factor.is_none() && set.normalise {
            f.out = Some("no_uv_peak");
        }
    }
    Some((rt_ref, reference))
}

/// One point of an ion in a series, with everything that decided its use.
pub(super) struct Pt {
    pub(super) id: String,
    pub(super) series: String,
    pub(super) file: usize,
    pub(super) area: Option<f64>,
    pub(super) state: String,
    pub(super) y: Option<f64>,
    pub(super) out: Option<&'static str>,
}

fn points(files: &[File], areas: &[Area], ids: &[String], series: &[String], set: &Set) -> Vec<Pt> {
    let mut out = Vec::new();
    for id in ids {
        for sr in series {
            for (k, f) in files.iter().enumerate().filter(|(_, f)| {
                f.series == *sr || (f.kind == "t0" && f.out != Some("t0_not_used"))
            }) {
                if f.kind != "sample" && f.kind != "t0" {
                    continue;
                }
                let a = areas.iter().find(|a| a.stem == f.stem && a.id == *id);
                let area = a.and_then(|a| a.area);
                let scale = if set.normalise {
                    f.factor.unwrap_or(1.0)
                } else {
                    1.0
                };
                let (y, why) = match (area, f.out) {
                    (_, Some(w)) => (None, Some(w)),
                    (None, None) if set.zero_missing => (Some(0.0), None),
                    (None, None) => (None, Some("no_peak")),
                    (Some(v), None) => (Some(v / scale), None),
                };
                out.push(Pt {
                    id: id.clone(),
                    series: sr.clone(),
                    file: k,
                    area,
                    state: a.map(|a| a.state.clone()).unwrap_or_default(),
                    y,
                    out: why,
                });
            }
        }
    }
    out
}

fn fit_json(
    id: &str,
    sr: &str,
    pts: &[&Pt],
    files: &[File],
    set: &Set,
) -> (Value, Vec<&'static str>) {
    let head = || Obj::new().put("id", text(id)).put("series", text(sr));
    let used: Vec<&&Pt> = pts.iter().filter(|p| p.y.is_some()).collect();
    let (t, y): (Vec<f64>, Vec<f64>) = used
        .iter()
        .map(|p| (files[p.file].time.unwrap_or(0.0), p.y.unwrap_or(0.0)))
        .unzip();
    let mut times = t.clone();
    times.sort_by(f64::total_cmp);
    times.dedup();
    let forced = used.iter().filter(|p| p.state == "forced").count();
    let missing = pts
        .iter()
        .filter(|p| p.area.is_none() && p.out != Some("no_uv_peak"))
        .count();
    let head = head()
        .i("n", used.len() as i64)
        .i("n_forced", forced as i64)
        .i("n_missing", missing as i64);
    if times.len() < MIN_TIMES || y.iter().all(|v| *v <= 0.0) {
        let why = if times.len() < MIN_TIMES {
            "too_few_times"
        } else {
            "no_signal"
        };
        return (head.put("error", text(why)).done(), vec!["notFitted"]);
    }
    let f = fit_decay(&t, &y, set.boot, set.seed);
    let get = |k: &str| f.get(k).and_then(Value::f64).unwrap_or(f64::NAN);
    let (k, sse) = (get("k"), get("sse"));
    let mean = y.iter().sum::<f64>() / y.len() as f64;
    let sst: f64 = y.iter().map(|v| (v - mean) * (v - mean)).sum();
    let rho = wspearman(&t, &y, None);
    let note = if k > 0.0 {
        ""
    } else if rho >= RISING {
        "rising"
    } else {
        "k_zero"
    };
    let reliable = f.get("reliable").and_then(Value::bool).unwrap_or(false);
    let o = head
        .put("times", Value::Arr(times.iter().map(|v| num(*v)).collect()))
        .n("A", get("A"))
        .n("k", k)
        .put("k_ci", f.get("k_ci").cloned().unwrap_or(Value::Null))
        .n("half_life", get("half_life"))
        .n("sse", sse)
        .put("r2", opt((sst > 0.0).then(|| 1.0 - sse / sst)))
        .n("spearman_time", rho)
        .b("reliable", reliable)
        .put("note", text(note))
        .done();
    let flags = [(k <= 0.0, "noDecay"), (!reliable, "fewPoints")];
    (o, flags.iter().filter(|c| c.0).map(|c| c.1).collect())
}

/// The series with at least one used sample file, in the order of the samples.
fn series_of(files: &[File]) -> Vec<String> {
    let mut series: Vec<String> = Vec::new();
    for f in files
        .iter()
        .filter(|f| f.kind == "sample" && f.out.is_none())
    {
        if !series.contains(&f.series) {
            series.push(f.series.clone());
        }
    }
    series
}

/// The ions to fit: the `ids` parameter, or every id of the areas, in the order of the table.
fn ids_of(areas: &[Area], set: &Set) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for a in areas {
        if !ids.contains(&a.id) && (set.ids.is_empty() || set.ids.contains(&a.id)) {
            ids.push(a.id.clone());
        }
    }
    ids
}

/// The `kinetics-series/1` JSON and the `warn.flow.kinetics.*` keys of the conditions met.
pub fn kinetics_text(
    params: &[(String, Value)],
    areas: &[u8],
    samples: &[u8],
    uv: &[u8],
) -> Result<(String, Vec<String>)> {
    let set = settings(params)?;
    let areas = parse_areas(areas)?;
    let uvp = if uv.iter().all(|b| b.is_ascii_whitespace()) {
        Vec::new()
    } else {
        parse_uv(uv)?
    };
    let mut log: Vec<Choice> = Vec::new();
    let mut files = files_of(samples, &areas, &set, &mut log)?;
    let series = series_of(&files);
    if series.is_empty() {
        return Err(Error::new("err.flow.kinetics.samples")
            .with("detail", "no sample with a time and areas"));
    }
    let uv_ref = uv_factors(&mut files, &uvp, &set, &mut log);
    let ids = ids_of(&areas, &set);
    let pts = points(&files, &areas, &ids, &series, &set);
    let mut flags: Vec<&str> = Vec::new();
    let mut fits = Vec::new();
    for id in &ids {
        for sr in &series {
            let own: Vec<&Pt> = pts
                .iter()
                .filter(|p| p.id == *id && p.series == *sr)
                .collect();
            let (j, flag) = fit_json(id, sr, &own, &files, &set);
            fits.push(j);
            flags.extend(flag);
        }
    }
    let unmatched: Vec<&Area> = areas
        .iter()
        .filter(|a| !files.iter().any(|f| f.stem == a.stem))
        .collect();
    for a in &unmatched {
        choice(
            &mut log,
            "areas_not_in_samples",
            &a.stem,
            a.id.clone(),
            "file not in the samples",
        );
    }
    let warnings = warn_keys(
        &pts,
        &files,
        &flags,
        !unmatched.is_empty(),
        &set,
        &uvp,
        uv_ref,
    );
    Ok((
        document(
            &set,
            &files,
            &pts,
            fits,
            &log,
            uv_json(&set, uv_ref, &files),
            &warnings,
        ),
        warnings,
    ))
}

fn warn_keys(
    pts: &[Pt],
    files: &[File],
    flags: &[&str],
    unmatched: bool,
    set: &Set,
    uvp: &[Uv],
    uv_ref: Option<(f64, f64)>,
) -> Vec<String> {
    let used = |f: &&File| f.out.is_none();
    let off = files.iter().filter(used).any(|f| {
        f.factor
            .is_some_and(|v| (v - 1.0).abs() * 100.0 > set.uv_tol)
    });
    let (t, a): (Vec<f64>, Vec<f64>) = files
        .iter()
        .filter(used)
        .filter_map(|f| f.time.zip(f.uv.map(|u| u.1)))
        .unzip();
    let trend = t.len() >= 4 && wspearman(&t, &a, None).abs() >= UV_TREND;
    let conds = [
        (
            "warn.flow.kinetics.forced",
            pts.iter().any(|p| p.y.is_some() && p.state == "forced"),
        ),
        (
            "warn.flow.kinetics.missing",
            pts.iter()
                .any(|p| p.area.is_none() && p.out.is_none_or(|w| w == "no_peak")),
        ),
        ("warn.flow.kinetics.fewPoints", flags.contains(&"fewPoints")),
        ("warn.flow.kinetics.notFitted", flags.contains(&"notFitted")),
        ("warn.flow.kinetics.noDecay", flags.contains(&"noDecay")),
        ("warn.flow.kinetics.unmatched", unmatched),
        (
            "warn.flow.kinetics.uvMissing",
            uv_ref.is_some()
                && !uvp.is_empty()
                && files.iter().any(|f| {
                    f.out == Some("no_uv_peak") || (f.out.is_none() && f.factor.is_none())
                }),
        ),
        ("warn.flow.kinetics.uvOff", uv_ref.is_some() && off),
        ("warn.flow.kinetics.uvTrend", uv_ref.is_some() && trend),
    ];
    conds
        .iter()
        .filter(|c| c.1)
        .map(|c| c.0.to_string())
        .collect()
}
