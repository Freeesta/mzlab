//! The English text of an engine error for the terminal: the catalogue the page shows (`mzlab/web/lang/en/errori.js`) with its `{param}` filled in,
//! and the key kept in brackets so the message can still be searched or matched by scripts. A message that does not start with a known key passes
//! unchanged (usage errors, file-system errors).

const CATALOGO: &str = include_str!("../../../mzlab/web/lang/en/errori.js");

/// The catalogue text of `key` (the catalogue is one `"key": "text",` per line, without escapes).
fn testo_di(key: &str) -> Option<&'static str> {
    let segno = format!("\n  \"{key}\": \"");
    let da = CATALOGO.find(&segno)? + segno.len();
    let riga = CATALOGO[da..].lines().next()?.trim_end();
    let riga = riga.strip_suffix(',').unwrap_or(riga);
    riga.strip_suffix('"')
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
    let Some(testo) = testo_di(key) else {
        return msg.to_string();
    };
    let mut t = testo.to_string();
    let mut avanzo = String::new();
    for (k, v) in parametri(resto) {
        let segno = format!("{{{k}}}");
        if t.contains(&segno) {
            t = t.replace(&segno, &v);
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

#[cfg(test)]
#[path = "testo_errore_tests.rs"]
mod tests;
