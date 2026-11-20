//! What `features` chose by itself for the first file, from the one engine note (`mzlab_ops::run::feat::lr_note`, the function behind the notices of
//! `estimates/1`): said on stderr and kept as `features-notes/1`, which `--notes PATH` copies out. stdout (the CSV) is untouched.

use std::path::Path;

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
        out.push(format!(
            "warning: {name}: {}",
            crate::error_text::warning(&notice)
        ));
    }
    out
}

/// The `features-notes/1` text: the axis of the recipe, its unit-resolution numbers, the scans counted and the `notices` (key and parameters, the
/// members `estimates/1` carries), the same facts as the stderr lines.
pub(crate) fn notes_text(name: &str, n: &Note) -> String {
    let num = |x: f64| value_text(&Value::Num(x));
    let mut o = Out::default();
    o.raw("{\"schema\":\"features-notes/1\",\"file\":")
        .str(name);
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
    o.raw(&format!(",\"notices\":{}}}\n", notices_text(&n.notices)));
    o.0
}

/// The note of the first file: its lines on stderr, and `features-notes/1` written next to its index (`IDX` with the extension `notes.json`), where
/// the command finds it for `--notes`. Nothing when the index cannot be opened: the run is not about to fail on a note.
pub(crate) fn report(recipe: &Recipe, idx: &Path, file: &Path) {
    let Ok(index) = crate::open(idx) else { return };
    let (name, note) = (file.display().to_string(), note_of(recipe, &index));
    for l in lines(&name, &note) {
        eprintln!("{l}");
    }
    let _ = std::fs::write(notes_path(idx), notes_text(&name, &note));
}

/// Where `report` keeps the note for the index `idx`.
pub(crate) fn notes_path(idx: &Path) -> std::path::PathBuf {
    idx.with_extension("notes.json")
}

#[cfg(test)]
#[path = "features_note_tests.rs"]
mod tests;
