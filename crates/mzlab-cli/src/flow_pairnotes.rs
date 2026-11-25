//! The log of the pairing of the samples with the files of a table (`samples::order_for_roles`): one stderr line per automatic choice, so the user sees
//! which sample was left out and which role a name gave, and the hint that names the commands writing a `samples/1`.

use mzlab_core::samples::PairNote;

use crate::samples_import::say;

/// One line per note (texts are `err.cli.pair.*` of `lang/en`).
pub(crate) fn lines(notes: &[PairNote]) -> Vec<String> {
    let rows: Vec<[&str; 4]> = notes
        .iter()
        .map(|n| [n.action, &n.file, &n.role, &n.evidence])
        .collect();
    rows_lines(&rows)
}

/// The lines of the `pairing` array of a result document (`{"pairing":[{"action","file","role","evidence"}]}`): what the run of a step that only has its JSON says.
pub(crate) fn json_lines(result: &str) -> Vec<String> {
    let Ok(v) = mzlab_core::json::parse(result.as_bytes()) else {
        return Vec::new();
    };
    let text = |n: &mzlab_core::json::Value, k: &str| {
        n.get(k).and_then(|x| x.str()).unwrap_or_default().to_string()
    };
    let owned: Vec<[String; 4]> = v
        .get("pairing")
        .map(|p| p.arr())
        .unwrap_or_default()
        .iter()
        .map(|n| [text(n, "action"), text(n, "file"), text(n, "role"), text(n, "evidence")])
        .collect();
    let rows: Vec<[&str; 4]> = owned
        .iter()
        .map(|r| [r[0].as_str(), r[1].as_str(), r[2].as_str(), r[3].as_str()])
        .collect();
    rows_lines(&rows)
}

/// `[action, file, role, evidence]` to the line of its key: a left-out sample, a time zero read from the minutes, a role read from a word, a role read from the name with
/// no single word (the lone `D` of a dark control), or a plain sample (the name has no role word).
fn rows_lines(rows: &[[&str; 4]]) -> Vec<String> {
    rows.iter()
        .map(|[action, file, role, evidence]| {
            let key = match (*action, *role, evidence.is_empty()) {
                ("left_out", _, _) => "err.cli.pair.leftOut",
                (_, "t0", false) => "err.cli.pair.proposedTime",
                (_, _, false) => "err.cli.pair.proposed",
                (_, "sample", true) => "err.cli.pair.proposedNone",
                (_, _, true) => "err.cli.pair.proposedName",
            };
            let v = [("file", file.to_string()), ("role", role.to_string()), ("part", evidence.to_string())];
            say(key, &v)
        })
        .collect()
}

/// The lines on stderr.
pub(crate) fn print(notes: &[PairNote]) {
    for l in lines(notes) {
        eprintln!("{l}");
    }
}

/// The lines of the `pairing` of a result document on stderr.
pub(crate) fn print_json(result: &str) {
    for l in json_lines(result) {
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
