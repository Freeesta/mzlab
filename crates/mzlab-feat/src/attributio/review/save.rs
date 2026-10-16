//! The decision of the chemist in the Banco 2b, written in the format of `feature_decisions.csv`: the page sends one decision as JSON; this module checks it with
//! the rules of the Banco, adds it to the decisions already in the folder (a history: only the row with the same `decided_on` and `decided_by` is replaced) and
//! gives back the text of the whole file for the caller to store. The checklist the chemist ticked goes in the last column, `evidence_checklist`, separated by `|`.

use super::banco::{Bench, CHECKLIST};
use super::rules::{decision_changes, Current, Rec};
use crate::attributio::annotate::Feature;
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::io::Io;
use crate::attributio::readers::feature_decisions::{
    merge_decisions, write_feature_decisions, DecisionTable, EVIDENCE_COLUMNS,
};
use mzlab_core::json::{self, Value};

/// The file after the decision and what happened to the row.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    pub path: String,
    pub text: String,
    pub replaced: usize,
    pub added: usize,
    pub unchanged: usize,
}

/// What `level` of the file each level of the page is written as (`2a` is `2`); the empty level keeps the cascade as it is.
fn file_level(page: &str) -> Option<&'static str> {
    match page.trim() {
        "" => Some(""),
        "2a" | "2" => Some("2"),
        "2b" => Some("2b"),
        "3" => Some("3"),
        "3a" => Some("3a"),
        "3b" => Some("3b"),
        _ => None,
    }
}

/// A scalar of the JSON as text (numbers as they print), empty for null or missing.
fn text(v: Option<&Value>) -> String {
    match v {
        Some(x) => x
            .str()
            .map(str::to_string)
            .or_else(|| x.f64().map(|n| n.to_string()))
            .unwrap_or_default(),
        None => String::new(),
    }
    .trim()
    .to_string()
}

fn bad(msg: impl Into<String>) -> CampaignError {
    CampaignError::value(msg)
}

/// `value` when it is one of `allowed`, else an error naming `field`.
fn choice(field: &str, value: String, allowed: &[&str]) -> Result<String> {
    if allowed.contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(bad(format!("{field} {value:?}")))
    }
}

/// The keys of the checklist the chemist ticked; a key that is not in the checklist is an error.
fn ticked_items(d: &Value) -> Result<Vec<String>> {
    let ticked: Vec<String> = d
        .get("checklist")
        .map(|c| {
            c.arr()
                .iter()
                .filter_map(|k| k.str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    match ticked
        .iter()
        .find(|k| !CHECKLIST.iter().any(|c| c.0 == k.as_str()))
    {
        Some(k) => Err(bad(format!("checklist item {k:?}"))),
        None => Ok(ticked),
    }
}

/// The rules of the Banco: a note when the decision changes something, a note for every 2b, no analog as Level 2a.
fn check_rules(
    f: &Feature,
    level: &str,
    group: &str,
    name: &str,
    nota: &str,
    etype: &str,
) -> Result<()> {
    let cur = Current {
        level: f.level,
        level_sub: &f.level_sub,
        decision_2b: f.decision.decision_2b,
        name: f.name(),
        group: f.decision.group_override.as_deref(),
    };
    let changes = decision_changes(&cur, &Rec { level, group, name });
    if !changes.is_empty() && nota.is_empty() {
        return Err(bad(format!(
            "a note (justification) is required: the decision changes {}",
            changes.join(", ")
        )));
    }
    if level == "2b" && nota.is_empty() {
        return Err(bad("a '2b' row needs a note (justification)"));
    }
    if etype == "analog" && level == "2" {
        return Err(bad("an analog is never Level 2a"));
    }
    Ok(())
}

/// `decision` is `{pol, id, level, name, group, justification, decided_by, decided_on, rt_check, checklist: [keys], evidence: {type, library, entry, cosine, matched, delta_mz, delta_formula, note}}`.
pub fn decision_text(io: &Io, root: &str, decision: &str) -> Result<Saved> {
    let d = json::parse(decision.as_bytes()).map_err(|_| bad("the decision is not valid JSON"))?;
    let g = |k: &str| text(d.get(k));
    let (pol, id) = (
        g("pol"),
        g("id")
            .parse::<i64>()
            .map_err(|_| bad("the decision has no feature id"))?,
    );
    let b = Bench::load(io, root)?;
    let f = b
        .feature(&pol, id)
        .ok_or_else(|| bad(format!("feature {pol} {id} is not in the table")))?;
    let level = file_level(&g("level")).ok_or_else(|| bad(format!("level {:?}", g("level"))))?;
    let group = choice("group", g("group"), &["", "ambiguous"])?;
    let rt_check = choice("rt_check", g("rt_check"), &["", "peak", "isomer"])?;
    let ev = d.get("evidence");
    let ev_text = |k: &str| text(ev.and_then(|e| e.get(k)));
    let etype = choice(
        "evidence_type",
        ev_text("type"),
        &["", "library", "analog", "fragments", "in_silico", "context"],
    )?;
    let (name, nota) = (g("name"), g("justification"));
    check_rules(f, level, &group, &name, &nota, &etype)?;
    let ticked = ticked_items(&d)?;
    let mut row = DecisionTable::empty();
    row.rows.push(vec![String::new(); row.columns.len()]);
    let num = |v: Option<f64>| {
        v.filter(|x| x.is_finite())
            .map_or(String::new(), |x| x.to_string())
    };
    let cells: Vec<(&str, String)> = vec![
        ("pol", pol),
        ("feature_id", id.to_string()),
        ("mz", num(f.mz)),
        ("rt", num(f.rt)),
        ("level", level.to_string()),
        ("name", name),
        ("identity", g("identity")),
        ("decided_on", g("decided_on")),
        ("decided_by", g("decided_by")),
        ("justification", nota),
        ("group", group),
        ("rt_check", rt_check),
        ("evidence_type", etype),
        ("evidence_library", ev_text("library")),
        ("evidence_entry", ev_text("entry")),
        ("evidence_cosine", ev_text("cosine")),
        ("evidence_matched", ev_text("matched")),
        ("evidence_delta_mz", ev_text("delta_mz")),
        ("evidence_delta_formula", ev_text("delta_formula")),
        ("evidence_note", ev_text("note")),
        ("evidence_checklist", ticked.join("|")),
    ];
    debug_assert!(EVIDENCE_COLUMNS
        .iter()
        .all(|c| cells.iter().any(|x| x.0 == *c)));
    for (col, v) in cells {
        row.put(0, col, &v);
    }
    let (path, _) = b.decisions_path();
    let (merged, stats) = merge_decisions(&b.decisions(io)?, &row);
    Ok(Saved {
        path,
        text: write_feature_decisions(&merged),
        replaced: stats.replaced,
        added: stats.added,
        unchanged: stats.unchanged,
    })
}
