//! The JSON of `step/1`: the keys and their order follow `docs/studi/workflow/step.features.detect.json`; `registry_json` writes one step per line.

use super::{
    Algorithm, Applicability, Facets, Kind, Param, Port, Preset, Reference, RunsIn, Status, Step,
};
use crate::json::{Out, Value};

fn list<T>(o: &mut Out, items: &[T], mut each: impl FnMut(&mut Out, &T)) {
    o.raw("[");
    for (i, x) in items.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        each(o, x);
    }
    o.raw("]");
}

/// `,"key":` (every key but the first of an object).
fn key(o: &mut Out, k: &str) {
    o.raw(",").str(k).raw(":");
}

fn value(o: &mut Out, v: &Value) {
    match v {
        Value::Null => {
            o.raw("null");
        }
        Value::Bool(b) => {
            o.raw(if *b { "true" } else { "false" });
        }
        Value::Num(n) => {
            o.num(*n);
        }
        Value::Str(s) => {
            o.str(s);
        }
        Value::Arr(a) => list(o, a, value),
        Value::Obj(m) => {
            o.raw("{");
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    o.raw(",");
                }
                o.str(k).raw(":");
                value(o, x);
            }
            o.raw("}");
        }
    }
}

fn strs(o: &mut Out, a: &[&str]) {
    list(o, a, |o, s| {
        o.str(s);
    });
}

fn facets(o: &mut Out, f: &Facets) {
    o.raw("{");
    let mut first = true;
    let mut open = |o: &mut Out, k: &str| {
        if !first {
            o.raw(",");
        }
        first = false;
        o.str(k).raw(":");
    };
    if !f.resolution.is_empty() {
        open(o, "resolution");
        strs(o, &f.resolution);
    }
    if !f.polarity.is_empty() {
        open(o, "polarity");
        list(o, &f.polarity, |o, p| {
            o.int(i64::from(*p));
        });
    }
    if !f.ms_levels.is_empty() {
        open(o, "ms_levels");
        list(o, &f.ms_levels, |o, p| {
            o.int(i64::from(*p));
        });
    }
    o.raw("}");
}

fn port(o: &mut Out, p: &Port) {
    o.str("type").raw(":").str(p.ty.name());
    if p.many {
        key(o, "many");
        o.raw("true");
    }
    if p.optional {
        key(o, "optional");
        o.raw("true");
    }
    key(o, "title");
    o.str(p.title);
    if !p.accepts.is_empty() {
        key(o, "accepts");
        list(o, &p.accepts, |o, t| {
            o.str(t.name());
        });
    }
    if p.requires != Facets::default() {
        key(o, "requires");
        facets(o, &p.requires);
    }
}

fn ports(o: &mut Out, ps: &[Port]) {
    o.raw("{");
    for (i, p) in ps.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(p.name).raw(":{");
        port(o, p);
        o.raw("}");
    }
    o.raw("}");
}

fn param(o: &mut Out, p: &Param) {
    o.str("title").raw(":").str(p.title);
    match &p.kind {
        Kind::Number | Kind::Integer | Kind::Bool => {
            key(o, "type");
            o.str(match p.kind {
                Kind::Number => "number",
                Kind::Integer => "integer",
                _ => "boolean",
            });
        }
        Kind::Pair(min) => {
            o.raw(",\"type\":\"array\",\"items\":{\"type\":\"number\",\"minimum\":")
                .num(*min);
            o.raw("},\"minItems\":2,\"maxItems\":2");
        }
        Kind::Choice(v) => {
            key(o, "enum");
            list(o, v, value);
        }
    }
    if let Some(v) = p.min {
        key(o, "minimum");
        o.num(v);
    }
    if let Some(v) = p.max {
        key(o, "maximum");
        o.num(v);
    }
    if let Some(v) = &p.default {
        key(o, "default");
        value(o, v);
    }
    if let Some(u) = p.unit {
        key(o, "x-unit");
        o.str(u);
    }
    for (on, k) in [
        (p.estimate, "x-estimate"),
        (p.noresult, "x-noresult"),
        (p.advanced, "x-advanced"),
    ] {
        if on {
            key(o, k);
            o.raw("true");
        }
    }
    if let Some(d) = p.doc {
        key(o, "description");
        o.str(d);
    }
}

fn params(o: &mut Out, ps: &[Param]) {
    o.raw("{\"type\":\"object\",\"properties\":{");
    for (i, p) in ps.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(p.name).raw(":{");
        param(o, p);
        o.raw("}");
    }
    o.raw("}}");
}

fn references(o: &mut Out, rs: &[Reference]) {
    list(o, rs, |o, r| {
        o.raw("{\"doi\":").str(r.doi);
        key(o, "citation_key");
        o.str(r.citation_key);
        key(o, "note");
        o.str(r.note).raw("}");
    });
}

fn applicability(o: &mut Out, a: &Applicability) {
    o.raw("{");
    let mut first = true;
    let mut open = |o: &mut Out, k: &str| {
        if !first {
            o.raw(",");
        }
        first = false;
        o.str(k).raw(":");
    };
    if !a.resolution.is_empty() {
        open(o, "resolution");
        strs(o, &a.resolution);
    }
    if !a.spectrum_mode.is_empty() {
        open(o, "spectrum_mode");
        strs(o, &a.spectrum_mode);
    }
    if let Some((lo, hi)) = a.peak_fwhm_s {
        open(o, "peak_fwhm_s");
        o.floats(&[lo, hi]);
    }
    if !a.note.is_empty() {
        open(o, "note");
        o.str(a.note);
    }
    o.raw("}");
}

fn algorithm(o: &mut Out, a: &Algorithm) {
    o.raw("{\"id\":").str(a.id);
    key(o, "version");
    o.int(i64::from(a.version));
    key(o, "title");
    o.str(a.title);
    key(o, "doc");
    o.str(a.doc);
    if !a.references.is_empty() {
        key(o, "references");
        references(o, &a.references);
    }
    key(o, "applicability");
    applicability(o, &a.applicability);
    key(o, "params");
    params(o, &a.params);
    key(o, "status");
    o.str(match a.status {
        Status::Available => "available",
        Status::Planned => "planned",
    })
    .raw("}");
}

fn preset(o: &mut Out, p: &Preset) {
    o.raw("{\"title\":").str(p.title);
    key(o, "when");
    facets(o, &p.when);
    key(o, "params");
    value(
        o,
        &Value::Obj(
            p.params
                .iter()
                .map(|(k, v)| ((*k).to_string(), v.clone()))
                .collect(),
        ),
    );
    key(o, "evidence");
    o.str(p.evidence);
    if let Some(a) = p.algorithm {
        key(o, "algorithm");
        o.str(a);
    }
    o.raw("}");
}

fn runs_in(o: &mut Out, r: &RunsIn) {
    o.raw("{");
    if let Some(w) = r.wasm {
        o.raw("\"wasm\":").str(w).raw(",");
    }
    if let Some(c) = r.wasm_ceiling_bytes {
        o.raw("\"wasm_ceiling_bytes\":").int(c as i64).raw(",");
    }
    o.raw("\"native\":")
        .raw(if r.native { "true" } else { "false" })
        .raw("}");
}

/// The `step/1` JSON of one step.
pub fn step_json(s: &Step) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"step/1\",\"id\":").str(s.id);
    key(&mut o, "version");
    o.int(i64::from(s.version));
    key(&mut o, "title");
    o.str(s.title);
    key(&mut o, "doc");
    o.str(s.doc);
    if let Some(cv) = s.cv {
        key(&mut o, "cv");
        o.str(cv);
    }
    key(&mut o, "package");
    o.str(s.package);
    if !s.references.is_empty() {
        key(&mut o, "references");
        references(&mut o, &s.references);
    }
    key(&mut o, "in");
    ports(&mut o, &s.ins);
    key(&mut o, "out");
    ports(&mut o, &s.outs);
    key(&mut o, "params");
    params(&mut o, &s.params);
    key(&mut o, "runs_in");
    runs_in(&mut o, &s.runs_in);
    key(&mut o, "deterministic");
    o.raw(if s.deterministic { "true" } else { "false" });
    key(&mut o, "errors");
    strs(&mut o, &s.errors);
    tail(&mut o, s);
    o.raw("}");
    o.0
}

/// The optional keys after `errors`: algorithms, defaults, per-item, conversion, cost, memory, presets.
fn tail(o: &mut Out, s: &Step) {
    if !s.algorithms.is_empty() {
        key(o, "algorithms");
        list(o, &s.algorithms, algorithm);
    }
    if let Some(a) = s.default_algorithm {
        key(o, "default_algorithm");
        o.str(a);
    }
    if s.per_item {
        key(o, "per_item");
        o.raw("true");
    }
    if let Some((from, to)) = s.converts {
        key(o, "converts");
        o.raw("{\"from\":")
            .str(from.name())
            .raw(",\"to\":")
            .str(to.name())
            .raw("}");
    }
    if let Some(c) = s.cost {
        key(o, "cost");
        o.num(c);
    }
    if let Some(m) = s.memory_hint_bytes {
        key(o, "memory_hint_bytes");
        o.int(m as i64);
    }
    if !s.presets.is_empty() {
        key(o, "presets");
        o.raw("{");
        for (i, p) in s.presets.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.str(p.id).raw(":");
            preset(o, p);
        }
        o.raw("}");
    }
    if let Some(d) = s.default_preset {
        key(o, "default_preset");
        o.str(d);
    }
}

/// A JSON array with one step per line.
pub fn registry_json(steps: &[Step]) -> String {
    let lines: Vec<String> = steps.iter().map(step_json).collect();
    format!("[\n{}\n]\n", lines.join(",\n"))
}
