//! `mzlab-cli normative build`: the builder of the regulatory lists (EU and US) from the official sources. Readers turn each source into entries, `resolve`
//! turns CAS numbers into structures, and the result is one A1+ CSV per list plus `provenance.json`. Native only; nothing here runs at analysis time.
//! Area file: `docs/agenti/normative.md`.

pub mod build;
pub mod build_out;
pub mod build_rows;
pub mod csvrd;
pub mod entry;
pub mod fetch;
pub mod markup;
pub mod pack;
#[cfg(test)]
mod pack_tests;
pub mod readers;
pub mod resolve;
pub mod resolve_src;
pub mod sources;
pub mod table_rd;
#[cfg(test)]
mod tests;

use std::fmt;
use std::path::{Path, PathBuf};

use crate::Outcome;

impl fmt::Display for NormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.msg)
    }
}

impl NormError {
    fn new(key: &'static str, code: u8, msg: String) -> NormError {
        NormError { key, msg, code }
    }
    pub fn usage(m: &str) -> Self {
        Self::new("err.normative.usage", 2, m.into())
    }
    pub fn sources(m: &str) -> Self {
        Self::new("err.normative.sources", 2, m.into())
    }
    pub fn pin(m: &str) -> Self {
        Self::new(
            "err.normative.pin",
            3,
            format!("{m} (run with --update-pins after checking the change)"),
        )
    }
    pub fn large(url: &str, limit: u64) -> Self {
        Self::new(
            "err.normative.large",
            4,
            format!(
                "{url} is above {} MB (use --allow-large)",
                limit / 1_000_000
            ),
        )
    }
    pub fn fetch(url: &str, m: &str) -> Self {
        Self::new("err.normative.fetch", 5, format!("{url}: {m}"))
    }
    pub fn parse(list: &str, m: &str) -> Self {
        Self::new("err.normative.parse", 5, format!("{list}: {m}"))
    }
    pub fn io(p: &Path, e: &std::io::Error) -> Self {
        Self::new("err.normative.io", 6, format!("{}: {e}", p.display()))
    }
}

pub const HELP: &str = "mzlab-cli normative: the regulatory lists built from the official sources.

usage: mzlab-cli normative pack IN OUT       the data release of the committed lists: OUT/normative.mzsus, zlib pieces and catalogo.json (no network)
       mzlab-cli normative listings PATH     the listings JSON of a package folder or a .mzsus file
       mzlab-cli normative build --cache DIR --out DIR [--only ID...] [--update-pins] [--no-resolve] [--allow-large]
                                 [--sources FILE] [--overrides FILE]

  --cache DIR     where downloads are kept (one file per URL with its SHA-256 and date); a warm cache makes the build offline and byte-identical
  --out DIR       writes lists/ID.csv, provenance.json and unresolved.tsv there
  --only ID...    build these lists only (the others keep their files)
  --update-pins   write the SHA-256 of what was read into sources.json (without it a changed source stops the build)
  --no-resolve    read the entries and count them, no structures (a check of the readers)
  --allow-large   accept downloads above 90 MB (PubChem and SusDat files)
  --sources FILE  default OUT/sources.json
  --overrides FILE  default OUT/overrides.tsv (list_id;entry_ref;cas;inchikey;reason)

exit: 0 done; 1 a list kept its last good file (stale); 2 usage or sources.json; 3 pin mismatch; 4 download too large; 5 a resolver source failed; 6 file error";

/// An error of the builder with its `err.normative.*` key.
#[derive(Debug, Clone)]
pub struct NormError {
    pub key: &'static str,
    pub msg: String,
    pub code: u8,
}

struct Args {
    cache: PathBuf,
    opts: build::Opts,
    large: bool,
}

fn parse(args: Vec<String>) -> Result<Option<Args>, NormError> {
    let mut it = args.into_iter().peekable();
    if it.next().as_deref() != Some("build") {
        return Ok(None);
    }
    let (mut cache, mut out, mut sources, mut overrides) = (None, None, None, None);
    let (mut only, mut update, mut resolve, mut large) = (Vec::new(), false, true, false);
    while let Some(a) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .ok_or_else(|| NormError::usage(&format!("{name} needs a value")))
        };
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--cache" => cache = Some(PathBuf::from(value("--cache")?)),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--sources" => sources = Some(PathBuf::from(value("--sources")?)),
            "--overrides" => overrides = Some(PathBuf::from(value("--overrides")?)),
            "--only" => {
                only.push(value("--only")?);
                while let Some(n) = it.next_if(|n| !n.starts_with("--")) {
                    only.push(n);
                }
            }
            "--update-pins" => update = true,
            "--no-resolve" => resolve = false,
            "--allow-large" => large = true,
            other => return Err(NormError::usage(&format!("unknown argument {other}"))),
        }
    }
    let out = out.ok_or_else(|| NormError::usage("--out is needed"))?;
    let cache = cache.ok_or_else(|| NormError::usage("--cache is needed"))?;
    Ok(Some(Args {
        cache,
        large,
        opts: build::Opts {
            sources: sources.unwrap_or_else(|| out.join("sources.json")),
            overrides: overrides.unwrap_or_else(|| out.join("overrides.tsv")),
            out,
            only,
            update_pins: update,
            resolve,
        },
    }))
}

fn finish(r: Result<build::Report, NormError>) -> Outcome {
    match r {
        Ok(rep) => Outcome {
            code: u8::from(rep.stale > 0),
            stdout: rep
                .lines
                .iter()
                .map(|l| format!("{l}\n"))
                .collect::<String>()
                .into_bytes(),
            stderr: if rep.stale > 0 {
                format!("{} list(s) kept their last good file (stale)\n", rep.stale)
            } else {
                String::new()
            },
        },
        Err(e) => Outcome {
            code: e.code,
            stdout: Vec::new(),
            stderr: format!("error: {e}\n"),
        },
    }
}

/// `mzlab-cli normative ...` with the real network.
pub fn run(args: Vec<String>) -> Outcome {
    run_with(args, fetch::Http::default())
}

/// `pack` and `listings`: no network, no cache.
fn run_package(args: &[String]) -> Option<Outcome> {
    let paths: Vec<&Path> = args[1..]
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(Path::new)
        .collect();
    let r = match (args.first().map(String::as_str), paths.as_slice()) {
        (Some("pack"), [i, o]) => pack::pack(i, o).map(|l| l.join("\n") + "\n"),
        (Some("listings"), [p]) => pack::listings(p),
        (Some("pack" | "listings"), _) => Err(NormError::usage("wrong number of paths")),
        _ => return None,
    };
    Some(match r {
        Ok(t) => Outcome {
            code: 0,
            stdout: t.into_bytes(),
            stderr: String::new(),
        },
        Err(e) => Outcome {
            code: e.code,
            stdout: Vec::new(),
            stderr: format!("error: {e}\n{HELP}\n"),
        },
    })
}

/// The same with any `Fetch` (the tests give a fake one).
pub fn run_with<F: fetch::Fetch>(args: Vec<String>, net: F) -> Outcome {
    if let Some(o) = run_package(&args) {
        return o;
    }
    match parse(args) {
        Ok(None) => Outcome {
            code: 0,
            stdout: format!("{HELP}\n").into_bytes(),
            stderr: String::new(),
        },
        Ok(Some(a)) => {
            let limit = if a.large { u64::MAX } else { fetch::LARGE };
            let cache = fetch::Cache::new(&a.cache, net, fetch::today, limit);
            finish(build::run(&cache, &a.opts))
        }
        Err(e) => Outcome {
            code: e.code,
            stdout: Vec::new(),
            stderr: format!("error: {e}\n{HELP}\n"),
        },
    }
}
