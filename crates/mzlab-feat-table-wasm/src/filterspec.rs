//! The rules of `table.filter` in the language of the engine (`normalizza`/`filtra`), from the parameters of the step: one rule for `flow run` and the browser.
use mzlab_core::campioni;
use mzlab_core::error::Result;
use mzlab_core::json::{parse, Out, Value};
use mzlab_feat::filters::QC_MIN_FILES;
use mzlab_feat::filters_blank::Labels;

fn param<'a>(job: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    job.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

fn number(job: &[(String, Value)], name: &str) -> Option<f64> {
    param(job, name).and_then(|v| v.f64())
}

/// The options of the blank rule that differ from the defaults, as the fields that follow `valore`.
fn blank_extra(job: &[(String, Value)]) -> String {
    let text = |k: &str| param(job, k).and_then(|v| v.str());
    let mut o = String::new();
    let mut put = |key: &str, v: &str| o.push_str(&format!(",\"{key}\":\"{v}\""));
    match text("blank_quantity") {
        Some("height") => put("grandezza", "altezza"),
        Some("norm_area") => put("grandezza", "area_norm"),
        _ => {}
    }
    if text("blank_stat") == Some("max") {
        put("statistica", "massimo");
    }
    if text("blank_scope") == Some("each_feature") {
        put("ambito", "ciascuna");
    }
    match text("blank_match") {
        Some("batch") => put("abbinamento", "lotto"),
        Some("matrix") => put("abbinamento", "matrice"),
        _ => {}
    }
    if param(job, "blank_subtract").and_then(|v| v.bool()) == Some(true) {
        o.push_str(",\"sottrai\":true");
    }
    if param(job, "blank_keep_istd").and_then(|v| v.bool()) == Some(true) {
        o.push_str(",\"istd_esente\":true");
    }
    o
}

/// Whether the rules need the labels of the files (batch, matrix, series, group).
fn wants_labels(job: &[(String, Value)]) -> bool {
    number(job, "min_replicate_fraction").is_some()
        || matches!(
            param(job, "blank_match").and_then(|v| v.str()),
            Some("batch" | "matrix")
        )
}

/// The batch, matrix, series and group of every file of a `campioni/2` or `/3` document; empty for `campioni/1`, which has none.
fn labels_of(samples: &[u8]) -> Labels {
    let Ok(list) = campioni::from_json(samples) else {
        return Labels::default();
    };
    Labels {
        batch: list.iter().map(|c| c.batch.clone()).collect(),
        matrix: list.iter().map(|c| c.matrix.clone()).collect(),
        series: list.iter().map(|c| c.series.clone()).collect(),
        group: list.iter().map(|c| c.group.clone()).collect(),
    }
}

/// `spec_text` with the labels of the files read from `samples` (the samples file of the step), when a rule needs them.
pub fn spec_with_samples(job: &[(String, Value)], samples: &[u8]) -> String {
    let labels = if wants_labels(job) {
        labels_of(samples)
    } else {
        Labels::default()
    };
    spec_labelled(job, &labels)
}

/// `{"normalizza":..., "filtra":[...]}`: the rules in the order of the descriptor, each one only when its parameter is set.
pub fn spec_text(job: &[(String, Value)]) -> String {
    spec_labelled(job, &Labels::default())
}

fn spec_labelled(job: &[(String, Value)], labels: &Labels) -> String {
    let mut rules: Vec<String> = Vec::new();
    let mut rule = |name: &str, value: Option<f64>, extra: &str| {
        if let Some(v) = value {
            let mut o = Out::default();
            o.raw("{\"regola\":").str(name).raw(",\"valore\":").num(v);
            rules.push(format!("{}{extra}}}", o.0));
        }
    };
    rule("min_rilevati", number(job, "min_detected"), "");
    rule("repliche_min", number(job, "min_replicate_fraction"), "");
    rule("min_area", number(job, "min_area"), "");
    rule("min_altezza", number(job, "min_height"), "");
    rule("bianco", number(job, "blank_ratio"), &blank_extra(job));
    let qc_files = number(job, "qc_min_files").unwrap_or(QC_MIN_FILES as f64);
    let extra = format!(",\"qc_min_files\":{qc_files}");
    let zero = param(job, "qc_zero").and_then(|v| v.bool()) == Some(true);
    let extra = if zero {
        format!("{extra},\"qc_zero\":true")
    } else {
        extra
    };
    rule("qc_cv_max", number(job, "qc_cv_max"), &extra);
    if let Some(ppm) = number(job, "dup_ppm") {
        let rt = number(job, "dup_rt").unwrap_or(0.1);
        let mut o = Out::default();
        o.raw("{\"regola\":\"duplicati\",\"ppm\":")
            .num(ppm)
            .raw(",\"rt_min\":")
            .num(rt);
        rules.push(format!("{}}}", o.0));
    }
    if param(job, "require_ms2").and_then(|v| v.bool()) == Some(true) {
        rules.push("{\"regola\":\"ms2\"}".to_string());
    }
    let method = match param(job, "normalize").and_then(|v| v.str()) {
        Some("sum") => "\"somma\"".to_string(),
        Some("median") => "\"mediana\"".to_string(),
        Some("istd") => {
            let mut o = Out::default();
            o.raw("\"istd\",\"istd\":{");
            match (
                number(job, "istd_id"),
                number(job, "istd_mz"),
                number(job, "istd_rt"),
            ) {
                (Some(id), ..) => o.raw("\"id\":").num(id),
                (None, Some(mz), Some(rt)) => o.raw("\"mz\":").num(mz).raw(",\"rt\":").num(rt),
                _ => &mut o,
            };
            o.raw("}");
            o.0
        }
        _ => "\"nessuna\"".to_string(),
    };
    let labels = if labels.is_empty() {
        String::new()
    } else {
        format!(",\"etichette\":{}", labels.to_json())
    };
    format!(
        "{{\"normalizza\":{{\"metodo\":{method}}},\"filtra\":[{}]{labels}}}",
        rules.join(",")
    )
}

/// The same from a JSON object of parameters (the defaults of the descriptor under the ones the plan writes), as the browser holds them.
pub fn spec_from_json(params_json: &str) -> Result<String> {
    Ok(spec_text(parse(params_json.as_bytes())?.entries()))
}

/// `spec_from_json` with the labels of the files read from the samples document.
pub fn spec_from_json_with_samples(params_json: &str, samples: &[u8]) -> Result<String> {
    Ok(spec_with_samples(
        parse(params_json.as_bytes())?.entries(),
        samples,
    ))
}
