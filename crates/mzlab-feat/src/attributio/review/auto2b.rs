//! The optional automatic 2b (`[levels] auto_2b = true` in `campaign.toml`, off by default): a Level 3 feature whose proposal of the review is `2b-strong`
//! gets a decision row with `decided_by = auto`. The rows are an ordinary decisions table placed BEFORE the chemist's own, so any row of the chemist
//! supersedes them (deleting an `auto` row restores the programme's level); `annotate::decide` marks those features `auto-2b` and keeps them in the audit log.

use super::banco::Run;
use crate::attributio::annotate::Feature;
use crate::attributio::error::Result;
use crate::attributio::io::Io;
use crate::attributio::readers::feature_decisions::{DecisionTable, COLUMNS};

/// `decided_by` of the rows this module writes.
pub const AUTO: &str = "auto";

/// The proposal tier that earns an automatic 2b.
const STRONG: &str = "2b-strong";

/// The decision rows for the `(feature, tier, reasons)` items whose tier is `2b-strong` and that are Level 3 without the chemist's 3b.
pub fn auto_rows(items: &[(&Feature, &str, &[String])]) -> DecisionTable {
    let mut t = DecisionTable {
        columns: COLUMNS.iter().map(|c| c.to_string()).collect(),
        rows: Vec::new(),
    };
    for (f, tier, reasons) in items {
        if *tier != STRONG || f.level != 3 || f.level_sub == "b" || f.decision.decision_2b {
            continue;
        }
        let why = format!("auto 2b ({STRONG}): {}", reasons.join("; "));
        t.rows.push(
            [
                f.pol.clone(),
                f.id.to_string(),
                f.mz.map_or(String::new(), |v| v.to_string()),
                f.rt.map_or(String::new(), |v| v.to_string()),
                "2b".into(),
                String::new(),
                String::new(),
                String::new(),
                AUTO.into(),
                why,
            ]
            .to_vec(),
        );
    }
    t
}

/// `user` with the rows of `auto` in front: the file is a history in which a later row supersedes the earlier ones, so the chemist always has the last word.
pub fn with_auto(user: &DecisionTable, auto: &DecisionTable) -> DecisionTable {
    let mut columns = user.columns.clone();
    for c in &auto.columns {
        if !columns.contains(c) {
            columns.push(c.clone());
        }
    }
    let cells = |t: &DecisionTable, r: usize| -> Vec<String> {
        columns.iter().map(|c| t.get(r, c).to_string()).collect()
    };
    let rows = (0..auto.len())
        .map(|r| cells(auto, r))
        .chain((0..user.len()).map(|r| cells(user, r)))
        .collect();
    DecisionTable { columns, rows }
}

/// The automatic 2b rows of the campaign at the root of `io`: empty unless `[levels] auto_2b = true`.
pub fn auto_2b_table(io: &Io, root: &str) -> Result<DecisionTable> {
    let run = Run::load(io, root)?;
    if !run.b.campaign.auto_2b {
        return Ok(auto_rows(&[]));
    }
    let proposals: Vec<_> = run
        .b
        .features
        .iter()
        .map(|f| (f, run.proposal(io, f).0))
        .collect();
    let items: Vec<(&Feature, &str, &[String])> = proposals
        .iter()
        .map(|(f, p)| (*f, p.class.as_str(), p.reasons.as_slice()))
        .collect();
    Ok(auto_rows(&items))
}
