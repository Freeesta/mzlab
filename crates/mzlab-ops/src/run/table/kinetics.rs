//! The text of `quant.kinetics@1`: the areas of a time series (fixed RT windows, from `quant.targets`) joined to the times of `samples/1`, divided by the
//! UV area of a reference peak per file when a UV table is wired (the injection check), and a first-order decay `y = A exp(-k t)` fitted per ion and
//! series with its bootstrap interval (`mzlab_core::kinetics::fit_decay`). Every choice is logged; nothing is left out silently. Bytes in, text out:
//! `flow run` (`mzlab-cli`) and the browser module `flowkin` both call it, so they write the same bytes.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;
use mzlab_core::jsonb::{num, opt, text, write, Obj};
use mzlab_core::kinetics::fit_decay;
use mzlab_core::stats::wspearman;
use mzlab_core::tracestat::percentile;
use mzlab_feat::samples::samples_of;

use super::kinetics_read::{parse_areas, parse_uv, settings, stem, Area, Set, Uv};

/// Fewest distinct times a decay is fitted on; the engine calls a fit reliable from five points.
const MIN_TIMES: usize = 3;
/// Spearman rho of a flat-fitted series from which it is called rising (a formation, not a decay).
const RISING: f64 = 0.8;
/// |Spearman rho| of the UV area against time from which the reference looks like it reacts, not like an injection check.
const UV_TREND: f64 = 0.8;

/// One file of the series: its role, series, time and what the UV reference says.
struct File {
    stem: String,
    name: String,
    kind: String,
    series: String,
    time: Option<f64>,
    uv: Option<(f64, f64)>,
    factor: Option<f64>,
    /// `None` = its areas enter the fits, else why not.
    out: Option<&'static str>,
}

/// An automatic choice, written in the log.
struct Choice(String, String, String, String);

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
struct Pt {
    id: String,
    series: String,
    file: usize,
    area: Option<f64>,
    state: String,
    y: Option<f64>,
    out: Option<&'static str>,
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

fn uv_json(set: &Set, uv: Option<(f64, f64)>, files: &[File]) -> Value {
    let Some((rt, reference)) = uv else {
        return Obj::new().b("wired", false).done();
    };
    let used: Vec<&File> = files
        .iter()
        .filter(|f| f.uv.is_some() && f.out.is_none())
        .collect();
    let (t, a): (Vec<f64>, Vec<f64>) = used
        .iter()
        .filter_map(|f| f.time.zip(f.uv.map(|u| u.1)))
        .unzip();
    let rho = (t.len() >= 4).then(|| wspearman(&t, &a, None));
    Obj::new()
        .b("wired", true)
        .put(
            "mode",
            text(if set.normalise { "normalise" } else { "check" }),
        )
        .n("rt_min", rt)
        .n("rt_tol_min", set.uv_rt_tol)
        .n("tol_pct", set.uv_tol)
        .n("reference_area", reference)
        .put("time_spearman", opt(rho))
        .done()
}

fn strs(v: &[&str]) -> Value {
    Value::Arr(v.iter().map(|s| text(s)).collect())
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
    let mut series: Vec<String> = Vec::new();
    for f in files
        .iter()
        .filter(|f| f.kind == "sample" && f.out.is_none())
    {
        if !series.contains(&f.series) {
            series.push(f.series.clone());
        }
    }
    if series.is_empty() {
        return Err(Error::new("err.flow.kinetics.samples")
            .with("detail", "no sample with a time and areas"));
    }
    let uv_ref = uv_factors(&mut files, &uvp, &set, &mut log);
    let mut ids: Vec<String> = Vec::new();
    for a in &areas {
        if !ids.contains(&a.id) && (set.ids.is_empty() || set.ids.contains(&a.id)) {
            ids.push(a.id.clone());
        }
    }
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

fn document(
    set: &Set,
    files: &[File],
    pts: &[Pt],
    fits: Vec<Value>,
    log: &[Choice],
    uv: Value,
    warnings: &[String],
) -> String {
    let params = Obj::new()
        .put(
            "ids",
            strs(&set.ids.iter().map(String::as_str).collect::<Vec<_>>()),
        )
        .b("use_t0", set.use_t0)
        .put(
            "missing",
            text(if set.zero_missing { "zero" } else { "skip" }),
        )
        .put(
            "uv_mode",
            text(if set.normalise { "normalise" } else { "check" }),
        )
        .n("uv_rt", set.uv_rt)
        .n("uv_rt_tol", set.uv_rt_tol)
        .n("uv_tol", set.uv_tol)
        .i("boot", set.boot as i64)
        .i("seed", set.seed as i64)
        .done();
    let file_rows = files.iter().map(|f| {
        let flag = f
            .factor
            .is_some_and(|v| (v - 1.0).abs() * 100.0 > set.uv_tol);
        Obj::new()
            .put("file", text(&f.name))
            .put("kind", text(&f.kind))
            .put("series", text(&f.series))
            .put("time_min", opt(f.time))
            .put("uv_rt_min", opt(f.uv.map(|u| u.0)))
            .put("uv_area", opt(f.uv.map(|u| u.1)))
            .put("factor", opt(f.factor))
            .b("uv_off", flag)
            .b("used", f.out.is_none())
            .put("reason", f.out.map_or(Value::Null, text))
            .done()
    });
    let point_rows = pts.iter().map(|p| {
        let factor = files[p.file].factor;
        Obj::new()
            .put("id", text(&p.id))
            .put("series", text(&p.series))
            .put("file", text(&files[p.file].name))
            .put("time_min", opt(files[p.file].time))
            .put("area", opt(p.area))
            .put("factor", opt(factor))
            .put("value", opt(p.y))
            .put("state", text(&p.state))
            .b("used", p.y.is_some())
            .put("reason", p.out.map_or(Value::Null, text))
            .done()
    });
    let choices = log.iter().map(|c| {
        Obj::new()
            .put("key", text(&c.0))
            .put("file", text(&c.1))
            .put("value", text(&c.2))
            .put("reason", text(&c.3))
            .done()
    });
    let o = Obj::new()
        .put("format", text("kinetics-series/1"))
        .put("params", params)
        .put("uv", uv)
        .put("files", Value::Arr(file_rows.collect()))
        .put("points", Value::Arr(point_rows.collect()))
        .put("fits", Value::Arr(fits))
        .put("choices", Value::Arr(choices.collect()))
        .put(
            "warnings",
            Value::Arr(warnings.iter().map(|w| text(w)).collect()),
        )
        .done();
    write(&o) + "\n"
}
