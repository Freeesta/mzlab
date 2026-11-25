//! `file_method/1`: the tandem-MS method parameters per scan segment (precursor, collision energy, its spread, declustering potential, fill time, Q1 resolution)
//! read from the `method/1` document of a sample and the `items/1` document of its scans, plus the check of the m/z a sample name carries against the precursors.
//! A value the file does not hold is `null` and `unavailable` says why: `not_in_method` (the method of this format has no such parameter for the segment),
//! `not_in_file` (the format has no field for it), `unknown` (mzLab does not read it for this format and it is not established that the file holds it) or
//! `damaged` (a stream of the method that holds it is damaged and was not read; listed in `damaged`). Pure functions of two JSON texts: site, app and CLI give the same bytes.

use crate::error::Result;
use crate::json::{parse, Out, Value};

pub const SCHEMA: &str = "file_method/1";
/// Message key of the name warning (its text is in the `lang/` catalog).
pub const WARN_NAME_MZ: &str = "warn.file.nameMz";
/// A name mass within this many m/z of a precursor agrees with it (a name rounds the mass, the method holds a unit-resolution value).
pub const NAME_TOL_MZ: f64 = 1.0;

/// The parameters of a segment that can be missing, in the order of the document.
const KEYS: [&str; 5] = ["ce_eV", "ces_eV", "dp_V", "fill_time_ms", "q1_resolution"];

#[derive(Default)]
struct Segment {
    experiment: Option<i64>,
    scan: Option<String>,
    ms_level: Option<i64>,
    name: Option<String>,
    precursor: Option<f64>,
    q3: Option<f64>,
    range: Option<[f64; 2]>,
    ce: Vec<f64>,
    ces: Option<f64>,
    dp: Option<f64>,
    dwell: Option<f64>,
    fill: Option<f64>,
}

fn nums(v: Option<&Value>) -> Vec<f64> {
    v.map(|v| v.arr().iter().filter_map(Value::f64).collect())
        .unwrap_or_default()
}

fn one(v: &[f64]) -> Option<f64> {
    match v {
        [x] => Some(*x),
        _ => None,
    }
}

/// The value of the parameter `id` of an experiment (`parameters[{id, value}]`).
fn param(e: &Value, id: &str) -> Option<f64> {
    e.get("parameters")?
        .arr()
        .iter()
        .find(|p| p.get("id").and_then(Value::str) == Some(id))?
        .get("value")?
        .f64()
}

fn level_of(tag: &str) -> Option<i64> {
    crate::wiff::modes::MODI
        .iter()
        .find(|m| m.tag == tag)
        .map(|m| i64::from(m.level))
}

/// The damaged streams a `method/1` document reports: (experiment index, part).
fn damaged_parts(doc: &Value) -> Vec<(i64, String)> {
    let periods = doc.get("ms").and_then(|m| m.get("periods")).map(Value::arr).unwrap_or_default();
    let mut out = Vec::new();
    for e in periods.iter().flat_map(|p| p.get("experiments").map(Value::arr).unwrap_or_default()) {
        let index = e.get("index").and_then(Value::f64).map_or(-1, |x| x as i64);
        for d in e.get("damaged").map(Value::arr).unwrap_or_default() {
            if let Some(part) = d.str() {
                out.push((index, part.to_string()));
            }
        }
    }
    out
}

/// The segments of a `method/1` document: one per transition of an MRM experiment, one per scan experiment.
fn segments(doc: &Value) -> Vec<Segment> {
    let from_mzml = doc
        .get("origin")
        .and_then(|o| o.get("format"))
        .and_then(Value::str)
        == Some("mzml");
    let mut out = Vec::new();
    let periods = doc
        .get("ms")
        .and_then(|m| m.get("periods"))
        .map(Value::arr)
        .unwrap_or_default();
    for e in periods
        .iter()
        .flat_map(|p| p.get("experiments").map(Value::arr).unwrap_or_default())
    {
        let index = e.get("index").and_then(Value::f64).map(|x| x as i64);
        let transitions = e.get("transitions").map(Value::arr).unwrap_or_default();
        for t in transitions {
            out.push(Segment {
                experiment: index,
                scan: Some("MRM".into()),
                ms_level: Some(2),
                name: t.get("name").and_then(Value::str).map(String::from),
                precursor: t.get("q1").and_then(Value::f64),
                q3: t.get("q3").and_then(Value::f64),
                ce: t.get("ce_eV").and_then(Value::f64).into_iter().collect(),
                dp: t.get("dp_V").and_then(Value::f64),
                dwell: t.get("dwell_ms").and_then(Value::f64),
                ..Default::default()
            });
        }
        if !transitions.is_empty() {
            continue;
        }
        let scan = e.get("scan_type").and_then(Value::str).map(String::from);
        let mut s = Segment {
            experiment: index,
            ms_level: if from_mzml {
                index
            } else {
                scan.as_deref().and_then(level_of)
            },
            precursor: e.get("precursor_mz").and_then(Value::f64),
            range: nums(e.get("mz")).get(..2).map(|r| [r[0], r[1]]),
            ce: nums(e.get("ce_eV")),
            ces: one(&nums(e.get("ces_eV"))),
            dp: one(&nums(e.get("dp_V"))),
            scan,
            ..Default::default()
        };
        if from_mzml {
            s.ce = param(e, "collision_energy").into_iter().collect();
            s.fill = param(e, "ion_injection_time");
        }
        // the collision energy of a survey scan is not applied: only a tandem segment carries one
        if s.ms_level == Some(1) {
            (s.ce, s.ces) = (Vec::new(), None);
        }
        out.push(s);
    }
    out
}

fn write_segment(o: &mut Out, s: &Segment) {
    o.raw("{\"experiment\":");
    match s.experiment {
        Some(i) => o.int(i),
        None => o.raw("null"),
    };
    o.raw(",\"scan\":");
    match &s.scan {
        Some(t) => o.str(t),
        None => o.raw("null"),
    };
    o.raw(",\"ms_level\":");
    match s.ms_level {
        Some(l) => o.int(l),
        None => o.raw("null"),
    };
    o.raw(",\"name\":");
    match &s.name {
        Some(t) => o.str(t),
        None => o.raw("null"),
    };
    o.raw(",\"precursor_mz\":")
        .opt(s.precursor)
        .raw(",\"q3_mz\":")
        .opt(s.q3)
        .raw(",\"mz_range\":");
    match s.range {
        Some(r) => o.floats(&r),
        None => o.raw("null"),
    };
    o.raw(",\"ce_eV\":")
        .opt(one(&s.ce))
        .raw(",\"ce_list_eV\":")
        .floats(&s.ce);
    o.raw(",\"ces_eV\":").opt(s.ces).raw(",\"dp_V\":").opt(s.dp);
    o.raw(",\"dwell_ms\":")
        .opt(s.dwell)
        .raw(",\"fill_time_ms\":")
        .opt(s.fill)
        .raw(",\"q1_resolution\":null}");
}

fn present(s: &Segment, key: &str) -> bool {
    match key {
        "ce_eV" => one(&s.ce).is_some(),
        "ces_eV" => s.ces.is_some(),
        "dp_V" => s.dp.is_some(),
        "fill_time_ms" => s.fill.is_some(),
        _ => false,
    }
}

/// Why a parameter that no tandem segment holds is missing, from the format the method came from. `damaged`: a stream of the method that holds it
/// is damaged, so the absence says nothing about the file. `unknown`: mzLab does not read it for this format and it is not established that the file has it.
fn reason(format: &str, key: &str, damaged: bool) -> &'static str {
    match (format, key) {
        (_, "ce_eV" | "ces_eV" | "dp_V") if damaged => "damaged",
        ("sciex-wiff", "fill_time_ms" | "q1_resolution") => "unknown",
        ("sciex-wiff", _) => "not_in_method",
        _ => "not_in_file",
    }
}

/// A name that carries a mass the precursors do not have: the numbers of the warning, in the same object `file items` and `file method` write.
pub fn write_name_warning(o: &mut Out, name: &str, m: &crate::namemz::Mismatch) {
    o.raw("{\"key\":")
        .str(WARN_NAME_MZ)
        .raw(",\"name\":")
        .str(name)
        .raw(",\"name_mz\":")
        .floats(&m.name_mz);
    o.raw(",\"nearest_name_mz\":")
        .num(m.nearest_name_mz)
        .raw(",\"nearest_precursor\":")
        .num(m.nearest_precursor);
    o.raw(",\"delta_mz\":")
        .num(m.delta)
        .raw(",\"tolerance_mz\":")
        .num(NAME_TOL_MZ)
        .raw("}");
}

/// The warnings of `names` (a file name, a sample name) against `precursors`, one per distinct name that carries a mass the precursors do not hold.
pub fn name_warnings(names: &[&str], precursors: &[f64]) -> Vec<String> {
    let mut seen: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for n in names.iter().copied().filter(|n| !n.is_empty()) {
        if seen.contains(&n) {
            continue;
        }
        seen.push(n);
        if let Some(m) = crate::namemz::mismatch(n, precursors, NAME_TOL_MZ) {
            let mut o = Out::default();
            write_name_warning(&mut o, n, &m);
            out.push(o.0);
        }
    }
    out
}

/// The precursors an `items/1` document holds (the `prec` of every `ms2_exps` row), sorted and distinct.
pub fn item_precursors(items: &Value) -> Vec<f64> {
    let mut v: Vec<f64> = items
        .get("items")
        .map(Value::arr)
        .unwrap_or_default()
        .iter()
        .filter_map(|i| i.get("info")?.get("ms2_exps"))
        .flat_map(|x| x.arr().iter().filter_map(|e| e.get("prec")?.f64()))
        .collect();
    crate::sortc::by(&mut v, f64::total_cmp);
    v.dedup();
    v
}

fn nullable(o: &mut Out, text: &str) {
    if text.is_empty() {
        o.raw("null");
    } else {
        o.str(text);
    }
}

/// What the scans hold: the precursors the data were acquired with (a dependent scan picks its own; the method holds a fixed one at most).
fn write_acquired(o: &mut Out, items: &Value) {
    let mut first = true;
    for item in items.get("items").map(Value::arr).unwrap_or_default() {
        let Some(info) = item.get("info") else {
            continue;
        };
        for e in info.get("ms2_exps").map(Value::arr).unwrap_or_default() {
            o.raw(if first { "" } else { "," });
            first = false;
            o.raw("{\"precursor_mz\":")
                .opt(e.get("prec").and_then(Value::f64))
                .raw(",\"ce_eV\":")
                .opt(e.get("ce").and_then(Value::f64));
            o.raw(",\"scans\":")
                .opt(e.get("n").and_then(Value::f64))
                .raw("}");
        }
    }
}

/// The parameters no tandem segment holds, each with the reason.
fn write_unavailable(o: &mut Out, segs: &[Segment], format: &str, damaged: bool) {
    let tandem: Vec<&Segment> = segs.iter().filter(|s| s.ms_level != Some(1)).collect();
    let mut sep = "";
    for key in KEYS {
        if !tandem.iter().any(|s| present(s, key)) {
            o.raw(sep).str(key).raw(":").str(reason(format, key, damaged));
            sep = ",";
        }
    }
}

/// The precursors a sample name is checked against: the fixed ones of the method and the ones of the scans, sorted and distinct. `file method` and
/// `file items` both use this set, so they warn about the same names.
fn name_precursors(segs: &[Segment], items: &Value) -> Vec<f64> {
    let mut precs: Vec<f64> = segs.iter().filter_map(|s| s.precursor).filter(|p| *p > 0.0).collect();
    precs.extend(item_precursors(items));
    crate::sortc::by(&mut precs, f64::total_cmp);
    precs.dedup();
    precs
}

/// The precursors of a `method/1` text (`None`: no method) and an `items/1` document for the name check of `file items`.
pub fn precursors_of(method: Option<&str>, items: &Value) -> Vec<f64> {
    let segs = method.and_then(|m| parse(m.as_bytes()).ok()).map(|d| segments(&d)).unwrap_or_default();
    name_precursors(&segs, items)
}

/// The `file_method/1` document of sample `sample` (`None` for an mzML) from its `method/1` text (`None`: the file carries none) and its `items/1` text.
pub fn document(sample: Option<u32>, method: Option<&str>, items: &str) -> Result<String> {
    let items = parse(items.as_bytes())?;
    let doc = match method {
        Some(m) => Some(parse(m.as_bytes())?),
        None => None,
    };
    let segs = doc.as_ref().map(segments).unwrap_or_default();
    let bad = doc.as_ref().map(damaged_parts).unwrap_or_default();
    let format = doc
        .as_ref()
        .and_then(|d| d.get("origin")?.get("format")?.str())
        .unwrap_or("");
    let sample_name = doc
        .as_ref()
        .and_then(|d| d.get("sample")?.get("name")?.str())
        .unwrap_or("");
    let file = items
        .get("items")
        .and_then(|i| i.at(0))
        .and_then(|i| i.get("info")?.get("file")?.str())
        .unwrap_or("");
    // the part tag of an item (`x.wiff#MS1`) is not part of the file name
    let file = file.split('#').next().unwrap_or(file);
    let mut o = Out::default();
    o.raw("{\"schema\":")
        .str(SCHEMA)
        .raw(",\"file\":")
        .str(file)
        .raw(",\"sample\":");
    match sample {
        Some(id) => o.int(i64::from(id)),
        None => o.raw("null"),
    };
    o.raw(",\"format\":");
    nullable(&mut o, format);
    o.raw(",\"sample_name\":");
    nullable(&mut o, sample_name);
    o.raw(",\"segments\":[");
    for (i, s) in segs.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        write_segment(&mut o, s);
    }
    o.raw("],\"acquired\":[");
    write_acquired(&mut o, &items);
    o.raw("],\"damaged\":[");
    for (i, (index, part)) in bad.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"experiment\":").int(*index);
        o.raw(",\"part\":").str(part).raw("}");
    }
    o.raw("],\"unavailable\":{");
    write_unavailable(&mut o, &segs, format, !bad.is_empty());
    let precs = name_precursors(&segs, &items);
    o.raw("},\"warnings\":[")
        .raw(&name_warnings(&[file, sample_name], &precs).join(","))
        .raw("]}");
    Ok(o.0)
}

#[cfg(test)]
#[path = "filemethod_tests.rs"]
mod tests;
