//! `mzlab-cli flow report RUNDIR [--out FILE]`: the `run-report/2` report of a run folder, the one `examples/flow.rs` leaves (`prov.json` next to the files the steps made).

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{run_report_build, Output};
use mzlab_core::json::parse;

use crate::Outcome;

/// A file or folder that cannot be read: the input failure of the contract (`err.cli.file`, exit 4) naming the path.
fn io(path: &Path) -> impl Fn(std::io::Error) -> Error + '_ {
    move |e| Error::new("err.cli.file").with("path", format!("{}: {e}", path.display()))
}

/// The `run-report/2` text of the folder: `prov.json` and every `.json` file beside it.
pub fn report(dir: &Path) -> Result<String> {
    let prov_path = dir.join("prov.json");
    let prov = std::fs::read(&prov_path).map_err(io(&prov_path))?;
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(io(dir))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json") && !p.ends_with("prov.json"))
        .collect();
    names.sort();
    let mut kept = Vec::new();
    for p in &names {
        let bytes = std::fs::read(p).map_err(io(p))?;
        kept.push((
            p.file_name()
                .map_or(String::new(), |n| n.to_string_lossy().into_owned()),
            bytes,
        ));
    }
    // the old `report/1` of a table is refused here (the CLI reads run folders; the site passes no output files), never read as "no table report"
    if kept
        .iter()
        .any(|(_, b)| b.starts_with(b"{\"schema\":\"report/1\""))
    {
        return Err(Error::new("err.flow.format.reportOld"));
    }
    let outputs: Vec<Output> = kept
        .iter()
        .map(|(n, b)| Output { name: n, bytes: b })
        .collect();
    Ok(run_report_build(&parse(&prov)?, &outputs)?.json())
}

/// `flow report RUNDIR [--out FILE]`: the report on stdout or in FILE; `--json` is the same document (it is JSON already).
pub fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    let (mut dir, mut out) = (None, None);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            s if s.starts_with("--") => return Outcome::fail(&format!("unknown option {s}")),
            _ if dir.is_none() => dir = Some(a),
            _ => return Outcome::fail("one run folder only"),
        }
    }
    let Some(dir) = dir else {
        return Outcome::fail("usage: mzlab-cli flow report RUNDIR [--out FILE]");
    };
    match report(Path::new(&dir)) {
        Ok(mut text) => {
            // one trailing newline, whether or not the document already ends with one
            if !text.ends_with('\n') {
                text.push('\n');
            }
            crate::emit(text, &out)
        }
        Err(e) => Outcome::fail_error(&e),
    }
}
