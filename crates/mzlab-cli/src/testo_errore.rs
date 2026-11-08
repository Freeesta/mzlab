//! The English text of an engine error for the terminal: the catalogue the page shows (`mzlab/web/lang/en/errori.js`) with its `{param}` filled in,
//! and the key kept in brackets so the message can still be searched or matched by scripts. A message that does not start with a known key passes
//! unchanged (usage errors, file-system errors).

const CATALOGO: &str = include_str!("../../../mzlab/web/lang/en/errori.js");

/// The people's names of the memory phases (`memory.phase.<id>`), the ones the page puts in the same sentence.
const FASI: &str = include_str!("../../../mzlab/web/lang/en/altre-lv-predef.js");

/// The text of `key` in a catalogue of one `"key": "text",` per line, without escapes.
fn testo_in(catalogo: &'static str, key: &str) -> Option<&'static str> {
    let segno = format!("\n  \"{key}\": \"");
    let da = catalogo.find(&segno)? + segno.len();
    let riga = catalogo[da..].lines().next()?.trim_end();
    let riga = riga.strip_suffix(',').unwrap_or(riga);
    riga.strip_suffix('"')
}

/// The catalogue text of `key`.
fn testo_di(key: &str) -> Option<&'static str> {
    testo_in(CATALOGO, key)
}

/// The key whose text the terminal shows, the same choice as `I18N.err` in the desktop app: a memory refusal that names its phase has the
/// sentence with the numbers (`error.memoryBudgetFase[.<fase>]`), and a key with a `.native` text uses it (the limit is this computer's memory).
fn chiave_resa(key: &str, fase: Option<&str>) -> String {
    let base = match fase {
        Some(f) if key == "error.memoryBudget" => {
            let propria = format!("error.memoryBudgetFase.{f}");
            if testo_di(&propria).is_some() {
                propria
            } else {
                "error.memoryBudgetFase".to_string()
            }
        }
        _ => key.to_string(),
    };
    let nativa = format!("{base}.native");
    if testo_di(&nativa).is_some() {
        nativa
    } else {
        base
    }
}

/// The parameters of `Error`'s `Display` form (` k=v k2=v2`): a word that starts with `name=` opens a parameter, the others continue its value.
fn parametri(resto: &str) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = Vec::new();
    for parola in resto.split(' ').filter(|p| !p.is_empty()) {
        let nome = parola
            .split_once('=')
            .map(|(k, _)| k)
            .filter(|k| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        match (nome, v.last_mut()) {
            (Some(k), _) => v.push((k.to_string(), parola[k.len() + 1..].to_string())),
            (None, Some((_, val))) => {
                val.push(' ');
                val.push_str(parola);
            }
            (None, None) => return Vec::new(),
        }
    }
    v
}

/// `msg` with its leading error key rendered: «text (key, unused=params)».
pub(crate) fn umano(msg: &str) -> String {
    let (key, resto) = msg.split_once(' ').unwrap_or((msg, ""));
    if !(key.starts_with("err.") || key.starts_with("error.")) {
        return msg.to_string();
    }
    rendi(msg, key, resto)
}

/// A calm notice (the one the desktop app gives its page as `{"key", "params"}`) as the terminal line: «text (key)».
pub fn avviso(n: &crate::convert::Notice) -> String {
    let resto: String = n.params.iter().map(|(k, v)| format!(" {k}={v}")).collect();
    rendi(n.key, n.key, &resto)
}

fn rendi(msg: &str, key: &str, resto: &str) -> String {
    let par = parametri(resto);
    let fase = par
        .iter()
        .find(|(k, _)| k == "fase")
        .map(|(_, v)| v.as_str());
    let Some(testo) = testo_di(&chiave_resa(key, fase)) else {
        return msg.to_string();
    };
    let mut t = testo.to_string();
    let mut avanzo = String::new();
    for (k, v) in par {
        let segno = format!("{{{k}}}");
        if t.contains(&segno) {
            let nome = (k == "fase")
                .then(|| testo_in(FASI, &format!("memory.phase.{v}")))
                .flatten();
            t = t.replace(&segno, nome.unwrap_or(&v));
        } else {
            avanzo += &format!(", {k}={v}");
        }
    }
    format!("{t} ({key}{avanzo})")
}

/// A usage or engine error (`Outcome::fail`, code 2) ends with the help of the command that was run: `(mzlab-cli channels --help)`, not another's.
pub(crate) fn con_aiuto(args: &[String], mut o: crate::Outcome) -> crate::Outcome {
    if o.code != 2 || !o.stderr.starts_with("error: ") || o.stderr.contains("--help)") {
        return o;
    }
    if o.stderr.starts_with("error: unknown command") {
        o.stderr += "(mzlab-cli --help)\n";
        return o;
    }
    let parola = |i: usize| {
        args.get(i)
            .map(|a| crate::plain::canonical(a))
            .filter(|a| !a.is_empty() && a.chars().all(|c| c.is_ascii_lowercase() || c == '-'))
    };
    let comando = match (parola(0), parola(1)) {
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
#[path = "testo_errore_tests.rs"]
mod tests;
