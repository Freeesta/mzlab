//! The English text of an engine error for the terminal: the catalogue the page shows (`mzlab/web/lang/en/errors-*.js`) with its `{param}` filled in,
//! and the key kept in brackets so the message can still be searched or matched by scripts. A message that does not start with a known key passes
//! unchanged (usage errors, file-system errors).

const CATALOG: &str = concat!(
    include_str!("../../../mzlab/web/lang/en/errors-a-m.js"),
    include_str!("../../../mzlab/web/lang/en/errors-file.js"),
    include_str!("../../../mzlab/web/lang/en/errors-flow.js"),
    include_str!("../../../mzlab/web/lang/en/errors-llm.js"),
    include_str!("../../../mzlab/web/lang/en/errors-n-w.js"),
);

/// The people's names of the memory phases (`memory.phase.<id>`), the ones the page puts in the same sentence.
const PHASES: &str = include_str!("../../../mzlab/web/lang/en/more-lv-defaults.js");

/// The text of `key` in a catalogue of one `"key": "text",` per line, without escapes.
fn text_in(catalog: &'static str, key: &str) -> Option<&'static str> {
    let sign = format!("\n  \"{key}\": \"");
    let da = catalog.find(&sign)? + sign.len();
    let row = catalog[da..].lines().next()?.trim_end();
    let line = row.strip_suffix(',').unwrap_or(row);
    line.strip_suffix('"')
}

/// The catalogue text of `key`.
/// The catalogue text of `key` (the language of the terminal), `None` when the key has none.
pub(crate) fn catalog_text(key: &str) -> Option<&'static str> {
    text_of(key)
}

fn text_of(key: &str) -> Option<&'static str> {
    text_in(CATALOG, key)
}

/// The key whose text the terminal shows, the same choice as `I18N.err` in the desktop app: a memory refusal that names its phase has the
/// sentence with the numbers (`error.memoryBudgetFase[.<fase>]`), and a key with a `.native` text uses it (the limit is this computer's memory).
fn returned_key(key: &str, phase: Option<&str>) -> String {
    let base = match phase {
        Some(f) if key == "error.memoryBudget" => {
            let own = format!("error.memoryBudgetFase.{f}");
            if text_of(&own).is_some() {
                own
            } else {
                "error.memoryBudgetFase".to_string()
            }
        }
        _ => key.to_string(),
    };
    let native = format!("{base}.native");
    // only these have a native variant (`err.flow.step.native` is a key of its own): the memory sentences and the browser-only wording
    // ("reload the page", "the browser ran short of space"); the list is the one of `NATIVE` in `mzlab/web/i18n.js`
    const NATIVE_KEYS: &[&str] = &[
        "error.badIndex",
        "error.generic",
        "err.engine.indexCrash",
        "err.file.nativeStopped",
    ];
    if (base.starts_with("error.memory") || NATIVE_KEYS.contains(&base.as_str()))
        && text_of(&native).is_some()
    {
        native
    } else {
        base
    }
}

/// The parameters of `Error`'s `Display` form (` k=v k2=v2`): a word that starts with `name=` opens a parameter, the others continue its value.
pub(crate) fn text_params(remainder: &str) -> Vec<(String, String)> {
    params(remainder)
}

fn params(remainder: &str) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = Vec::new();
    for word in remainder.split(' ').filter(|p| !p.is_empty()) {
        let name = word
            .split_once('=')
            .map(|(k, _)| k)
            .filter(|k| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        match (name, v.last_mut()) {
            (Some(k), _) => v.push((k.to_string(), word[k.len() + 1..].to_string())),
            (None, Some((_, val))) => {
                val.push(' ');
                val.push_str(word);
            }
            (None, None) => return Vec::new(),
        }
    }
    v
}

pub(crate) fn human(msg: &str) -> String {
    let (key, remainder) = msg.split_once(' ').unwrap_or((msg, ""));
    if !(key.starts_with("err.") || key.starts_with("error.")) {
        return msg.to_string();
    }
    render(msg, key, remainder)
}

/// A calm notice (the one the desktop app gives its page as `{"key", "params"}`) as the terminal line: «text (key)».
pub fn warning(n: &crate::convert::Notice) -> String {
    // the parameters stay whole (a file name with spaces or `=`), which the text form `key k=v` cannot tell apart
    let par = n
        .params
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    render_params(n.key, n.key, par)
}

/// A notice of the engine documents (`warnings` of `file method`) as the terminal line: «text (key)».
pub(crate) fn notice_text(key: &str, params: Vec<(String, String)>) -> String {
    render_params(key, key, params)
}

/// An entry of `method/1.warnings` (key and place) as the Method window says it: «text (key)», or «key: place» for a key without a text.
pub(crate) fn method_note(key: &str, place: &str) -> String {
    match text_of(key) {
        Some(t) => format!("{} ({key})", t.replace("{place}", place)),
        None => format!("{key}: {place}"),
    }
}

/// An engine error as the terminal says it, from its own key and parameters: a value with spaces and `name=` words (a channel id) stays whole,
/// which the text form `key k=v` cannot tell apart.
pub(crate) fn human_error(e: &mzlab_core::Error) -> String {
    let par = e
        .params
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    render_params(&e.to_string(), e.key, par)
}

impl crate::Outcome {
    /// An engine error as a failure: the code of its key (`exit::code_of`), its text rendered from the error itself (`human_error`)
    /// and its key and parameters for `error/1`.
    pub(crate) fn fail_error(e: &mzlab_core::Error) -> Self {
        crate::Outcome {
            code: crate::exit::code_of(e),
            stdout: Vec::new(),
            stderr: format!("error: {}\n", human_error(e)),
            error: Some(Box::new(crate::Failure {
                key: e.key.to_string(),
                params: e
                    .params
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
                ..crate::Failure::default()
            })),
        }
    }
}

/// `msg` with its leading error key rendered: «text (key, unused=params)».
fn render(msg: &str, key: &str, remainder: &str) -> String {
    render_params(msg, key, params(remainder))
}

fn render_params(msg: &str, key: &str, par: Vec<(String, String)>) -> String {
    let phase = par
        .iter()
        .find(|(k, _)| k == "fase")
        .map(|(_, v)| v.as_str());
    let Some(text) = text_of(&returned_key(key, phase)) else {
        return msg.to_string();
    };
    let mut t = text.to_string();
    let mut rest = String::new();
    for (k, v) in par {
        let sign = format!("{{{k}}}");
        if t.contains(&sign) {
            let name = (k == "fase")
                .then(|| text_in(PHASES, &format!("memory.phase.{v}")))
                .flatten();
            t = t.replace(&sign, name.unwrap_or(&v));
        } else {
            rest += &format!(", {k}={v}");
        }
    }
    format!("{t} ({key}{rest})")
}

/// A usage, input or selector failure (codes 2, 4, 5) ends with the help of the command that was run: `(mzlab-cli file channels --help)`, not another's.
pub(crate) fn with_help(args: &[String], mut o: crate::Outcome) -> crate::Outcome {
    if !matches!(o.code, 2 | 4 | 5)
        || !o.stderr.starts_with("error: ")
        || o.stderr.contains("--help)")
    {
        return o;
    }
    if o.stderr.starts_with("error: unknown command") {
        o.stderr += "(mzlab-cli --help)\n";
        return o;
    }
    let command = match crate::cmd::find(args) {
        Some((e, _)) => format!("{} ", e.name),
        None => match args.first() {
            Some(_) => crate::cmd::group_of(args).map_or(String::new(), |(p, _)| format!("{p} ")),
            None => String::new(),
        },
    };
    o.stderr += &format!("(mzlab-cli {command}--help)\n");
    o
}

/// What `main` answers when the program panics (the release profile aborts, so a hook answers before the abort): the failure `err.run.internal`,
/// exit 1, one `error/1` on stdout under `--json`. A command should refuse bad input with 2 or 4 before it gets here; this is the net under a bug.
pub fn panic_outcome(args: &[String], detail: &str) -> crate::Outcome {
    let detail: String = detail.chars().take(300).collect();
    let o = crate::Outcome::fail_error(
        &mzlab_core::Error::new("err.run.internal").with("detail", detail),
    )
    .with_hint("this is a bug of mzlab-cli: report the command line, no data is needed");
    let command = crate::cmd::find(args).map_or("", |(e, _)| e.name);
    with_help(args, crate::contract::finish(args, command, o))
}

#[path = "stdout.rs"]
mod stdout;
pub use stdout::write_stdout;

#[cfg(test)]
#[path = "error_text_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "no_panic_tests.rs"]
mod no_panic_tests;
