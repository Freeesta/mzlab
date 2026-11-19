//! `mzlab-cli file index FILE... --cache DIR`: builds the `.mzidx` of each file into the index cache ahead of the questions (`index-build/1`), so the
//! `file`, `trace`, `spectrum`, ... commands that follow with the same `--cache` reuse it. A cache that cannot be written is an error here (exit 6).

use std::path::PathBuf;

use mzlab_core::error::{Error, Result};

use crate::extract::{self, jstr};
use crate::Outcome;

pub const HELP: &str = "mzlab-cli file index: build the index of each file into the cache, as JSON (index-build/1: per file `status` built or reused, the cache `key`, `sha256`, `index_bytes`).

usage: mzlab-cli file index FILE... [--sample ID] [--out PATH]

  FILE        an mzML, or any file the app reads; the index is kept under the SHA-256 of the file, the sample, the index format and the program version
  --sample ID one sample of a file with several (.wiff); default the first; an id the file does not have: exit 5
  --out PATH  write there instead of the standard output
the cache is the global flag `--cache DIR` or the variable MZLAB_CACHE (one of them is required); the other file commands reuse the entries with it,
and say `index reused` or `index built` on the standard error";

/// The `files[]` member of one file.
fn one(dir: &std::path::Path, file: &std::path::Path, sample: Option<u32>) -> Result<String> {
    let name = file
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let label = sample.map_or(String::new(), |id| format!("{name}@{id}"));
    let e = extract::with_tmp("index", |tmp| {
        let idx = tmp.join("0.mzidx");
        crate::idx_cache::make(dir, file, &label, &idx, true, || {
            crate::flow_pace::build_index_with(
                file,
                &label,
                &idx,
                mzlab_core::budget_auto::native_index_budget(),
                &mut |_, _| {},
                &None,
            )
            .map(|_| ())
        })
    })?;
    let bytes = std::fs::metadata(dir.join(&e.key))
        .map_err(|x| Error::new("err.run.output").with("detail", x))?
        .len();
    Ok(format!(
        "{{\"file\":{},\"sample\":{},\"status\":\"{}\",\"key\":{},\"sha256\":\"{}\",\"index_bytes\":{bytes}}}",
        jstr(&file.display().to_string()),
        sample.map_or("null".into(), |s| s.to_string()),
        if e.reused { "reused" } else { "built" },
        jstr(&e.key),
        e.sha256
    ))
}

pub fn run(args: Vec<String>) -> Outcome {
    let p = match extract::parse(args, &["--sample", "--out"], &[]) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let files: Vec<PathBuf> = p.pos.iter().map(PathBuf::from).collect();
    if files.is_empty() {
        return Outcome::fail("file index needs a file");
    }
    let Some(dir) = crate::idx_cache::dir() else {
        return Outcome::fail("file index needs --cache DIR or MZLAB_CACHE");
    };
    let r: Result<Vec<String>> = files
        .iter()
        .map(|f| {
            let sample = p
                .sample(f)
                .map_err(|m| Error::new("err.cli.usage").with("message", m))?;
            one(&dir, f, sample)
        })
        .collect();
    match r {
        Err(e) => crate::sample_select::fail(&e),
        Ok(docs) => {
            let text = format!(
                "{{\"schema\":\"index-build/1\",\"cache\":{},\"files\":[{}]}}\n",
                jstr(&dir.display().to_string()),
                docs.join(",")
            );
            crate::emit(text, &p.out())
        }
    }
}
