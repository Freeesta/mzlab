//! `mzlab-cli campioni variabili|nome-gruppo`: the two name operations of `campioni::route` (`variabili`, `nome_gruppo`), printing the JSON the page gets
//! from `api/campioni/<op>`. The names come one per line from a file or the standard input.

use std::io::Read;

use mzlab_core::json::Out;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli campioni: how the names of a set of files split them (nomivar/1), as JSON.

usage: mzlab-cli campioni variabili [--names FILE|-] [--pos N [--sep CHARS]] [--has a,b [--substring]]
       mzlab-cli campioni nome-gruppo NAME...

  --names FILE|-  one file name per line (default: the standard input)
  --pos N         also the manual variable of the N-th token (1 = first, -1 = last), cut at --sep (default \"_-. \")
  --has a,b       also the manual variable of terms: a file goes to the first term equal to one of its tokens
  --substring     with --has: the term may be inside a token (O2 then catches H2O2)
  nome-gruppo     the name of a group made of NAMEs: their common tokens in order, empty when none";

fn names(src: &str) -> Result<Vec<String>, String> {
    let mut text = String::new();
    if src == "-" {
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
    } else {
        text = std::fs::read_to_string(src).map_err(|e| format!("{src}: {e}"))?;
    }
    Ok(text
        .lines()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

fn list(names: &[String]) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, n) in names.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(n);
    }
    o.raw("]");
    o.0
}

/// The body of `variabili`: the names and, when asked, the manual variable.
fn variabili(args: Vec<String>) -> Result<String, String> {
    let (mut src, mut pos, mut sep, mut has, mut substring) =
        ("-".to_string(), None, None, None, false);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--names" => src = it.next().ok_or("--names needs a file")?,
            "--pos" => {
                let p = it.next().ok_or("--pos needs a number")?;
                pos = Some(p.parse::<i64>().map_err(|_| "--pos needs a number")?);
            }
            "--sep" => sep = Some(it.next().ok_or("--sep needs characters")?),
            "--has" => has = Some(it.next().ok_or("--has needs terms")?),
            "--substring" => substring = true,
            o => return Err(format!("unknown option {o}")),
        }
    }
    if pos.is_some() && has.is_some() {
        return Err("--pos and --has are two manual variables: one at a time".into());
    }
    if (sep.is_some() && pos.is_none()) || (substring && has.is_none()) {
        return Err("--sep needs --pos, --substring needs --has".into());
    }
    let files: Vec<String> = names(&src)?
        .iter()
        .map(|n| {
            let mut o = Out::default();
            o.raw("{\"name\":").str(n).raw("}");
            o.0
        })
        .collect();
    let mut body = format!("{{\"files\":[{}]", files.join(","));
    if let Some(p) = pos {
        let mut o = Out::default();
        o.str(sep.as_deref().unwrap_or("_-. "));
        body += &format!(",\"manuale\":{{\"pos\":{p},\"sep\":{}}}", o.0);
    }
    if let Some(h) = has {
        let terms: Vec<String> = h.split(',').map(|t| t.trim().to_string()).collect();
        body += &format!(
            ",\"manuale\":{{\"has\":{},\"substring\":{substring}}}",
            list(&terms)
        );
    }
    body.push('}');
    Ok(body)
}

pub fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    let (op, body) = match it.next().as_deref() {
        Some("variabili") => ("variabili", variabili(it.collect())),
        Some("nome-gruppo") => {
            let n: Vec<String> = it.collect();
            ("nome_gruppo", Ok(format!("{{\"names\":{}}}", list(&n))))
        }
        Some("--help" | "-h") | None => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Some(c) => return Outcome::fail(&format!("unknown campioni operation {c}")),
    };
    match body.and_then(|b| {
        mzlab_core::campioni::route_tutto(op, &b).map_err(|e| format!("{} {:?}", e.key, e.params))
    }) {
        Ok(t) => Outcome::ok((t + "\n").into_bytes()),
        Err(e) => Outcome::fail(&e),
    }
}

#[cfg(test)]
#[path = "campioni_cmd_tests.rs"]
mod tests;
