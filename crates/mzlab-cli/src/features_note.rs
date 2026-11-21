//! What `features` chose by itself for the first file, from the one engine note (`mzlab_ops::run::feat::lr_note`, the function behind the notices of
//! `estimates/1`): said on stderr and kept as `features-notes/1`, which `--notes` (or `--out`, beside the table) keeps. stdout (the CSV) is untouched.

use std::path::{Path, PathBuf};

use mzlab_core::flow::value_text;
use mzlab_core::json::{Out, Value};
use mzlab_feat::recipe::Recipe;
use mzlab_ops::run::feat::lr_note::{note_of, notices_text, Note};

use crate::convert::Notice;

/// The lines for stderr, none for a ppm-axis file: the progress-style line of the recipe and, for each notice, its warning text.
pub(crate) fn lines(name: &str, n: &Note) -> Vec<String> {
    if !n.unit {
        return Vec::new();
    }
    let mut out = vec![format!(
        "recipe {name}: unit resolution (1 Da channels centred on n{:+.3}), floor {} counts, minimum height {} counts, {} of {} MS1 scans in profile mode",
        n.defect, n.floor, n.min_height, n.profile, n.scans
    )];
    for e in &n.notices {
        let notice = Notice {
            key: e.key,
            params: e.params.iter().map(|(k, v)| (*k, v.to_string())).collect(),
        };
        let kind = if e.key.starts_with("note.") {
            "note"
        } else {
            "warning"
        };
        out.push(format!(
            "{kind}: {name}: {}",
            crate::error_text::warning(&notice)
        ));
    }
    out
}

/// The `features-notes/1` text: one entry per file of the series, in the order given: its name, the axis of the recipe (one for the series), its
/// unit-resolution numbers, the scans counted on that file and its `notices` (key and parameters, the members `estimates/1` carries), the same facts as
/// the stderr lines.
pub(crate) fn notes_text(notes: &[(String, Note)]) -> String {
    let num = |x: f64| value_text(&Value::Num(x));
    let mut o = Out::default();
    o.raw("{\"schema\":\"features-notes/1\",\"files\":[");
    for (k, (name, n)) in notes.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" });
        o.raw("{\"file\":").str(name);
        o.raw(if n.unit {
            ",\"axis\":\"Da\""
        } else {
            ",\"axis\":\"ppm\""
        });
        o.raw(&format!(",\"defect\":{}", num(n.defect)));
        o.raw(&format!(",\"floor\":{}", num(n.floor)));
        o.raw(&format!(",\"min_height\":{}", num(n.min_height)));
        o.raw(&format!(
            ",\"profile_scans\":{},\"ms1_scans\":{}",
            n.profile, n.scans
        ));
        o.raw(&format!(",\"notices\":{}}}", notices_text(&n.notices)));
    }
    o.raw("]}\n");
    o.0
}

/// The note of every file of the series: its lines on stderr, and `features-notes/1` written next to the first index (`IDX` with the extension
/// `notes.json`), where the command finds it for `--notes` and `--out`. A file whose index cannot be opened is left out: the run is not about to fail on a note.
pub(crate) fn report(recipe: &Recipe, idxs: &[PathBuf], files: &[PathBuf], labels: &[String]) {
    let mut notes = Vec::new();
    for (k, idx) in idxs.iter().enumerate() {
        let Ok(index) = crate::open(idx) else {
            continue;
        };
        let name = match labels.get(k).filter(|l| !l.is_empty()) {
            Some(l) => l.clone(),
            None => files[k].display().to_string(),
        };
        let note = note_of(recipe, &index);
        for l in lines(&name, &note) {
            eprintln!("{l}");
        }
        notes.push((name, note));
    }
    let _ = std::fs::write(notes_path(&idxs[0]), notes_text(&notes));
}

/// Where `report` keeps the note for the index `idx`.
pub(crate) fn notes_path(idx: &Path) -> PathBuf {
    idx.with_extension("notes.json")
}

#[cfg(test)]
#[path = "features_note_tests.rs"]
mod tests;
