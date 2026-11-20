//! `mzlab-cli file pair FILE... [--json]`: where the companion of a SCIEX `.wiff` or `.wiff.scan` stands, by the rule the site and the app follow
//! (`mzlab_lettori::pair`). Only names are listed in the folder of each file, no file is read; `convert`, `channels` and `flow run` use the same rule
//! to take a `.wiff` given alone together with its `.wiff.scan`.

use std::path::PathBuf;

use mzlab_core::json::Out;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli file pair: find the other half of a SCIEX pair (.wiff + .wiff.scan, same base name, any case) in the folder of the file given.

usage: mzlab-cli file pair FILE... [--json]

  FILE   a .wiff or a .wiff.scan; the file beside it is looked for by name (no file is opened or read)
  --json one wiffpair/1 object on the standard output: {schema, ok, pairs: [{input, half, wanted, found, size, looked_in}]}; found is null when missing
exit: 0 every companion found; 1 one is missing (the others are still listed, the document stays on stdout); 2 usage (no file, not a .wiff / .wiff.scan, one error/1 under --json)";

fn parse(args: Vec<String>) -> Result<Option<(Vec<PathBuf>, bool)>, String> {
    let (mut files, mut json) = (Vec::new(), false);
    for a in args {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--json" => json = true,
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ => files.push(PathBuf::from(a)),
        }
    }
    if files.is_empty() {
        return Err("pair needs a .wiff or a .wiff.scan".into());
    }
    Ok(Some((files, json)))
}

pub fn run(args: Vec<String>) -> Outcome {
    let (files, json) = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let mut rows = Vec::new();
    for f in &files {
        match mzlab_lettori::pair::companion_in_dir(f) {
            Some(c) => rows.push((f.clone(), c)),
            None => return Outcome::fail(&format!("{}: not a .wiff or a .wiff.scan", f.display())),
        }
    }
    let ok = rows.iter().all(|(_, c)| c.found.is_some());
    let mut o = Out(String::new());
    let mut human = String::new();
    o.raw("{\"schema\":\"wiffpair/1\",\"ok\":")
        .raw(if ok { "true" } else { "false" })
        .raw(",\"pairs\":[");
    for (i, (f, c)) in rows.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        let dir = f
            .parent()
            .filter(|d| !d.as_os_str().is_empty())
            .map_or(".".to_string(), |d| d.display().to_string());
        let size = c
            .found
            .as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| m.len());
        o.raw("{\"input\":")
            .str(&f.display().to_string())
            .raw(",\"half\":")
            .str(c.half.key())
            .raw(",\"wanted\":")
            .str(&c.wanted)
            .raw(",\"found\":");
        match &c.found {
            Some(p) => o.str(&p.display().to_string()),
            None => o.raw("null"),
        };
        o.raw(",\"size\":");
        match size {
            Some(n) => o.int(n as i64),
            None => o.raw("null"),
        };
        o.raw(",\"looked_in\":").str(&dir).raw("}");
        human.push_str(&match &c.found {
            Some(p) => format!("{}\tfound\t{}\n", f.display(), p.display()),
            None => format!("{}\tmissing\t{} (looked in {dir})\n", f.display(), c.wanted),
        });
    }
    o.raw("]}");
    let (text, code) = (if json { o.0 + "\n" } else { human }, u8::from(!ok));
    Outcome {
        code,
        stdout: text.into_bytes(),
        // the summary goes to stderr in both forms, the data to stdout
        stderr: if ok {
            String::new()
        } else {
            "pair: a companion is missing\n".into()
        },
        error: None,
    }
}

#[cfg(test)]
#[path = "pair_tests.rs"]
mod tests;
