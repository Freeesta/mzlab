//! `mzlab-cli file items FILE [--out PATH]`: the items of a file as the page and the app list them (`Item.info()`, `lr::open_items`): counts, ranges,
//! polarity and `ms2_exps` (precursor, CE, and for a stepped CE its centre `ce_mid` and half width `ce_spread`). Stable keys, JSON only (units: `docs/schemi/items-1.json`).

use std::fs::File;
use std::path::PathBuf;

use mzlab_core::error::{Error, Result};
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::routes::Routes;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli file items: what the page learns about a file (one item, or one per part of a mixed file) as {\"schema\":\"items/1\",\"items\":[{info, hr}]}.

usage: mzlab-cli file items FILE [--out PATH]

  FILE        an mzML, or any single file the app reads (.raw, mzXML, ...); a file of several samples gives its first
  --out PATH  write there instead of the standard output

info.ms2_exps lists {prec (m/z), ce (eV, or % when info.nce), ce_mid, ce_spread, n}: ce_mid and ce_spread (35 and 15 for a stepped 20/35/50) are null with a single CE.";

fn parse(args: Vec<String>) -> std::result::Result<Option<(PathBuf, Option<PathBuf>)>, String> {
    let (mut file, mut out) = (None, None);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ if file.is_none() => file = Some(PathBuf::from(a)),
            _ => return Err("items takes one file".into()),
        }
    }
    Ok(Some((file.ok_or("items needs a file")?, out)))
}

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

fn read(file: &PathBuf, tmp: &std::path::Path) -> Result<String> {
    let name = file
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let mzml = name.to_lowercase().ends_with(".mzml");
    let sample = mzlab_core::lr::Sample::default();
    let idx = tmp.join("0.mzidx");
    crate::flow_pace::build_index_of(
        file,
        "",
        &idx,
        mzlab_core::budget_auto::native_index_budget(),
    )?;
    let mut r = Routes::new(crate::open(&idx)?);
    let items = if mzml {
        let f = File::open(file).map_err(io)?;
        let size = f.metadata().map_err(io)?.len();
        mzlab_core::lr::open_items(&mut r.idx, &mut Mzml::new(f, size), &name, &sample)?
    } else {
        let mut l = mzlab_lettori::apri(
            crate::flow_set::file_set(file)?,
            &mzlab_core::readers::Limits::desktop(),
        )?;
        let shell = mzlab_core::readers_mzml::guscio(l.as_mut(), 0)?;
        let size = shell.len() as u64;
        mzlab_core::lr::open_items(&mut r.idx, &mut Mzml::new(shell, size), &name, &sample)?
    };
    let body: Vec<String> = items
        .iter()
        .map(|i| format!("{{\"info\":{},\"hr\":{}}}", i.info, i.hr))
        .collect();
    Ok(format!(
        "{{\"schema\":\"items/1\",\"items\":[{}]}}",
        body.join(",")
    ))
}

pub fn run(args: Vec<String>) -> Outcome {
    let (file, out) = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-items{k}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp)
        .map_err(io)
        .and_then(|_| read(&file, &tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    match r {
        Err(e) => Outcome::fail_error(&e),
        Ok(t) => crate::emit(t + "\n", &out),
    }
}

#[cfg(test)]
#[path = "items_tests.rs"]
mod tests;
