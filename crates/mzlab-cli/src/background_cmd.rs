//! `mzlab-cli background FILE`: the TIC or BPC of a file without its chemical background, the JSON of `api/tic_background`
//! (`mzlab_core::ticbg_route`, the same function as the site and the app). `--set NAME=VALUE` are the query names of the route.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::routes::{Query, Routes};

use crate::Outcome;

pub const HELP: &str = "mzlab-cli background: the TIC or BPC of a file without its chemical background, as JSON.

usage: mzlab-cli background FILE [--blank FILE] [--set NAME=VALUE...] [--out PATH]

  FILE        an mzML, or any single file the app reads (.raw, mzXML, ...)
  --blank     a blank sample: with mode=blank its persistent ions and their levels are the background
  --set       kind=tic|bpc, level=N, mode=auto|quiet|blank, action=subtract|exclude, persist=0.8, res=auto|unit|hr,
              bin=1 (Da), ppm=10, block=0.01, window=15, floor=1|0, rt0=MIN, rt1=MIN (mode=quiet),
              ex=MZ:TOL,... (windows taken out), keep=MZ,... (ions left raw); NAME may carry an instance prefix (background.mode=quiet)
  --out PATH  write there instead of the standard output
  output      {\"rt\",\"y\",\"raw\",\"kind\",\"ions\":[{mz,lo,hi,persistence,cv,level,share,confidence}],\"removed\",\"windows\",\"floor\",\"params\"}";

const NAMES: [&str; 15] = ["kind", "level", "mode", "action", "persist", "res", "bin", "ppm", "block", "window", "floor", "rt0", "rt1", "ex", "keep"];

struct Request {
    file: PathBuf,
    blank: Option<PathBuf>,
    query: Vec<(String, String)>,
    out: Option<PathBuf>,
}

fn parse(args: Vec<String>) -> std::result::Result<Option<Request>, String> {
    let mut it = args.into_iter();
    let (mut file, mut blank, mut query, mut out) = (None, None, Vec::new(), None);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--blank" => blank = Some(PathBuf::from(it.next().ok_or("--blank needs a file")?)),
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            "--set" => {
                let kv = it.next().ok_or("--set needs NAME=VALUE")?;
                let (k, v) = kv.split_once('=').ok_or("--set needs NAME=VALUE")?;
                let k = k.rsplit('.').next().unwrap_or(k);
                if !NAMES.contains(&k) {
                    return Err(format!("unknown parameter {k}"));
                }
                query.retain(|(n, _): &(String, String)| n != k);
                query.push((k.to_string(), v.to_string()));
            }
            "--help" | "-h" => return Ok(None),
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            o if file.is_none() => file = Some(PathBuf::from(o)),
            o => return Err(format!("one file only ({o})")),
        }
    }
    let file = file.ok_or("background needs a file (see --help)")?;
    Ok(Some(Request { file, blank, query, out }))
}

fn index(file: &Path, tmp: &Path, k: u32) -> Result<Routes<std::fs::File>> {
    let idx = tmp.join(format!("{k}.mzidx"));
    crate::flow_pace::build_index_of(file, "", &idx, mzlab_core::budget_auto::native_index_budget())?;
    Ok(Routes::new(crate::open(&idx)?))
}

fn read(c: &Request, tmp: &Path) -> Result<String> {
    let mut files = HashMap::new();
    files.insert(0, index(&c.file, tmp, 0)?);
    let mut q = Query(c.query.clone());
    if let Some(b) = &c.blank {
        files.insert(1, index(b, tmp, 1)?);
        q.0.push(("blank".into(), "1".into()));
    }
    let blank = if c.blank.is_some() { 1 } else { -1 };
    mzlab_core::ticbg_route::route_files(&mut files, 0, blank, &q)?
        .ok_or_else(|| Error::new("err.request.badValue").with("name", "file"))
}

pub fn run(args: Vec<String>) -> Outcome {
    let c = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-background{k}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp)
        .map_err(|e| Error::new("err.file.index").with("detail", e))
        .and_then(|_| read(&c, &tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    match r {
        Err(e) => Outcome::fail_error(&e),
        Ok(t) => crate::emit(t + "\n", &c.out),
    }
}

#[cfg(test)]
#[path = "background_cmd_tests.rs"]
mod tests;
