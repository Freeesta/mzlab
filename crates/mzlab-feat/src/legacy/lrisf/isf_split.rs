//! The roles of an ion inherited from the parent (`isf.py::_split_inherited`): weights of in-source fragment / isotope / adduct / other from
//! the mass difference and the roles `origin_report` found. Physics is a hard constraint: an ion heavier than the parent is not a fragment.

use super::isf::{fdef, nn, truthy};
use crate::sortc::SortC;
use mzlab_core::json::Value;

/// Weights `[isf, isotope, adduct, other]` (sum 1) and the notes; `None` when the report has no `mz` / `parent_mz`.
pub fn split_inherited(rep: &Value, formula_p: Option<&str>) -> Option<([f64; 4], Vec<String>)> {
    let d = rep.get("mz")?.f64()? - rep.get("parent_mz")?.f64()?;
    let roles = rep.get("roles").map_or(&[][..], |v| v.arr());
    let kind = |r: &Value| r.get("role").and_then(|v| v.str()).map(str::to_string);
    let kinds: Vec<String> = roles.iter().filter_map(kind).collect();
    let has = |names: &[&str]| kinds.iter().any(|k| names.contains(&k.as_str()));
    let mut w = [0.0; 4];
    let mut notes = Vec::new();
    if d <= -0.5 {
        loss_weights(rep, roles, formula_p, &mut w, &mut notes);
    } else {
        notes.push("heavier than the parent: cannot be an in-source fragment".to_string());
        if has(&["isotope"]) {
            w[1] = 5.0;
        }
        if has(&["adduct", "dimer"]) {
            w[2] = 5.0;
        }
        if !has(&["isotope", "adduct", "dimer"]) {
            w[3] = 1.0;
        }
    }
    let s = w.iter().fold(0.0, |a, v| a + v);
    let s = if s == 0.0 { 1.0 } else { s };
    Some((w.map(|v| v / s), notes))
}

fn loss_weights(
    rep: &Value,
    roles: &[Value],
    formula_p: Option<&str>,
    w: &mut [f64; 4],
    notes: &mut Vec<String>,
) {
    let frag: Vec<&Value> = roles
        .iter()
        .filter(|r| r.get("role").and_then(|v| v.str()) == Some("fragment_candidate"))
        .collect();
    let ok_f: Vec<&&Value> = frag
        .iter()
        .filter(|r| r.get("formula_ok").is_none_or(|v| truthy(Some(v))))
        .collect();
    w[0] = 1.0;
    if !ok_f.is_empty() {
        w[0] = 3.0;
        let mut labels: Vec<&str> = ok_f
            .iter()
            .filter_map(|r| r.get("label").and_then(|v| v.str()))
            .collect();
        labels.sortc();
        labels.dedup();
        notes.push(format!(
            "known neutral loss: {}",
            labels
                .iter()
                .take(3)
                .copied()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else if !frag.is_empty() && formula_p.is_some_and(|f| !f.is_empty()) {
        w[0] = 0.3;
        notes.push("the neutral loss is not compatible with the parent formula".to_string());
    } else if frag.is_empty() {
        w[0] = 0.7;
        notes.push("no neutral loss in the table explains \u{394}m".to_string());
    }
    // isotope of a fragment: a lighter ion 1 or 2 Da below with the same profile and a plausible M+1 / M+2 ratio
    for nb in rep.get("neighbors").map_or(&[][..], |v| v.arr()) {
        if fdef(nb, "pearson", 0.0) > 0.9 && fdef(nb, "area_ratio", 9.0) <= 0.9 {
            w[1] = 2.5;
            let (delta, mz) = (
                nn(nb, "delta").and_then(|v| v.f64()).unwrap_or(0.0),
                nn(nb, "mz").and_then(|v| v.f64()).unwrap_or(0.0),
            );
            notes.push(format!(
                "possible M+{} isotope of the ion at m/z {:.1}",
                delta as i64, mz
            ));
            break;
        }
    }
}
