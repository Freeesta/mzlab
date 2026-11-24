//! `estimate/1`: the `estimates/1` of several files put together (AU-6). Per value (noise floor, minimum height, m/z tolerance, RT tolerance, ...) the
//! median over the files with the figures it comes from, and the parameters of a plan they would set (`--set` lines), each with its evidence and the
//! value the plan holds now. Nothing is applied here: the numbers are proposals, the agent or the user passes them. The CLI (`mzlab-cli estimate`), the app
//! route and the site's `.wasm` all call `route`, so the three print the same bytes.

use crate::error::{Error, Result};
use crate::flow::value_text;
use crate::json::{self, Value};
use crate::jsonb::{int, msg, num, opt, text, write, Obj};

/// Fewer files than this: the median rests on little data and a note says so (nothing is refused).
pub const FEW_FILES: usize = 3;
/// Significant figures of a proposed value in a `--set` line (the median is kept whole in the evidence).
const FIGURES: i32 = 4;

/// One measured value: its name, where it sits in `estimates/1`, where its count of points sits and its unit.
struct Measure {
    name: &'static str,
    path: &'static [&'static str],
    points: Option<&'static str>,
    unit: &'static str,
}

const MEASURES: &[Measure] = &[
    Measure {
        name: "noise_floor",
        path: &["noise", "floor"],
        points: None,
        unit: "counts",
    },
    Measure {
        name: "min_height",
        path: &["min_height", "value"],
        points: None,
        unit: "counts",
    },
    Measure {
        name: "ppm",
        path: &["ppm", "value"],
        points: Some("ppm"),
        unit: "ppm",
    },
    Measure {
        name: "da",
        path: &["da", "value"],
        points: Some("da"),
        unit: "Da",
    },
    Measure {
        name: "fwhm",
        path: &["fwhm", "value"],
        points: Some("fwhm"),
        unit: "min",
    },
    Measure {
        name: "rt_tol",
        path: &["rt_tol"],
        points: None,
        unit: "min",
    },
    Measure {
        name: "dead",
        path: &["dead"],
        points: None,
        unit: "min",
    },
    Measure {
        name: "defect",
        path: &["defect"],
        points: None,
        unit: "Da",
    },
];

/// A parameter of a step a measure can set: the measure, the step id, the parameter and the axis it needs (`""` any).
struct Target {
    measure: &'static str,
    step: &'static str,
    param: &'static str,
    axis: &'static str,
}

/// The parameters the measures map to (each exists in the descriptor of its step; a test of the CLI walks the registry).
const TARGETS: &[Target] = &[
    Target {
        measure: "noise_floor",
        step: "features.detect",
        param: "floor",
        axis: "",
    },
    Target {
        measure: "min_height",
        step: "features.detect",
        param: "min_height",
        axis: "",
    },
    Target {
        measure: "defect",
        step: "features.detect",
        param: "mass_defect",
        axis: "Da",
    },
    Target {
        measure: "noise_floor",
        step: "features.gapfill",
        param: "floor",
        axis: "",
    },
    Target {
        measure: "ppm",
        step: "features.align",
        param: "ppm",
        axis: "ppm",
    },
    Target {
        measure: "ppm",
        step: "features.gapfill",
        param: "ppm",
        axis: "ppm",
    },
    Target {
        measure: "da",
        step: "features.align",
        param: "tol_da",
        axis: "Da",
    },
    Target {
        measure: "da",
        step: "features.gapfill",
        param: "tol_da",
        axis: "Da",
    },
    Target {
        measure: "rt_tol",
        step: "features.gapfill",
        param: "rt_tol",
        axis: "",
    },
];

/// The `(step, param)` pairs the proposals can set, for the test that checks them against the registry.
pub fn targets() -> Vec<(&'static str, &'static str)> {
    TARGETS.iter().map(|t| (t.step, t.param)).collect()
}

/// What one file contributes: its name, its `estimates/1` (`None` when it has an error), its polarity and axis.
struct File<'a> {
    name: String,
    doc: Option<&'a Value>,
    error: bool,
}

fn pick(doc: &Value, path: &[&str]) -> Option<f64> {
    let mut v = doc;
    for k in path {
        v = v.get(k)?;
    }
    v.f64().filter(|x| x.is_finite())
}

fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

/// `x` to `FIGURES` significant figures.
fn rounded(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let scale = libm::pow(
        10.0,
        FIGURES as f64 - 1.0 - libm::floor(libm::log10(x.abs())),
    );
    libm::round(x * scale) / scale
}

/// The last part of a step id, the instance name of `flow steps --template`.
fn instance_of(step: &str) -> &str {
    step.rsplit('.').next().unwrap_or(step)
}

/// The `(instance id, step id, params)` of the steps of a plan.
fn plan_steps(plan: &Value) -> Vec<(String, String, Option<Value>)> {
    plan.get("steps")
        .map_or(&[][..], Value::arr)
        .iter()
        .filter_map(|s| {
            let id = s.get("id").and_then(Value::str)?;
            let step = s.get("step").and_then(Value::str)?;
            let base = step.split('@').next().unwrap_or(step);
            Some((id.to_string(), base.to_string(), s.get("params").cloned()))
        })
        .collect()
}

/// The group of files the values are taken from: the most frequent `(polarity, axis)` among the files that have an estimate (ties: the first seen).
fn majority(files: &[File]) -> Option<(f64, String)> {
    let mut seen: Vec<((f64, String), usize)> = Vec::new();
    for f in files.iter().filter(|f| f.doc.is_some()) {
        let doc = f.doc?;
        let key = key_of(doc);
        match seen.iter_mut().find(|(k, _)| *k == key) {
            Some(s) => s.1 += 1,
            None => seen.push((key, 1)),
        }
    }
    let mut best: Option<((f64, String), usize)> = None;
    for s in seen {
        if best.as_ref().is_none_or(|b| s.1 > b.1) {
            best = Some(s);
        }
    }
    best.map(|b| b.0)
}

/// The `(polarity, axis)` a file decided.
fn key_of(doc: &Value) -> (f64, String) {
    (
        doc.get("polarity").and_then(Value::f64).unwrap_or(0.0),
        doc.get("axis")
            .and_then(Value::str)
            .unwrap_or("")
            .to_string(),
    )
}

/// The files of the group, the `left_out` entries of the others and how many files have no estimate.
fn split_group<'a>(
    files: &'a [File],
    group: Option<&(f64, String)>,
) -> (Vec<&'a File<'a>>, Vec<Value>, i64) {
    let (mut taken, mut left_out, mut no_estimate) = (Vec::new(), Vec::new(), 0);
    for f in files {
        let Some(doc) = f.doc else {
            no_estimate += 1;
            continue;
        };
        let key = key_of(doc);
        if Some(&key) == group {
            taken.push(f);
        } else {
            left_out.push(
                Obj::new()
                    .put("file", text(&f.name))
                    .n("polarity", key.0)
                    .put("axis", text(&key.1))
                    .done(),
            );
        }
    }
    (taken, left_out, no_estimate)
}

/// A measure over the files taken: its entry of `values` and, when some file has it, `(measure, median, sorted values, points)`.
type Found<'a> = (&'a Measure, f64, Vec<f64>, i64);

fn value_of<'a>(m: &'a Measure, taken: &[&File]) -> (Value, Option<Found<'a>>) {
    let (mut per_file, mut sorted, mut points) = (Vec::new(), Vec::new(), 0i64);
    for f in taken {
        let Some(doc) = f.doc else { continue };
        let x = pick(doc, m.path);
        let n = m.points.and_then(|p| pick(doc, &[p, "n"]));
        points += n.unwrap_or(0.0) as i64;
        sorted.extend(x);
        per_file.push(
            Obj::new()
                .put("file", text(&f.name))
                .put("value", opt(x))
                .put("points", opt(n))
                .done(),
        );
    }
    crate::sortc::by(&mut sorted, |a, b| a.total_cmp(b));
    let o = Obj::new()
        .put("name", text(m.name))
        .put("unit", text(m.unit));
    let pts = if m.points.is_some() {
        int(points)
    } else {
        Value::Null
    };
    if sorted.is_empty() {
        let o = o
            .i("files", 0)
            .put("median", Value::Null)
            .put("min", Value::Null)
            .put("max", Value::Null)
            .put("points", Value::Null);
        return (o.put("per_file", Value::Arr(per_file)).done(), None);
    }
    let med = median(&sorted);
    let o = o
        .i("files", sorted.len() as i64)
        .n("median", med)
        .n("min", sorted[0])
        .n("max", sorted[sorted.len() - 1])
        .put("points", pts);
    (
        o.put("per_file", Value::Arr(per_file)).done(),
        Some((m, med, sorted, points)),
    )
}

/// One `params` entry: the plan parameter `id.param` the measure sets, with its evidence.
fn proposal(t: &Target, f: &Found, id: &str, current: Option<Value>) -> Value {
    let (m, med, sorted, points) = f;
    let proposed = rounded(*med);
    Obj::new()
        .put(
            "set",
            text(&format!("{id}.{}={}", t.param, value_text(&num(proposed)))),
        )
        .put("instance", text(id))
        .put("step", text(t.step))
        .put("param", text(t.param))
        .put("value", num(proposed))
        .put("unit", text(m.unit))
        .put("current", current.unwrap_or(Value::Null))
        .put("from", text(m.name))
        .put(
            "rule",
            text("median over the files, to 4 significant figures; the evidence is under values"),
        )
        .i("files", sorted.len() as i64)
        .n("min", sorted[0])
        .n("max", sorted[sorted.len() - 1])
        .put(
            "points",
            if m.points.is_some() {
                int(*points)
            } else {
                Value::Null
            },
        )
        .done()
}

/// The plan parameters the measures set (`params`) and those left alone with the reason (`skipped`).
fn proposals(found: &[Found], plan: Option<&Value>, axis: &str) -> (Vec<Value>, Vec<Value>) {
    let (mut params, mut skipped) = (Vec::new(), Vec::new());
    let steps = plan.map(plan_steps);
    for t in TARGETS {
        if !t.axis.is_empty() && t.axis != axis {
            skipped.push(skip(t, "axis"));
            continue;
        }
        let Some(f) = found.iter().find(|f| f.0.name == t.measure) else {
            skipped.push(skip(t, "noValue"));
            continue;
        };
        // each instance of the step in the plan, or the default instance name when no plan was given
        let instances: Vec<(String, Option<Value>)> = match &steps {
            Some(all) => all
                .iter()
                .filter(|(_, s, _)| s == t.step)
                .map(|(id, _, p)| (id.clone(), p.clone()))
                .collect(),
            None => vec![(instance_of(t.step).to_string(), None)],
        };
        if instances.is_empty() {
            skipped.push(skip(t, "noInstance"));
        }
        for (id, current) in instances {
            let now = current.as_ref().and_then(|p| p.get(t.param)).cloned();
            params.push(proposal(t, f, &id, now));
        }
    }
    (params, skipped)
}

/// The `choices` and `notes` of a run whose files do not all agree or are few.
fn choices_notes(left_out: usize, taken: usize, gp: f64, ga: &str) -> (Vec<Value>, Vec<Value>) {
    let (mut choices, mut notes) = (Vec::new(), Vec::new());
    if left_out > 0 {
        choices.push(
            Obj::new()
                .put("what", text("group"))
                .put("value", Obj::new().n("polarity", gp).put("axis", text(ga)).done())
                .put(
                    "why",
                    text("the polarity and axis most files share; the others are in left_out and count in none of the values"),
                )
                .done(),
        );
        notes.push(msg(
            "warn.estimate.mixed",
            &[("left_out", int(left_out as i64))],
        ));
    }
    if taken < FEW_FILES {
        notes.push(msg(
            "note.estimate.fewFiles",
            &[("files", int(taken as i64))],
        ));
    }
    (choices, notes)
}

/// `estimate/1` of `{"files": [estimates/1 object or {"file", "error"} ...], "plan": mzflow/2 object or null}`.
pub fn route(body: &str) -> Result<String> {
    let v = json::parse(body.as_bytes())?;
    let list = match v.get("files") {
        Some(Value::Arr(a)) if !a.is_empty() => a,
        _ => {
            return Err(Error::new("err.flow.shape")
                .with("path", "files")
                .with("expected", "a non-empty array"))
        }
    };
    let files: Vec<File> = list
        .iter()
        .map(|e| File {
            name: e.get("file").and_then(Value::str).unwrap_or("").to_string(),
            doc: e.get("schema").map(|_| e),
            error: e.get("error").is_some(),
        })
        .collect();
    let group = majority(&files);
    let (taken, left_out, no_estimate) = split_group(&files, group.as_ref());
    let (gp, ga) = group.unwrap_or((0.0, String::new()));
    let (choices, notes) = choices_notes(left_out.len(), taken.len(), gp, &ga);
    let (mut values, mut found) = (Vec::new(), Vec::new());
    for m in MEASURES {
        let (entry, f) = value_of(m, &taken);
        values.push(entry);
        found.extend(f);
    }
    let plan = v.get("plan").filter(|p| !p.is_null());
    let (params, skipped) = proposals(&found, plan, &ga);
    let summary = Obj::new()
        .i("files", files.len() as i64)
        .i("estimated", taken.len() as i64)
        .i("no_estimate", no_estimate)
        .i("errors", files.iter().filter(|f| f.error).count() as i64)
        .n("polarity", gp)
        .put("axis", text(&ga))
        .done();
    let doc = Obj::new()
        .put("schema", text("estimate/1"))
        .put("summary", summary)
        .put("files", Value::Arr(list.clone()))
        .put("values", Value::Arr(values))
        .put("params", Value::Arr(params))
        .put("skipped", Value::Arr(skipped))
        .put("left_out", Value::Arr(left_out))
        .put("choices", Value::Arr(choices))
        .put("notes", Value::Arr(notes))
        .done();
    Ok(write(&doc) + "\n")
}

/// A parameter not proposed and why: no value measured, the other axis, or no instance of its step in the plan.
fn skip(t: &Target, why: &str) -> Value {
    Obj::new()
        .put("step", text(t.step))
        .put("param", text(t.param))
        .put("from", text(t.measure))
        .put("reason", text(why))
        .done()
}
