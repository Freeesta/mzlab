//! `mzlab-cli raw cache FILE...` (`raw-cache/1`, `docs/studies/RAW_ACCESS_STUDY_2.md` §2): builds or reuses the `.mzidx` entry of each file in the
//! index cache (the entry `file index` writes) and says what it holds (scans, sections, ion quantum); `--check` builds nothing and says which
//! entries a next call would reuse (exit 1 when one is not). Without `--cache DIR` or `MZLAB_CACHE` nothing is written: exit 2.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Out;
use mzlab_core::routes::Routes;

use crate::extract::{self, jstr};
use crate::raw_cmd::{
    error_entry, missing, name_of, prov_file, provenance, warnings_of, whole_run,
};
use crate::Outcome;

pub const CACHE_HELP: &str = "mzlab-cli raw cache: build or reuse the index of each file in the cache, as JSON (raw-cache/1: per file `status` built,
reused, stale or missing, the cache `key`, `bytes`, `n_scans`, `sections` and `ion_quantum`), so `raw` and the other file commands start from it.

usage: mzlab-cli raw cache FILE... [--sample ID] [--check] [--out PATH]

  FILE        an mzML, or any file the app reads; the entry is the one `file index` writes (SHA-256 of the file, sample, index format, version)
  --sample ID one sample of a file with several (.wiff); default the first; an id the file does not have: exit 5
  --check     build nothing: `reused` is a sound entry the next call reuses, `stale` one it would rebuild, `missing` none; exit 1 unless all reused
  --out PATH  write there instead of the standard output
the cache is the global flag `--cache DIR` or the variable MZLAB_CACHE (one is required, exit 2 without);
exit 0 answered, 1 a file could not be read (its entry has `error`) or under --check an entry is not reused (the document stays on stdout), 2 usage, 4 a file is missing, 5 unknown sample, 6 the cache cannot be written";

/// The `sections`, `n_scans` and `ion_quantum` members of an entry, read from it.
fn held(entry: &Path) -> Result<String> {
    let mut r = Routes::new(crate::open(entry)?);
    let mut o = Out::default();
    o.raw("\"n_scans\":")
        .int(r.idx.scans.len() as i64)
        .raw(",\"sections\":[");
    for (i, (key, si)) in r.idx.keys.iter().enumerate() {
        let (level, pol) = key.split_once('/').unwrap_or((key, ""));
        let s = &r.idx.sections[*si];
        o.raw(if i > 0 { "," } else { "" });
        o.raw("{\"level\":")
            .raw(level)
            .raw(",\"polarity\":")
            .str(pol);
        o.raw(",\"n_scans\":")
            .int(s.n_scans as i64)
            .raw(",\"n_peaks\":")
            .int(s.n_peaks as i64)
            .raw("}");
    }
    let level = r
        .idx
        .scans
        .iter()
        .map(|s| s.level)
        .filter(|l| *l >= 1)
        .min();
    let p = match level {
        Some(l) => Some(r.probe(l, mzlab_core::rawinfo::PROBE)?),
        None => None,
    };
    o.raw("],\"ion_quantum\":")
        .raw(&mzlab_core::rawinfo::quantum_json(p.as_ref()));
    Ok(o.0)
}

/// The entry of one file and its `provenance.files` member.
fn one(
    dir: &Path,
    f: &Path,
    sample: Option<u32>,
    check: bool,
    warnings: &mut Vec<String>,
) -> Result<(String, String)> {
    let name = name_of(f);
    let label = sample.map_or(String::new(), |id| format!("{name}@{id}"));
    let (status, key, sha) = if check {
        crate::idx_cache::peek(dir, f, &label)?
    } else {
        let e = extract::with_tmp("rawcache", |tmp| {
            let idx = tmp.join("0.mzidx");
            crate::idx_cache::make(dir, f, &label, &idx, true, || {
                crate::flow_pace::build_index_with(
                    f,
                    &label,
                    &idx,
                    mzlab_core::budget_auto::native_index_budget(),
                    &mut |_, _| {},
                    &None,
                )
                .map(|_| ())
            })
        })?;
        (if e.reused { "reused" } else { "built" }, e.key, e.sha256)
    };
    let entry = dir.join(&key);
    let mut o = Out::default();
    o.raw("{\"file\":")
        .str(&name)
        .raw(",\"sample\":")
        .raw(&sample.map_or("null".into(), |s| s.to_string()));
    o.raw(",\"status\":")
        .str(status)
        .raw(",\"key\":")
        .str(&key)
        .raw(",\"sha256\":")
        .str(&sha);
    if status == "built" || status == "reused" {
        let bytes = std::fs::metadata(&entry)
            .map_err(|x| Error::new("err.run.output").with("detail", x))?
            .len();
        let members = held(&entry)?;
        // the quantum of the entry carries the same warning as `raw info`
        warnings_of(&name, sample, &members, warnings);
        o.raw(",\"bytes\":")
            .int(bytes as i64)
            .raw(",")
            .raw(&members)
            .raw("}");
    } else {
        o.raw(",\"bytes\":null,\"n_scans\":null,\"sections\":null,\"ion_quantum\":null}");
    }
    let size = std::fs::metadata(f).ok().map(|m| m.len());
    Ok((o.0, prov_file(&name, &sha, size, sample)))
}

pub fn cache_run(args: Vec<String>) -> Outcome {
    let command: Vec<String> = ["mzlab-cli", "raw", "cache"]
        .iter()
        .map(|s| s.to_string())
        .chain(args.iter().cloned())
        .collect();
    let p = match extract::parse(args, &["--sample", "--out"], &["--check"]) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{CACHE_HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let files: Vec<PathBuf> = p.pos.iter().map(PathBuf::from).collect();
    if files.is_empty() {
        return Outcome::fail("raw cache needs a file");
    }
    let Some(dir) = crate::idx_cache::dir() else {
        return Outcome::fail("raw cache needs --cache DIR or MZLAB_CACHE")
            .with_hint("add --cache DIR after the command name, or set MZLAB_CACHE");
    };
    if let Some(e) = missing(&files) {
        return crate::sample_select::fail(&e);
    }
    let check = p.flag("--check");
    let (mut entries, mut provs, mut warnings, mut bad) =
        (Vec::new(), Vec::new(), Vec::new(), false);
    for f in &files {
        let sample = match p.sample(f) {
            Ok(s) => s,
            Err(m) => return Outcome::fail(&m),
        };
        match one(&dir, f, sample, check, &mut warnings) {
            Ok((e, pv)) => {
                // under --check, an entry the next call would build is a verdict like `exit 1 if changed`
                bad |= check && !e.contains("\"status\":\"reused\"");
                entries.push(e);
                provs.push(pv);
            }
            Err(e) if whole_run(&e) || crate::exit::code_of(&e) == crate::exit::OUTPUT => {
                return crate::sample_select::fail(&e)
            }
            Err(e) => {
                bad = true;
                entries.push(error_entry(f, &e, "raw cache"));
            }
        }
    }
    let mut params = Out::default();
    params
        .raw("{\"check\":")
        .raw(if check { "true" } else { "false" });
    params
        .raw(",\"sample\":")
        .raw(&p.get("--sample").map_or("null".into(), jstr))
        .raw("}");
    let text = format!(
        "{{\"schema\":\"raw-cache/1\",\"cache\":{},\"check\":{check},\"files\":[{}],\"warnings\":[{}],{}}}\n",
        jstr(&dir.display().to_string()),
        entries.join(","),
        warnings.join(","),
        provenance(&command, &provs, &params.0, &[])
    );
    let mut o = crate::emit(text, &p.out());
    if o.code == 0 && bad {
        o.code = crate::exit::DATA;
    }
    o
}
