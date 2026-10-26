//! The decisions of the chemist on the rows of the table of the products (`tp.review`): the level and the note of each row, checked with the rules of
//! `LIVELLI.decide` (`mzlab/web/livelli.js`) against the level the interpretation proposes (`proposal.sublevel` of the site), written as `review/1`.
//! The programme proposes and the chemist decides: a decision the rules refuse is listed with its reason and never counted.

use std::collections::BTreeMap;

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::json::{parse, Out, Value};

use super::store;
use crate::store_read::{read_table_from, Slice};

pub const SCHEMA: &str = "review/1";
/// The schema of the decisions the page keeps (`tpdec/1`): `{"rows": {rowKey: {"l": level, "n": note}}}`.
pub const DECISIONS_SCHEMA: &str = "tpdec/1";
/// The levels that can be decided, in the order of `LIVELLI.ORDER`.
pub const LEVELS: [&str; 8] = ["1", "2a", "2b", "3a", "3b", "4", "5", "0"];
/// The decision «not a TP».
pub const NOT_TP: &str = "nottp";
const NOTE_MAX: usize = 200;

/// Why the rules refuse `level` on a row proposed at `proposed` (`""` when it has none), `None` when it is allowed. A standard is never marked here, so 1 is refused.
pub fn refusal(proposed: &str, level: &str) -> Option<&'static str> {
    if level.is_empty() || level == NOT_TP {
        return None;
    }
    if !LEVELS.contains(&level) {
        return Some("unknown level");
    }
    if level == "2b" && !["2a", "3a", "3b", "4"].contains(&proposed) {
        return Some("2b needs a row proposed at 2a, 3a, 3b or 4");
    }
    if level == "1" {
        return Some("1 needs a standard marked");
    }
    None
}

/// The key of a row that stays the same between two equal runs: m/z at 0.0001 and RT at 0.05 min (`TPDEC.rowKey` of the page, high resolution).
pub fn row_key(mz: f64, rt: f64) -> String {
    format!("{:.4}@{:.2}", mz, (rt / 0.05).round() * 0.05)
}

fn proposed_of<R: ReadAt>(r: &mut R, size: u64, id: u32) -> Result<String> {
    let Some(text) = store::read_row(r, size, id)? else {
        return Ok(String::new());
    };
    let doc = parse(text.as_bytes())?;
    let sub = doc
        .get("site")
        .and_then(|s| s.get("proposal"))
        .and_then(|p| p.get("sublevel"))
        .and_then(Value::str);
    Ok(sub.unwrap_or("").to_string())
}

/// `review/1` of the `tpdec/1` text `decisions` over the `.mzfeat` `tp` (the rows of its table and the block `interp`).
pub fn review_text(tp: &[u8], decisions: &str) -> Result<String> {
    let doc = parse(decisions.as_bytes())?;
    if doc.get("schema").and_then(Value::str) != Some(DECISIONS_SCHEMA) {
        return Err(
            Error::new("error.interp.failed").with("detail", "the decisions are not tpdec/1")
        );
    }
    let size = tp.len() as u64;
    let (table, _) = read_table_from(&mut Slice(tp), size)?;
    let mut keys: BTreeMap<String, usize> = BTreeMap::new();
    for i in 0..table.mz.len() {
        keys.entry(row_key(table.mz[i], table.rt[i])).or_insert(i);
    }
    let mut asked: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
    if let Some(rows) = doc.get("rows") {
        for (k, d) in rows.entries() {
            let level = d.get("l").and_then(Value::str).unwrap_or("");
            let note = d.get("n").and_then(Value::str).unwrap_or("");
            asked.insert(k, (level, note));
        }
    }
    let mut out = Out::default();
    out.raw("{\"schema\":").str(SCHEMA).raw(",\"rows\":[");
    let (mut decided, mut not_tp, mut refused, mut unmatched) = (0, 0, 0, 0);
    for (n, (key, (level, note))) in asked.iter().enumerate() {
        let note: String = note.trim().chars().take(NOTE_MAX).collect();
        let row = keys.get(*key).copied();
        let proposed = match row {
            Some(i) => proposed_of(&mut Slice(tp), size, i as u32)?,
            None => String::new(),
        };
        let (status, reason) = match (row, refusal(&proposed, level)) {
            (None, _) => ("unmatched", ""),
            (_, Some(why)) => ("refused", why),
            _ => ("ok", ""),
        };
        match status {
            "unmatched" => unmatched += 1,
            "refused" => refused += 1,
            _ if *level == NOT_TP => not_tp += 1,
            _ if !level.is_empty() => decided += 1,
            _ => {}
        }
        if n > 0 {
            out.raw(",");
        }
        out.raw("{\"key\":").str(key).raw(",\"id\":");
        match row {
            Some(i) => out.int(i as i64),
            None => out.raw("null"),
        };
        out.raw(",\"proposed\":").str(&proposed);
        out.raw(",\"level\":")
            .str(level)
            .raw(",\"note\":")
            .str(&note);
        out.raw(",\"status\":")
            .str(status)
            .raw(",\"reason\":")
            .str(reason);
        out.raw("}");
    }
    out.raw("],\"summary\":{\"decided\":").int(decided);
    out.raw(",\"not_tp\":").int(not_tp);
    out.raw(",\"refused\":").int(refused);
    out.raw(",\"unmatched\":").int(unmatched).raw("}}");
    Ok(out.0)
}
