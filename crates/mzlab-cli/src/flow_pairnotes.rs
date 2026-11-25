//! The log of the pairing of the samples with the files of a table (`samples::order_for_roles`): one stderr line per automatic choice, so the user sees
//! which sample was left out and which role a name gave, and the hint that names the commands writing a `samples/1`.

use mzlab_core::samples::PairNote;

use crate::samples_import::say;

/// One line per note (texts are `err.cli.pair.*` of `lang/en`).
pub(crate) fn lines(notes: &[PairNote]) -> Vec<String> {
    notes
        .iter()
        .map(|n| {
            let v = |part: &'static str| {
                vec![
                    ("file", n.file.clone()),
                    ("role", n.role.clone()),
                    (part, n.evidence.clone()),
                ]
            };
            match (n.action, n.evidence.is_empty()) {
                ("left_out", _) => say("err.cli.pair.leftOut", &v("part")),
                (_, false) if n.role == "t0" => say("err.cli.pair.proposedTime", &v("part")),
                (_, false) => say("err.cli.pair.proposed", &v("part")),
                (_, true) => say("err.cli.pair.proposedNone", &v("part")),
            }
        })
        .collect()
}

/// The lines on stderr.
pub(crate) fn print(notes: &[PairNote]) {
    for l in lines(notes) {
        eprintln!("{l}");
    }
}

/// The hint of a failed run whose step refused its samples (`error.badSamples`); empty for any other key.
pub(crate) fn hint(error_key: &str) -> String {
    if error_key == "error.badSamples" {
        say("err.cli.samples.hint", &[])
    } else {
        String::new()
    }
}
