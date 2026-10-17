//! The verdict of one ion (`isf.py::classify_report`): probabilities that it is inherited from the parent (in-source fragment, isotope,
//! adduct, other) or a true transformation product, the evidence behind them, the doubtful cases with their reason and a readable explanation.
//! MS2 and DP-ramp evidence enter as log-odds with heuristic weights, flagged `calibrated: false`. Texts in English, identical to the Python.

use super::isf::{fdef, features, nn, predict_logit, sigmoid, sub, truthy, NAMES};
use super::isf_split::split_inherited;
use super::isf_text::{english, evidence_value, LABELS};
use crate::sortc::SortC;
use mzlab_core::json::Value;
use mzlab_core::jsonb::{num, text, Obj};
use mzlab_core::stats::wspearman;

/// Breakdown data from standards at different declustering potentials: DP (V) and the intensities of the fragment and of the parent.
pub struct Ramp<'a> {
    pub dp: &'a [f64],
    pub f: &'a [f64],
    pub p: &'a [f64],
}

/// Spearman of F/(F+P) against the DP (`source_ramp`): `None` below three finite ratios.
fn ramp_spearman(r: &Ramp) -> Option<f64> {
    let mut pts: Vec<(f64, f64)> = (0..r.dp.len().min(r.f.len()).min(r.p.len()))
        .map(|i| (r.dp[i], r.f[i] / (r.f[i] + r.p[i])))
        .filter(|p| p.1.is_finite())
        .collect();
    if pts.len() < 3 {
        return None;
    }
    pts.sortc_by(|a, b| a.0.total_cmp(&b.0));
    let (d, v): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
    Some(wspearman(&d, &v, None)).filter(|s| s.is_finite())
}

/// The log-odds of the MS2 and ramp evidence: `(name, log-odds, text)`.
fn extra_evidence(rep: &Value, ramp: Option<&Ramp>) -> Vec<(&'static str, f64, String)> {
    let mut extra = Vec::new();
    let ms2 = sub(rep, "ms2");
    if let Some(inp) = nn(ms2, "x_in_p") {
        let present = truthy(Some(inp));
        let mut v = if present { 1.0 } else { -0.5 };
        let mc = nn(ms2, "modified_cosine")
            .and_then(|m| m.f64())
            .filter(|m| m.is_finite());
        if let Some(m) = mc {
            v += 2.0 * (m - 0.5);
        }
        let tail = mc.map_or(String::new(), |m| format!(", modified cosine {m:.2}"));
        extra.push((
            "ms2",
            v,
            format!(
                "MS2: {} among the products of the parent{tail}",
                if present { "present" } else { "absent" }
            ),
        ));
    }
    if let Some(s) = ramp.and_then(ramp_spearman) {
        extra.push((
            "ramp",
            3.0 * s,
            format!("DP ramp: Spearman {s:.2} of the F/(F+P) ratio with the DP"),
        ));
    }
    extra
}

fn evidence_entry(name: &str, label: &str, value: &str, c: f64, calibrated: bool) -> (f64, Value) {
    let o = Obj::new()
        .put("name", text(name))
        .put("label", text(label))
        .put("value", text(value))
        .n("log_odds", c);
    (
        c,
        o.put("toward", text(if c > 0.0 { "parent" } else { "tp" }))
            .b("calibrated", calibrated)
            .done(),
    )
}

/// Evidence list sorted by weight (stable), most decisive first.
fn evidence(
    rep: &Value,
    feat: &[f64; 9],
    contrib: &[f64; 9],
    extra: &[(&str, f64, String)],
) -> Option<Vec<(f64, Value)>> {
    let mut ev = Vec::new();
    for k in 0..9 {
        if contrib[k].abs() < 0.05 && !feat[k].is_finite() {
            continue;
        }
        ev.push(evidence_entry(
            NAMES[k],
            LABELS[k],
            &evidence_value(k, feat, rep)?,
            contrib[k],
            true,
        ));
    }
    ev.extend(
        extra
            .iter()
            .map(|(n, v, t)| evidence_entry(n, t, "", *v, false)),
    );
    let mut order: Vec<usize> = (0..ev.len()).collect();
    crate::sortc::by(&mut order, |&a, &b| ev[b].0.abs().total_cmp(&ev[a].0.abs()));
    Some(order.into_iter().map(|i| ev[i].clone()).collect())
}

fn reasons(rep: &Value, feat: &[f64; 9], p_inh: f64, other: f64) -> Vec<String> {
    let mut r = Vec::new();
    if (0.25..=0.75).contains(&p_inh) {
        r.push(format!(
            "intermediate probability ({:.0}% of being inherited from the parent)",
            100.0 * p_inh
        ));
    }
    if feat[0].is_finite() && feat[0] >= 0.9 && feat[4].is_finite() && feat[4] < -2.0 {
        r.push("profile identical to that of the parent but F/P ratio not constant across samples: possible mixture with a co-eluting isobaric product, or saturation of the parent (see the MCR families)".into());
    }
    if !truthy(sub(rep, "profile").get("reliable")) {
        r.push("unreliable measurement (few scans, missing peak or weak ion)".into());
    }
    for w in rep.get("warnings").map_or(&[][..], |v| v.arr()) {
        if matches!(
            w.get("key").and_then(|k| k.str()),
            Some("of.warn.massCoincidence" | "of.warn.flatTop")
        ) {
            r.push(english(w));
        }
    }
    let heavy = fdef(rep, "mz", 0.0) - fdef(rep, "parent_mz", 0.0) > -0.5;
    if heavy && p_inh >= 0.5 && other > 0.5 {
        r.push("heavier than the parent and not explained by a known isotope or adduct".into());
    }
    r
}

fn explanation(
    lead: (&str, f64),
    ev: &[(f64, Value)],
    notes: &[String],
    reasons: &[String],
) -> String {
    let mut s = format!(
        "{}: {:.0}% {}. ",
        if reasons.is_empty() {
            "Outcome"
        } else {
            "Doubt"
        },
        100.0 * lead.1,
        lead.0
    );
    let top: Vec<String> = ev
        .iter()
        .filter(|e| e.0.abs() >= 0.3)
        .take(4)
        .map(|(_, e)| {
            let g = |k: &str| e.get(k).and_then(|v| v.str()).unwrap_or("").to_string();
            format!(
                "{} ({}) {}",
                g("label"),
                g("value"),
                if g("toward") == "parent" {
                    "\u{2192} parent"
                } else {
                    "\u{2192} true TP"
                }
            )
        })
        .collect();
    if !top.is_empty() {
        s += &format!("Main evidence: {}. ", top.join("; "));
    }
    if !notes.is_empty() {
        s += &format!("Notes: {}. ", notes.join("; "));
    }
    if !reasons.is_empty() {
        s += &format!("Reasons for the doubt: {}. ", reasons.join("; "));
    }
    s + "Unit resolution: this is a probability, not an identification."
}

fn strs(a: &[String]) -> Value {
    Value::Arr(a.iter().map(|s| text(s)).collect())
}

/// Probabilities and evidence for the ion of one `origin_report` (`fam` is not used: the engine calls it with `fam = False`). `None` on a
/// report that lacks what the Python reads (it raises there).
pub fn classify_report(rep: &Value, formula_p: Option<&str>, ramp: Option<&Ramp>) -> Option<Value> {
    let feat = features(rep);
    let (mut logit, contrib) = predict_logit(&feat);
    let extra = extra_evidence(rep, ramp);
    extra.iter().for_each(|e| logit += e.1);
    let p_inh = sigmoid(logit);
    let (split, notes) = split_inherited(rep, formula_p)?;
    // insertion order of the Python dict decides ties for the leading hypothesis
    let probs = [
        ("isf", p_inh * split[0]),
        ("isotope", p_inh * split[1]),
        ("adduct", p_inh * split[2]),
        ("tp", 1.0 - p_inh),
        ("other_from_parent", p_inh * split[3]),
    ];
    let ev = evidence(rep, &feat, &contrib, &extra)?;
    let reasons = reasons(rep, &feat, p_inh, split[3]);
    let lead = probs
        .iter()
        .fold(probs[0], |b, p| if p.1 > b.1 { *p } else { b });
    let name = match lead.0 {
        "isf" => "in-source fragment (ISF)",
        "isotope" => "isotope of the parent or of one of its fragments",
        "adduct" => "adduct/dimer of the parent",
        "other_from_parent" => "ion derived from the parent (unclassified)",
        _ => "true transformation product",
    };
    let pj = probs.iter().fold(Obj::new(), |o, (k, v)| o.n(k, *v)).done();
    let fj = NAMES
        .iter()
        .zip(feat)
        .fold(Obj::new(), |o, (k, v)| o.put(k, num(v)))
        .done();
    Some(
        Obj::new()
            .put("mz", num(fdef(rep, "mz", f64::NAN)))
            .put("parent_mz", num(fdef(rep, "parent_mz", f64::NAN)))
            .put("probs", pj)
            .n("p_inherited", p_inh)
            .n("isf_ness", 100.0 * probs[0].1)
            .put(
                "evidence",
                Value::Arr(ev.iter().map(|e| e.1.clone()).collect()),
            )
            .b("doubtful", !reasons.is_empty())
            .put("reasons", strs(&reasons))
            .put(
                "explanation",
                text(&explanation((name, lead.1), &ev, &notes, &reasons)),
            )
            .put("notes", strs(&notes))
            .put(
                "warnings",
                rep.get("warnings").cloned().unwrap_or(Value::Arr(vec![])),
            )
            .put("features", fj)
            .b("calibrated", true)
            .done(),
    )
}
