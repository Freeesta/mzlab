//! The check-OK / run-fail pairs of `flow_pairs_tests` the registry has today, the verdict of a pair and the report: kept apart so the harness stays readable.

use std::collections::BTreeSet;

/// The mismatches the registry has today, by consumer port, source and error key: the test fails on any other, and on one of these that is gone (drop it).
/// - a `filtri/1` (`tp.filter`) is typed `table` like a `.mzfeat`;
/// - `qc.mass_accuracy` writes `table` only when its optional `table` input is wired;
/// - `spectra.match.query` accepts `report`, and the CSV exports are no spectra;
/// - `spectra.msn_graph.molecules` (a molecules CSV) accepts `table`, and a `.mzfeat` is no CSV;
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
        "features.gapfill.table <- qc.mass_accuracy.table",
        "err.flow.lineage",
    ),
    (
        "features.gapfill.table <- tp.filter.table",
        "err.flow.lineage",
    ),
    (
        "table.filter.table <- table of one sample",
        "error.badCampioni",
    ),
    (
        "table.filter.table <- qc.mass_accuracy.table",
        "err.flow.port.missing",
    ),
    ("table.filter.table <- tp.filter.table", "error.badMzfeat"),
    (
        "features.group.table <- qc.mass_accuracy.table",
        "err.flow.port.missing",
    ),
    ("features.group.table <- tp.filter.table", "error.badMzfeat"),
    (
        "table.stats.table <- qc.mass_accuracy.table",
        "err.flow.port.missing",
    ),
    ("table.stats.table <- tp.filter.table", "error.badMzfeat"),
    (
        "table.kmd.table <- qc.mass_accuracy.table",
        "err.flow.port.missing",
    ),
    ("table.kmd.table <- tp.filter.table", "error.badMzfeat"),
    (
        "table.plausible.table <- qc.mass_accuracy.table",
        "err.flow.port.missing",
    ),
    (
        "table.plausible.table <- tp.filter.table",
        "error.badMzfeat",
    ),
    (
        "export.table_csv.table <- table of one sample",
        "error.badCampioni",
    ),
    (
        "export.table_csv.table <- qc.mass_accuracy.table",
        "err.flow.port.missing",
    ),
    (
        "export.table_csv.table <- tp.filter.table",
        "err.flow.export.view",
    ),
    (
        "export.mztab.table <- table of one sample",
        "error.badCampioni",
    ),
    (
        "export.mztab.table <- qc.mass_accuracy.table",
        "err.flow.port.missing",
    ),
    (
        "export.mztab.table <- tp.filter.table",
        "err.flow.export.view",
    ),
    (
        "spectra.chimeric.features <- qc.mass_accuracy.table",
        "err.flow.port.missing",
    ),
    (
        "spectra.chimeric.features <- tp.filter.table",
        "err.flow.export.view",
    ),
    (
        "spectra.match.query <- export.table_csv.report",
        "err.flow.match.format",
    ),
    (
        "spectra.match.query <- export.mztab.report",
        "err.flow.match.format",
    ),
    (
        "spectra.match.query <- features.table.report",
        "err.flow.match.format",
    ),
    (
        "spectra.match.query <- export.annotations.report",
        "err.flow.match.format",
    ),
    (
        "spectra.match.query <- export.inclusion.report",
        "err.flow.match.format",
    ),
    ("tp.link.table <- table with no row", "err.flow.lineage"),
    ("tp.link.table <- table of one sample", "err.flow.lineage"),
    (
        "tp.link.table <- qc.mass_accuracy.table",
        "err.flow.lineage",
    ),
    ("tp.link.table <- tp.filter.table", "err.flow.lineage"),
    ("spectra.msn_graph.molecules <- gap.table", "err.flow.io"),
    ("spectra.msn_graph.molecules <- ali.table", "err.flow.io"),
    (
        "spectra.msn_graph.molecules <- table with no row",
        "err.flow.io",
    ),
    (
        "spectra.msn_graph.molecules <- table of one sample",
        "err.flow.io",
    ),
    (
        "spectra.msn_graph.molecules <- features.align.table",
        "err.flow.io",
    ),
    (
        "spectra.msn_graph.molecules <- features.gapfill.table",
        "err.flow.io",
    ),
    (
        "spectra.msn_graph.molecules <- table.filter.table",
        "err.flow.io",
    ),
    (
        "table.match.table <- table of one sample",
        "error.badCampioni",
    ),
    (
        "table.match.table <- qc.mass_accuracy.table",
        "err.flow.port.missing",
    ),
    (
        "table.match.table <- tp.filter.table",
        "err.flow.export.view",
    ),
    (
        "export.annotations.table <- qc.mass_accuracy.table",
        "err.flow.port.missing",
    ),
    (
        "export.annotations.table <- tp.filter.table",
        "error.badMzfeat",
    ),
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
