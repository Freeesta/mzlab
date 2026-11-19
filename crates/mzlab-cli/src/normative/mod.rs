//! `mzlab-cli db normative build`: the builder of the regulatory lists (EU and US) from the official sources. Readers turn each source into entries, `resolve`
//! turns CAS numbers into structures, and the result is one A1+ CSV per list plus `provenance.json`. Native only; nothing here runs at analysis time.
//! Area file: `docs/agenti/normative.md`.

pub use mzlab_feat::normative::{csvrd, entry};

pub mod build;
#[cfg(test)]
mod build_json_tests;
pub mod build_out;
pub mod build_rows;
pub mod fetch;
pub mod import_official;
#[cfg(test)]
mod import_official_tests;
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
    fn new(key: &'static str, msg: String) -> NormError {
        NormError { key, msg }
    }
    pub fn usage(m: &str) -> Self {
        Self::new("err.normative.usage", m.into())
    }
    pub fn sources(m: &str) -> Self {
        Self::new("err.normative.sources", m.into())
    }
    pub fn pin(m: &str) -> Self {
        Self::new(
            "err.normative.pin",
            format!("{m} (run with --update-pins after checking the change)"),
        )
    }
    pub fn large(url: &str, limit: u64) -> Self {
        Self::new(
            "err.normative.large",
            format!(
                "{url} is above {} MB (use --allow-large)",
                limit / 1_000_000
            ),
        )
    }
    pub fn fetch(url: &str, m: &str) -> Self {
        Self::new("err.normative.fetch", format!("{url}: {m}"))
    }
    pub fn parse(list: &str, m: &str) -> Self {
        Self::new("err.normative.parse", format!("{list}: {m}"))
    }
    /// A file that cannot be written.
    pub fn io(p: &Path, e: &std::io::Error) -> Self {
        Self::new("err.normative.io", format!("{}: {e}", p.display()))
    }
    /// A file that cannot be read.
    pub fn input(p: &Path, e: &std::io::Error) -> Self {
        Self::new("err.normative.input", format!("{}: {e}", p.display()))
    }
    /// The failure as the dispatcher reports it: the text of its key, `message` as parameter, the code of the table.
    fn outcome(&self) -> Outcome {
        Outcome::fail_error(&mzlab_core::Error::new(self.key).with("message", &self.msg))
    }
}

pub const HELP: &str = "mzlab-cli db normative: the regulatory lists built from the official sources.

usage: mzlab-cli db normative pack IN OUT       the data release of the committed lists: OUT/normative.mzsus, zlib pieces and catalogo.json (no network)
       mzlab-cli db normative listings PATH     the listings JSON of a package folder or a .mzsus file
       mzlab-cli db normative import-official READER CSV SIDECAR OUT [--file NAME] [--date YYYY-MM-DD]
                                             your own copy of the ECHA Candidate List (svhc) or Proposition 65 (ca_prop65) as a private .mzsus (no network)
       mzlab-cli db normative build --cache DIR --out DIR [--only ID...] [--update-pins] [--no-resolve] [--allow-large]
                                 [--sources FILE] [--overrides FILE] [--json]

  --cache DIR     where downloads are kept (one file per URL with its SHA-256 and date); a warm cache makes the build offline and byte-identical
  --out DIR       writes lists/ID.csv, provenance.json and unresolved.tsv there
  --only ID...    build these lists only (the others keep their files)
  --update-pins   write the SHA-256 of what was read into sources.json (without it a changed source stops the build)
  --no-resolve    read the entries and count them, no structures (a check of the readers)
  --allow-large   accept downloads above 90 MB (PubChem and SusDat files)
  --sources FILE  default OUT/sources.json
  --overrides FILE  default OUT/overrides.tsv (list_id;entry_ref;cas;inchikey;reason)
  --json          stdout is the normative-build/1 document (lists, sources with pins, stale); without it stdout is empty and the lines go to stderr

exit: 0 done; 1 a list kept its last good file (stale); 2 usage; 3 sources.json not valid or pin mismatch; 4 an input file cannot be read; 6 a download was refused, a source failed or a file cannot be written (the table is in docs/agenti/cli-agenti.md)";

/// An error of the builder with its `err.normative.*` key.
#[derive(Debug, Clone)]
pub struct NormError {
    pub key: &'static str,
    pub msg: String,
}

struct Args {
    cache: PathBuf,
    opts: build::Opts,
    large: bool,
    json: bool,
}

fn parse(args: Vec<String>) -> Result<Option<Args>, NormError> {
    let mut it = args.into_iter().peekable();
    match it.next().as_deref() {
        Some("build") => {}
        None | Some("--help" | "-h") => return Ok(None),
        Some(other) => {
            return Err(NormError::usage(&format!(
                "unknown normative operation {other}"
            )))
        }
    }
    let (mut cache, mut out, mut sources, mut overrides) = (None, None, None, None);
    let (mut only, mut update, mut resolve, mut large) = (Vec::new(), false, true, false);
    let mut json = false;
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
            "--json" => json = true,
            other => return Err(NormError::usage(&format!("unknown argument {other}"))),
        }
    }
    let out = out.ok_or_else(|| NormError::usage("--out is needed"))?;
    let cache = cache.ok_or_else(|| NormError::usage("--cache is needed"))?;
    Ok(Some(Args {
        cache,
        large,
        json,
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

/// The `normative-build/1` document of a run: coverage per list, the sources read with their pins, the stale lists and why.
fn report_json(rep: &build::Report) -> String {
    let mut o = mzlab_core::json::Out::default();
    o.raw("{\"schema\":\"normative-build/1\",\"stale_lists\":")
        .int(rep.stale as i64);
    o.raw(",\"pins_updated\":")
        .raw(if rep.pins_updated { "true" } else { "false" });
    o.raw(",\"lists\":[");
    for (i, (id, c)) in rep.lists.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"id\":").str(id);
        for (k, n) in [
            "entries",
            "screenable",
            "groups",
            "uvcb",
            "inorganic",
            "unresolved",
        ]
        .iter()
        .zip(c)
        {
            o.raw(",").str(k).raw(":").int(*n as i64);
        }
        o.raw("}");
    }
    o.raw("],\"stale\":[");
    for (i, f) in rep.failed.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .str(&f.id);
        o.raw(",\"why\":").str(&f.why).raw("}");
    }
    o.raw("],\"sources\":[");
    for (i, s) in rep.sources.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"url\":")
            .str(&s.url);
        o.raw(",\"sha256\":").str(&s.sha256).raw(",\"pin\":");
        match &s.pin {
            Some(p) => {
                o.str(p)
                    .raw(",\"pin_ok\":")
                    .raw(if *p == s.sha256 { "true" } else { "false" })
            }
            None => o.raw("null,\"pin_ok\":null"),
        };
        o.raw(",\"pin_updated\":")
            .raw(if s.pin_updated { "true" } else { "false" })
            .raw("}");
    }
    o.raw("]}\n");
    o.0
}

/// The result of `build`: the per-list lines are progress (stderr); stdout is the `normative-build/1` document under `--json` and nothing otherwise.
/// Exit 1 when a list kept its last good file (stale).
fn finish(r: Result<build::Report, NormError>, json: bool) -> Outcome {
    match r {
        Ok(rep) => {
            let mut text: String = rep.lines.iter().map(|l| format!("{l}\n")).collect();
            if rep.stale > 0 {
                text += &format!("{} list(s) kept their last good file (stale)\n", rep.stale);
            }
            Outcome {
                code: u8::from(rep.stale > 0),
                stdout: if json {
                    report_json(&rep).into_bytes()
                } else {
                    Vec::new()
                },
                stderr: text,
                error: None,
            }
        }
        Err(e) => e.outcome(),
    }
}

/// `mzlab-cli db normative ...` with the real network.
pub fn run(args: Vec<String>) -> Outcome {
    run_with(args, fetch::Http::default())
}

/// `pack` and `listings`: no network, no cache.
fn run_package(args: &[String]) -> Option<Outcome> {
    let rest = args.get(1..).unwrap_or_default();
    // `import-official` has options of its own; for `pack` and `listings` any option is unknown
    if let (Some("pack" | "listings"), Err(e)) = (
        args.first().map(String::as_str),
        crate::argv::no_options(rest),
    ) {
        return Some(Outcome::fail_error(&e));
    }
    let paths: Vec<&Path> = rest
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(Path::new)
        .collect();
    let r = match (args.first().map(String::as_str), paths.as_slice()) {
        // the pieces written are progress, not data: stdout stays empty
        (Some("pack"), [i, o]) => pack::pack(i, o).map(|l| (String::new(), l.join("\n") + "\n")),
        (Some("listings"), [p]) => pack::listings(p).map(|t| (t, String::new())),
        (Some("import-official"), _) => import_official::run(rest).map(|t| (t, String::new())),
        (Some("pack" | "listings"), _) => Err(NormError::usage("wrong number of paths")),
        _ => return None,
    };
    Some(match r {
        Ok((t, note)) => Outcome {
            stderr: note,
            ..Outcome::ok(t.into_bytes())
        },
        Err(e) => e.outcome(),
    })
}

/// The same with any `Fetch` (the tests give a fake one).
pub fn run_with<F: fetch::Fetch>(args: Vec<String>, net: F) -> Outcome {
    if let Some(o) = run_package(&args) {
        return o;
    }
    match parse(args) {
        Ok(None) => Outcome::ok(format!("{HELP}\n").into_bytes()),
        Ok(Some(a)) => {
            let limit = if a.large { u64::MAX } else { fetch::LARGE };
            let cache = fetch::Cache::new(&a.cache, net, fetch::today, limit);
            finish(build::run(&cache, &a.opts), a.json)
        }
        Err(e) => e.outcome(),
    }
}
