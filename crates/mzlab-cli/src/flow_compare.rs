//! `meta.compare@1` on files: a plan that wraps a step in `meta.compare` is expanded before the check into one instance per variant (the wrapped step with
//! the algorithm, preset and parameters of the variant, on the same inputs) and a compare instance that reads their outputs. `compare_text` measures how the
//! outputs differ (`comparison/1`). It only reports the differences; it never says which variant is right.

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Instance, Plan, Ref};
use mzlab_core::json::{parse, round, Out, Value};
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

/// The descriptor of the compare instance an expanded plan holds: it reads the output of every variant, in the order of `labels`.
pub fn compare_step() -> Step {
    Step {
        ins: vec![Port::new("variants", DataType::Candidates, "Output of every variant")
            .many()
            .accepts(&[DataType::Annot])],
        outs: vec![Port::new("comparison", DataType::Comparison, "Differences between variants")],
        params: vec![
            Param::text("step", "Wrapped step"),
            Param::text("labels", "Labels of the variants, comma separated, in the order of the inputs"),
        ],
        runs_in: RunsIn {
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.flow.compare.sameVariant", "err.flow.compare.variants", "err.flow.compare.type"],
        ..Step::core(
            "meta.compare",
            "Compare algorithms",
            "Runs the wrapped step once per variant on the same inputs and measures how the outputs differ; it does not say which variant is right.",
        )
    }
}

/// One variant of the `variants` parameter.
struct Variant {
    label: String,
    algorithm: String,
    preset: Option<String>,
    /// Sorted by name, so that two spellings of the same variant are equal.
    params: Vec<(String, Value)>,
}

fn variant(v: &Value) -> Result<Variant> {
    let text = |k: &str| v.get(k).and_then(Value::str).map(str::to_string);
    let algorithm = text("algorithm").ok_or_else(|| Error::new("err.flow.compare.variants"))?;
    let mut params = match v.get("params") {
        Some(Value::Obj(o)) => o.clone(),
        _ => Vec::new(),
    };
    params.sort_by(|a, b| a.0.cmp(&b.0));
    let label = text("label").unwrap_or_else(|| algorithm.clone());
    if label.is_empty() || label.contains(',') {
        return Err(Error::new("err.flow.compare.variants"));
    }
    Ok(Variant {
        label,
        algorithm,
        preset: text("preset"),
        params,
    })
}

fn variants(inst: &Instance) -> Result<Vec<Variant>> {
    let list = inst
        .params
        .iter()
        .find(|(k, _)| k == "variants")
        .map_or(&[][..], |(_, v)| v.arr());
    let all = list.iter().map(variant).collect::<Result<Vec<_>>>()?;
    if all.len() < 2 {
        return Err(Error::new("err.flow.compare.variants"));
    }
    for (k, a) in all.iter().enumerate() {
        for b in &all[..k] {
            if a.label == b.label {
                return Err(Error::new("err.flow.compare.variants").with("label", &a.label));
            }
            if (&a.algorithm, &a.preset, &a.params) == (&b.algorithm, &b.preset, &b.params) {
                return Err(Error::new("err.flow.compare.sameVariant").with("variant", &a.label));
            }
        }
    }
    Ok(all)
}

/// The step `meta.compare` wraps, from `"id@version"`.
fn wrapped<'a>(inst: &Instance, registry: &'a [Step]) -> Result<&'a Step> {
    let name = inst
        .params
        .iter()
        .find(|(k, _)| k == "step")
        .and_then(|(_, v)| v.str())
        .unwrap_or("");
    let found = name
        .rsplit_once('@')
        .and_then(|(id, v)| Some((id, v.parse::<u32>().ok()?)))
        .and_then(|(id, v)| registry.iter().find(|s| s.id == id && s.version == v));
    let step = found.ok_or_else(|| Error::new("err.flow.compare.step").with("step", name))?;
    match step.outs.as_slice() {
        [o] if matches!(o.ty, DataType::Candidates | DataType::Annot) => Ok(step),
        _ => Err(Error::new("err.flow.compare.type").with("step", name)),
    }
}

fn expand_one(inst: &Instance, registry: &[Step]) -> Result<Vec<Instance>> {
    let step = wrapped(inst, registry)?;
    let all = variants(inst)?;
    let mut made: Vec<Instance> = Vec::new();
    let mut refs = Vec::new();
    for v in &all {
        let id = format!("{}~{}", inst.id, v.label);
        refs.push(Ref::Out {
            instance: id.clone(),
            port: step.outs[0].name.to_string(),
        });
        made.push(Instance {
            id,
            step: step.id.to_string(),
            version: step.version,
            algorithm: Some(v.algorithm.clone()),
            preset: v.preset.clone(),
            label: Some(v.label.clone()),
            ins: inst.ins.clone(),
            params: v.params.clone(),
        });
    }
    let labels: Vec<&str> = all.iter().map(|v| v.label.as_str()).collect();
    let mut params: Vec<(String, Value)> = inst
        .params
        .iter()
        .filter(|(k, _)| k != "variants" && k != "step")
        .cloned()
        .collect();
    params.push((
        "step".into(),
        Value::Str(format!("{}@{}", step.id, step.version)),
    ));
    params.push(("labels".into(), Value::Str(labels.join(","))));
    made.push(Instance {
        ins: vec![("variants".into(), refs)],
        params,
        step: inst.step.clone(),
        ..inst.clone()
    });
    Ok(made)
}

/// The plan with every `meta.compare` replaced by its variants and the compare instance (the other instances unchanged, in the same order).
pub fn expand(plan: Plan, registry: &[Step]) -> Result<Plan> {
    let mut steps = Vec::new();
    for inst in &plan.steps {
        if inst.step == "meta.compare" {
            steps.extend(expand_one(inst, registry)?);
        } else {
            steps.push(inst.clone());
        }
    }
    Ok(Plan { steps, ..plan })
}

/// What one variant gave: for each key (a query, or a row of the annotation) the ranked values, best first (the ids of the candidates, or the level).
struct Ranked {
    kind: &'static str,
    by: &'static str,
    keys: Vec<(String, Vec<String>)>,
}

fn field(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::Str(s)) => s.clone(),
        Some(Value::Num(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn ranked(bytes: &[u8]) -> Result<Ranked> {
    let v = parse(bytes)?;
    let (kind, by, list, key, value) = if v.get("hits").is_some() {
        ("candidates", "query", "hits", "query", "id")
    } else {
        ("annot", "row", "rows", "id", "level")
    };
    let mut keys: Vec<(String, Vec<String>)> = Vec::new();
    for h in v.get(list).map_or(&[][..], Value::arr) {
        let (k, x) = (field(h, key), field(h, value));
        let x = if x.is_empty() { field(h, "name") } else { x };
        match keys.iter_mut().find(|e| e.0 == k) {
            Some(e) => e.1.push(x),
            None => keys.push((k, vec![x])),
        }
    }
    Ok(Ranked { kind, by, keys })
}

/// Spearman correlation of the order of the values both lists hold (None below three shared values).
fn rank_corr(a: &[String], b: &[String]) -> Option<f64> {
    let shared: Vec<&String> = a.iter().filter(|x| b.contains(x)).collect();
    let n = shared.len() as f64;
    if shared.len() < 3 {
        return None;
    }
    let order_b: Vec<&String> = b.iter().filter(|x| shared.contains(x)).collect();
    let d2: f64 = shared
        .iter()
        .enumerate()
        .map(|(i, x)| {
            let j = order_b.iter().position(|y| y == x).unwrap_or(0) as f64;
            (i as f64 - j).powi(2)
        })
        .sum();
    Some(1.0 - 6.0 * d2 / (n * (n * n - 1.0)))
}

fn overlap(a: &[String], b: &[String]) -> f64 {
    let both = a.iter().filter(|x| b.contains(x)).count() as f64;
    let all = (a.len() + b.len()) as f64 - both;
    if all == 0.0 {
        1.0
    } else {
        both / all
    }
}

fn find<'a>(r: &'a Ranked, key: &str) -> Option<&'a Vec<String>> {
    r.keys.iter().find(|e| e.0 == key).map(|e| &e.1)
}

/// The keys of all variants, in the order they first appear.
fn all_keys(all: &[Ranked]) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for e in all.iter().flat_map(|r| &r.keys) {
        if !keys.contains(&e.0) {
            keys.push(e.0.clone());
        }
    }
    keys
}

fn write_pair(o: &mut Out, labels: (&str, &str), (a, b): (&Ranked, &Ranked), keys: &[String]) {
    let (mut both, mut agree, mut only_a, mut only_b) = (0usize, 0usize, 0usize, 0usize);
    let (mut overlaps, mut corrs) = (Vec::new(), Vec::new());
    for k in keys {
        match (find(a, k), find(b, k)) {
            (Some(x), Some(y)) => {
                both += 1;
                agree += usize::from(x[0] == y[0]);
                overlaps.push(overlap(x, y));
                corrs.extend(rank_corr(x, y));
            }
            (Some(_), None) => only_a += 1,
            (None, Some(_)) => only_b += 1,
            (None, None) => {}
        }
    }
    let mean = |v: &[f64]| (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64);
    o.raw("{\"a\":").str(labels.0).raw(",\"b\":").str(labels.1);
    o.raw(",\"both\":").int(both as i64);
    o.raw(",\"best_agree\":").int(agree as i64);
    o.raw(",\"only_a\":").int(only_a as i64);
    o.raw(",\"only_b\":").int(only_b as i64);
    o.raw(",\"mean_overlap\":")
        .opt(mean(&overlaps).map(|m| round(m, 4)));
    o.raw(",\"mean_rank_corr\":")
        .opt(mean(&corrs).map(|m| round(m, 4)));
    o.raw("}");
}

fn write_key(o: &mut Out, all: &[Ranked], key: &str) {
    let best: Vec<Option<&String>> = all.iter().map(|r| find(r, key).map(|v| &v[0])).collect();
    let agree = best.iter().all(|b| b.is_some() && *b == best[0]);
    o.raw("{\"key\":").str(key).raw(",\"best\":[");
    for (i, b) in best.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        match b {
            Some(s) => o.str(s),
            None => o.raw("null"),
        };
    }
    o.raw("],\"agree\":")
        .raw(if agree { "true" } else { "false" })
        .raw("}");
}

/// The `comparison/1` text for the outputs of the variants (`labels`, one file each, in the same order) of the step `wrapped`.
pub fn compare_text(wrapped: &str, labels: &[&str], files: &[&[u8]]) -> Result<String> {
    let all = files
        .iter()
        .map(|f| ranked(f))
        .collect::<Result<Vec<_>>>()?;
    let (Some(first), true) = (all.first(), all.len() == labels.len()) else {
        return Err(Error::new("err.flow.compare.variants"));
    };
    let keys = all_keys(&all);
    let mut o = Out::default();
    o.raw("{\"format\":\"comparison/1\",\"step\":").str(wrapped);
    o.raw(",\"type\":")
        .str(first.kind)
        .raw(",\"by\":")
        .str(first.by);
    o.raw(",\"variants\":[");
    for (i, l) in labels.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(l);
    }
    o.raw("],\"pairs\":[");
    let mut first_pair = true;
    for i in 0..all.len() {
        for j in i + 1..all.len() {
            o.raw(if first_pair { "" } else { "," });
            first_pair = false;
            write_pair(&mut o, (labels[i], labels[j]), (&all[i], &all[j]), &keys);
        }
    }
    o.raw("],\"keys\":[");
    for (i, k) in keys.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        write_key(&mut o, &all, k);
    }
    o.raw("]}");
    Ok(o.0)
}
