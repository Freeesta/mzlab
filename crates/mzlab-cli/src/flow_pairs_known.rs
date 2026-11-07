//! The check-OK / run-fail pairs of `flow_pairs_tests` the registry has today, the verdict of a pair and the report: kept apart so the harness stays readable.

use std::collections::BTreeSet;

/// The mismatches the registry has today, by consumer port, source and error key: the test fails on any other, and on one of these that is gone (drop it).
/// - `features.gapfill` and `tp.link` need a table aligned in the same plan (`err.flow.lineage`);
/// - a table of one file with the samples of six (`error.badCampioni`): data the check does not read.
const KNOWN: &[(&str, &str)] = &[
    (
        "features.gapfill.table <- table with no row",
        "err.flow.lineage",
    ),
    (
        "features.gapfill.table <- table of one sample",
        "err.flow.lineage",
    ),
    (
        "table.filter.table <- table of one sample",
        "error.badCampioni",
    ),
    (
        "export.table_csv.table <- table of one sample",
        "error.badCampioni",
    ),
    (
        "export.mztab.table <- table of one sample",
        "error.badCampioni",
    ),
    ("tp.link.table <- table with no row", "err.flow.lineage"),
    ("tp.link.table <- table of one sample", "err.flow.lineage"),
    (
        "table.match.table <- table of one sample",
        "error.badCampioni",
    ),
];

/// The pairs the formats of the ports close (`format` / `formats`, `err.flow.port.format`): the check refuses each, with that key.
const BY_FORMAT: &[&str] = &[
    "export.annotations.table <- tp.filter.table",
    "export.mztab.table <- tp.filter.table",
    "export.table_csv.table <- tp.filter.table",
    "features.gapfill.table <- tp.filter.table",
    "features.group.table <- tp.filter.table",
    "spectra.chimeric.features <- tp.filter.table",
    "spectra.match.query <- export.annotations.report",
    "spectra.match.query <- export.inclusion.report",
    "spectra.match.query <- export.mztab.report",
    "spectra.match.query <- export.table_csv.report",
    "spectra.match.query <- features.table.report",
    "spectra.msn_graph.molecules <- ali.table",
    "spectra.msn_graph.molecules <- features.align.table",
    "spectra.msn_graph.molecules <- features.gapfill.table",
    "spectra.msn_graph.molecules <- gap.table",
    "spectra.msn_graph.molecules <- table of one sample",
    "spectra.msn_graph.molecules <- table with no row",
    "spectra.msn_graph.molecules <- table.filter.table",
    "table.filter.table <- tp.filter.table",
    "table.kmd.table <- tp.filter.table",
    "table.match.table <- tp.filter.table",
    "table.plausible.table <- tp.filter.table",
    "table.stats.table <- tp.filter.table",
    "tp.link.table <- tp.filter.table",
];

/// Result of a consumer reading one source on one port.
pub(super) enum Verdict {
    Refused(String),
    Ran,
    /// The canonical input of the port fails too: the pair says nothing.
    Uncovered(String),
    Mismatch(String),
}

/// Writes the report and fails on a mismatch not in `KNOWN`, on a `KNOWN` entry that no longer fails, and on a view the check refuses or the run fails.
pub(super) fn report_and_assert(verdicts: &[(String, Verdict)]) {
    let mut report = String::new();
    for (what, v) in verdicts {
        let (kind, why) = match v {
            Verdict::Refused(w) => ("refused by the check", w.as_str()),
            Verdict::Ran => ("ran", ""),
            Verdict::Uncovered(w) => ("not covered", w.as_str()),
            Verdict::Mismatch(w) => ("MISMATCH", w.as_str()),
        };
        report.push_str(&format!("{what}\t{kind}\t{why}\n"));
    }
    std::fs::write(std::env::temp_dir().join("mzlab_pairs_report.tsv"), &report).unwrap();
    let found: BTreeSet<(String, String)> = verdicts
        .iter()
        .filter_map(|(w, v)| match v {
            Verdict::Mismatch(e) => Some((w.clone(), e.clone())),
            _ => None,
        })
        .collect();
    let known: BTreeSet<(String, String)> = KNOWN
        .iter()
        .map(|(w, e)| (w.to_string(), e.to_string()))
        .collect();
    let new: Vec<_> = found
        .difference(&known)
        .map(|(w, e)| format!("{w}: {e}"))
        .collect();
    let gone: Vec<_> = known
        .difference(&found)
        .map(|(w, e)| format!("{w}: {e}"))
        .collect();
    assert!(
        new.is_empty(),
        "check passes, run fails:\n{}",
        new.join("\n")
    );
    assert!(
        gone.is_empty(),
        "no longer a mismatch, drop it from KNOWN:\n{}",
        gone.join("\n")
    );
    for want in BY_FORMAT {
        let v = verdicts.iter().find(|(w, _)| w == want).map(|(_, v)| v);
        assert!(
            matches!(v, Some(Verdict::Refused(why)) if why.contains("err.flow.port.format")),
            "{want}: not refused by its format"
        );
    }
    // the view of `table.filter` runs wherever the check takes a table
    for (w, v) in verdicts {
        if w.ends_with("<- table.filter.table") && !w.starts_with("spectra.msn_graph") {
            assert!(
                !matches!(v, Verdict::Mismatch(_) | Verdict::Refused(_)),
                "{w}"
            );
        }
    }
}
