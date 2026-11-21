//! The rules of `table.filter` in the language of the engine (`normalize`/`filter`), from the parameters of the step: one rule for `flow run` and the browser.
use mzlab_core::error::Result;
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::samples;
use mzlab_feat::filters::QC_MIN_FILES;
use mzlab_feat::filters_blank::Labels;

fn param<'a>(job: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    job.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

fn number(job: &[(String, Value)], name: &str) -> Option<f64> {
    param(job, name).and_then(|v| v.f64())
}

/// The options of the blank rule that differ from the defaults, as the fields that follow `value`.
fn blank_extra(job: &[(String, Value)]) -> String {
    let text = |k: &str| param(job, k).and_then(|v| v.str());
    let mut o = String::new();
    let mut put = |key: &str, v: &str| o.push_str(&format!(",\"{key}\":\"{v}\""));
    match text("blank_quantity") {
        Some("height") => put("quantity", "height"),
        Some("norm_area") => put("quantity", "norm_area"),
        _ => {}
    }
    if text("blank_stat") == Some("max") {
        put("statistic", "max");
    }
    if text("blank_scope") == Some("each_feature") {
        put("scope", "each");
    }
    match text("blank_match") {
        Some("batch") => put("matching", "batch"),
        Some("matrix") => put("matching", "matrix"),
        _ => {}
    }
    if param(job, "blank_subtract").and_then(|v| v.bool()) == Some(true) {
        o.push_str(",\"subtract\":true");
    }
    if text("blank_cells") == Some("detected") {
        o.push_str(",\"blank_cells\":\"detected\"");
    }
    if param(job, "blank_keep_istd").and_then(|v| v.bool()) == Some(true) {
        o.push_str(",\"istd_exempt\":true");
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

/// The batch, matrix, series and group of every file of a `samples/1` document.
fn labels_of(samples: &[u8]) -> Labels {
    let Ok(list) = samples::from_json(samples) else {
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

/// `{"normalize":..., "filter":[...]}`: the rules in the order of the descriptor, each one only when its parameter is set.
pub fn spec_text(job: &[(String, Value)]) -> String {
    spec_labelled(job, &Labels::default())
}

fn spec_labelled(job: &[(String, Value)], labels: &Labels) -> String {
    let mut rules: Vec<String> = Vec::new();
    let mut rule = |name: &str, value: Option<f64>, extra: &str| {
        if let Some(v) = value {
            let mut o = Out::default();
            o.raw("{\"rule\":").str(name).raw(",\"value\":").num(v);
            rules.push(format!("{}{extra}}}", o.0));
        }
    };
    rule("min_detected", number(job, "min_detected"), "");
    rule("min_replicates", number(job, "min_replicate_fraction"), "");
    rule("min_area", number(job, "min_area"), "");
    rule("min_height", number(job, "min_height"), "");
    rule("blank", number(job, "blank_ratio"), &blank_extra(job));
    let qc_files = number(job, "qc_min_files").unwrap_or(QC_MIN_FILES as f64);
    let extra = format!(",\"qc_min_files\":{qc_files}");
    let zero = param(job, "qc_zero").and_then(|v| v.bool()) == Some(true);
    let extra = if zero {
        format!("{extra},\"qc_zero\":true")
    } else {
        extra
    };
    rule("qc_cv_max", number(job, "qc_cv_max"), &extra);
    let (ppm, da) = (number(job, "dup_ppm"), number(job, "dup_da"));
    if ppm.is_some() || da.is_some() {
        let rt = number(job, "dup_rt").unwrap_or(0.1);
        let mut o = Out::default();
        o.raw("{\"rule\":\"duplicates\"");
        if let Some(x) = ppm {
            o.raw(",\"ppm\":").num(x);
        }
        if let Some(x) = da {
            o.raw(",\"from\":").num(x);
        }
        o.raw(",\"rt_min\":").num(rt);
        rules.push(format!("{}}}", o.0));
    }
    if param(job, "require_ms2").and_then(|v| v.bool()) == Some(true) {
        rules.push("{\"rule\":\"ms2\"}".to_string());
    }
    let method = match param(job, "normalize").and_then(|v| v.str()) {
        Some("sum") => "\"sum\"".to_string(),
        Some("median") => "\"median\"".to_string(),
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
        _ => "\"none\"".to_string(),
    };
    let labels = if labels.is_empty() {
        String::new()
    } else {
        format!(",\"labels\":{}", labels.to_json())
    };
    format!(
        "{{\"normalize\":{{\"method\":{method}}},\"filter\":[{}]{labels}}}",
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
