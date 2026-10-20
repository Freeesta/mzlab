//! `mzlab-cli flow report RUNDIR [--out FILE]`: the `corsa/1` report of a run folder, the one `examples/flow.rs` leaves (`prov.json` next to the files the steps made).

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{corsa_build, Uscita};
use mzlab_core::json::parse;

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// The `corsa/1` text of the folder: `prov.json` and every `.json` file beside it.
pub fn report(dir: &Path) -> Result<String> {
    let prov = std::fs::read(dir.join("prov.json")).map_err(io)?;
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(io)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json") && !p.ends_with("prov.json"))
        .collect();
    names.sort();
    let mut kept = Vec::new();
    for p in &names {
        let bytes = std::fs::read(p).map_err(io)?;
        kept.push((
            p.file_name()
                .map_or(String::new(), |n| n.to_string_lossy().into_owned()),
            bytes,
        ));
    }
    let uscite: Vec<Uscita> = kept
        .iter()
        .map(|(n, b)| Uscita { name: n, bytes: b })
        .collect();
    Ok(corsa_build(&parse(&prov)?, &uscite)?.json())
}

/// The arguments after `flow report`.
pub fn run(mut args: impl Iterator<Item = String>) -> std::result::Result<Vec<u8>, String> {
    let mut dir = None;
    let mut out = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out = Some(args.next().ok_or("--out needs a path")?),
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            _ if dir.is_none() => dir = Some(a),
            _ => return Err("one run folder only".into()),
        }
    }
    let dir = dir.ok_or("no run folder")?;
    let text = report(Path::new(&dir)).map_err(|e| e.to_string())?;
    match out {
        None => Ok(text.into_bytes()),
        Some(p) => std::fs::write(&p, text)
            .map(|_| Vec::new())
            .map_err(|e| format!("{p}: {e}")),
    }
}
