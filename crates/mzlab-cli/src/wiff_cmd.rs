//! `mzlab-cli file info|verify`: what a SCIEX `.wiff` + `.wiff.scan` pair holds (every sample with its section of the `.scan`, its streams, a status and SHA-256) and whether
//! an output pair equals the sources it was assembled from. Read only; the same engine code (`wiff::inventory`) as the site's `wiffwrite` module, so the JSON is byte-identical.
//! Exit codes: 0 done, 1 verify found a difference, 2 usage or read error, 3 list found a sample that is not `ok`.

use std::path::PathBuf;

use mzlab_core::readers::Limits;
use mzlab_core::wiff::inventory;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli file info: inspect and compare SCIEX .wiff + .wiff.scan pairs (read only), JSON on stdout for agents.

usage: mzlab-cli file info FILE.wiff... [--inventory] [--out PATH]
       mzlab-cli file verify OUT.wiff --against SRC.wiff... [--out PATH]

  list        wiff.list/1: per file the SHA-256, the container (version, sector sizes), the .scan size and the streams shared by all samples; per sample its number,
              name, method, scans, status (ok, refused, truncated, damaged, unindexed) with the reason, its section of the .scan (start, bytes, slack, SHA-256, first
              44 bytes) and every stream with bytes and SHA-256. A refused or damaged sample is listed, never hidden
  --inventory also the whole .wiff.scan SHA-256 and the directory metadata of every entry (CLSID, state bits, creation and modification times)
  verify      wiff.verify/1: each sample of OUT is matched to the source sample with the same section (SHA-256) and compared stream by stream and on the directory
              metadata; the streams shared by all samples must equal those of the first source; `equal` is the answer, `differences` the evidence
  --json      accepted (the output is always JSON)
  --out PATH  write there instead of the standard output
exit codes: 0 done (verify: everything equal), 1 verify found a difference, 2 usage or read error, 3 list found a sample that is not ok";

struct Req {
    verb: String,
    files: Vec<PathBuf>,
    against: Vec<PathBuf>,
    inventory: bool,
    out: Option<PathBuf>,
}

fn parse(args: Vec<String>) -> Result<Option<Req>, String> {
    let mut it = args.into_iter();
    let verb = it.next().ok_or("wiff needs a verb: list or verify")?;
    if verb == "--help" || verb == "-h" {
        return Ok(None);
    }
    let mut r = Req {
        verb,
        files: vec![],
        against: vec![],
        inventory: false,
        out: None,
    };
    let mut after_against = false;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--json" => {}
            "--inventory" => r.inventory = true,
            "--out" => r.out = Some(PathBuf::from(it.next().ok_or("--out needs a value")?)),
            "--against" => {
                after_against = true;
                r.against
                    .push(PathBuf::from(it.next().ok_or("--against needs a file")?));
            }
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ if after_against => r.against.push(PathBuf::from(a)),
            _ => r.files.push(PathBuf::from(a)),
        }
    }
    match r.verb.as_str() {
        "list" if r.files.is_empty() => return Err("wiff list needs at least one .wiff".into()),
        "list" if !r.against.is_empty() => return Err("--against is for wiff verify".into()),
        "verify" if r.files.len() != 1 || r.against.is_empty() => {
            return Err("wiff verify takes OUT.wiff --against SRC.wiff...".into())
        }
        "verify" if r.inventory => return Err("--inventory is for wiff list".into()),
        "list" | "verify" => {}
        v => return Err(format!("unknown wiff verb {v}")),
    }
    Ok(Some(r))
}

fn run_req(r: &Req) -> mzlab_core::error::Result<(String, u8)> {
    let sets = |v: &[PathBuf]| {
        v.iter()
            .map(|p| crate::flow_set::file_set(p))
            .collect::<mzlab_core::error::Result<Vec<_>>>()
    };
    let limits = Limits::desktop();
    if r.verb == "list" {
        let (t, bad) = inventory::list(sets(&r.files)?, &limits, r.inventory)?;
        return Ok((t, if bad { 3 } else { 0 }));
    }
    let out = crate::flow_set::file_set(&r.files[0])?;
    let (t, equal) = inventory::verify(out, sets(&r.against)?, &limits)?;
    Ok((t, if equal { 0 } else { 1 }))
}

pub fn run(args: Vec<String>) -> Outcome {
    let r = match parse(args) {
        Ok(Some(r)) => r,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    match run_req(&r) {
        Ok((t, code)) => {
            let mut o = crate::emit(t + "\n", &r.out);
            if o.code == 0 {
                o.code = code;
            }
            o
        }
        Err(e) => Outcome::fail_error(&e),
    }
}

#[cfg(test)]
#[path = "wiff_tests.rs"]
mod tests;
