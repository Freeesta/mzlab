//! The agent contract of `mzlab-cli`, applied once for every command after it ran: `error/1` on stdout for a failure under `--json` (or for a
//! command whose stdout is JSON), a trailing newline on every text document, and the stderr text without the `error:` prefix as `message`.

use crate::{Failure, Outcome};

/// The failure of `o` as `error/1` needs it: the data the command gave, or a generic usage failure carrying its text.
fn failure_of(o: &mut Outcome, message: &str) -> Failure {
    match o.error.take() {
        Some(f) => *f,
        None => Failure {
            key: crate::exit::generic_key(o.code).to_string(),
            params: vec![("message".to_string(), message.to_string())],
            ..Failure::default()
        },
    }
}

/// The text of the failure: stderr without the `error: ` prefix, the `hint:` line and the closing `(mzlab-cli ... --help)` pointer.
fn message_of(stderr: &str) -> (String, String) {
    let body = stderr.strip_prefix("error: ").unwrap_or(stderr);
    let (msg, hint) = match body.split_once("\nhint: ") {
        Some((m, h)) => (m, h.lines().next().unwrap_or("")),
        None => (body, ""),
    };
    let msg = msg
        .lines()
        .take_while(|l| !(l.starts_with("(mzlab-cli ") && l.ends_with("--help)")))
        .collect::<Vec<_>>()
        .join("\n");
    (msg.trim_end().to_string(), hint.to_string())
}

/// The verdict commands, by command path: their exit 1 or 3 is a verdict of the answer they print (the listing has refused samples, the pair lacks
/// its companion, the file has errors, some rows of the list are wrong, a regulatory list stayed stale, a structure file needed a change under `--check`), the same document the site computes byte for byte. It stays on stdout and
/// carries its own verdict; every other exit of these, and every other command's non-zero exit, is one `error/1`.
pub(crate) const VERDICT: &[&str] = &[
    "file validate",
    "file pair",
    "file inventory",
    "file verify",
    "file unpack",
    "file check-pack",
    "raw info",
    "raw cache",
    "raw xic",
    "raw scan",
    "raw isotopes",
    "sample list",
    "sample import",
    "sample pair",
    "db ref-list",
    "llm eval",
    "db normative build",
    "mol open",
];

/// Whether the arguments start with the path of a verdict command (`sample list ...`, `file verify ...`).
pub(crate) fn is_verdict(args: &[String]) -> bool {
    VERDICT.iter().any(|v| {
        let words: Vec<&str> = v.split(' ').collect();
        args.len() >= words.len() && args.iter().zip(&words).all(|(a, w)| a == w)
    })
}

/// `o` with the contract applied; `command` is the name of the entry that ran (`""` when none matched).
pub(crate) fn finish(args: &[String], command: &str, mut o: Outcome) -> Outcome {
    let json = args.iter().any(|a| a == "--json") || crate::cmd::json_by_default(command);
    // `sample list` also answers exit 8 with its listing when the pair disagrees with itself and a member is still being written (`consistency`); a whole-run
    // refusal of a growing pair carries its own `error` and stays an `error/1`
    let growing_listing =
        o.code == crate::exit::GROWING && command == "sample list" && o.error.is_none();
    let verdict = is_verdict(args)
        && (matches!(o.code, crate::exit::DATA | crate::exit::INTEGRITY) || growing_listing)
        && !o.stdout.is_empty();
    if o.code != 0
        && json
        && !verdict
        && !String::from_utf8_lossy(&o.stdout).contains("\"schema\":\"error/1\"")
    {
        let (message, hint) = message_of(&o.stderr);
        let mut f = failure_of(&mut o, &message);
        if f.hint.is_empty() {
            f.hint = hint;
        }
        // what the command printed before it failed (a report, a partial listing) rides in the document as `report`
        let report = String::from_utf8(std::mem::take(&mut o.stdout))
            .ok()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty() && mzlab_core::json::parse(t.as_bytes()).is_ok());
        o.stdout = crate::flow_json::error_json(o.code, &f, &message, command, report.as_deref())
            .into_bytes();
        o.error = Some(Box::new(f));
    }
    if !o.stdout.is_empty() && !o.stdout.ends_with(b"\n") && std::str::from_utf8(&o.stdout).is_ok()
    {
        o.stdout.push(b'\n');
    }
    o
}

/// The failure of `--json` given to a command that has no JSON form.
pub(crate) fn json_unsupported() -> Outcome {
    Outcome::fail_error(&mzlab_core::Error::new("err.cli.json.unsupported"))
        .with_hint("run the command without --json")
}

/// One `error/1` line for a failure that is part of a longer answer (a file of several that cannot be listed); the dispatcher adds none then.
pub(crate) fn error_line(code: u8, e: &mzlab_core::Error, hint: &str, command: &str) -> String {
    let f = Failure {
        key: e.key.to_string(),
        params: e
            .params
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        hint: hint.to_string(),
        problems: None,
    };
    crate::flow_json::error_json(code, &f, &crate::error_text::human_error(e), command, None)
}
