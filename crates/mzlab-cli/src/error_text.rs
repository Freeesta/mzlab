//! The English text of an engine error for the terminal: the catalogue the page shows (`mzlab/web/lang/en/errori.js`) with its `{param}` filled in,
//! and the key kept in brackets so the message can still be searched or matched by scripts. A message that does not start with a known key passes
//! unchanged (usage errors, file-system errors).

const CATALOG: &str = include_str!("../../../mzlab/web/lang/en/errori.js");

/// The people's names of the memory phases (`memory.phase.<id>`), the ones the page puts in the same sentence.
const PHASES: &str = include_str!("../../../mzlab/web/lang/en/altre-lv-predef.js");

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
fn returned_key(key: &str, fase: Option<&str>) -> String {
    let base = match fase {
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
    if text_of(&native).is_some() {
        native
    } else {
        base
    }
}

/// The parameters of `Error`'s `Display` form (` k=v k2=v2`): a word that starts with `name=` opens a parameter, the others continue its value.
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

/// `msg` with its leading error key rendered: «text (key, unused=params)».
pub(crate) fn human(msg: &str) -> String {
    let (key, remainder) = msg.split_once(' ').unwrap_or((msg, ""));
    if !(key.starts_with("err.") || key.starts_with("error.")) {
        return msg.to_string();
    }
    render(msg, key, remainder)
}

/// A calm notice (the one the desktop app gives its page as `{"key", "params"}`) as the terminal line: «text (key)».
pub fn warning(n: &crate::convert::Notice) -> String {
    let remainder: String = n.params.iter().map(|(k, v)| format!(" {k}={v}")).collect();
    render(n.key, n.key, &remainder)
}

fn render(msg: &str, key: &str, remainder: &str) -> String {
    let par = params(remainder);
    let fase = par
        .iter()
        .find(|(k, _)| k == "fase")
        .map(|(_, v)| v.as_str());
    let Some(text) = text_of(&returned_key(key, fase)) else {
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

/// A usage or engine error (`Outcome::fail`, code 2) ends with the help of the command that was run: `(mzlab-cli channels --help)`, not another's.
pub(crate) fn with_help(args: &[String], mut o: crate::Outcome) -> crate::Outcome {
    if o.code != 2 || !o.stderr.starts_with("error: ") || o.stderr.contains("--help)") {
        return o;
    }
    if o.stderr.starts_with("error: unknown command") {
        o.stderr += "(mzlab-cli --help)\n";
        return o;
    }
    let word = |i: usize| {
        args.get(i)
            .map(|a| crate::plain::canonical(a))
            .filter(|a| !a.is_empty() && a.chars().all(|c| c.is_ascii_lowercase() || c == '-'))
    };
    let comando = match (word(0), word(1)) {
        (Some(c @ ("flow" | "llm")), Some(sub)) => format!("{c} {sub} "),
        (Some(c), _) if c != "-h" && c != "--help" && !c.starts_with('-') => format!("{c} "),
        _ => String::new(),
    };
    o.stderr += &format!("(mzlab-cli {comando}--help)\n");
    o
}

#[path = "stdout.rs"]
mod stdout;
pub use stdout::write_stdout;

#[cfg(test)]
#[path = "error_text_tests.rs"]
mod tests;
