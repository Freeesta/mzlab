//! `mzlab-cli canali FILE [--out PATH]`: the channels (`canali/1`) of a file, the JSON of `api/canali`. An mzML also gives its SRM and SIC chromatograms;
//! any other file the app reads gives the channels of its scans.

use std::fs::File;
use std::path::PathBuf;

use mzlab_core::error::{Error, Result};
use mzlab_core::mzidx::DEFAULT_BUDGET;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::routes::Routes;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli canali: the channels of a file (TIC, BPC, one per scan type or precursor, one per SRM transition) as JSON (canali/1).

usage: mzlab-cli canali FILE [--out PATH]

  FILE        an mzML, or any single file the app reads (.raw, mzXML, ...); SRM and SIC channels only for an mzML
  --out PATH  write there instead of the standard output";

fn parse(args: Vec<String>) -> std::result::Result<Option<(PathBuf, Option<PathBuf>)>, String> {
    let (mut file, mut out) = (None, None);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ if file.is_none() => file = Some(PathBuf::from(a)),
            _ => return Err("canali takes one file".into()),
        }
    }
    Ok(Some((file.ok_or("canali needs a file")?, out)))
}

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// The JSON of `api/canali` for `file`, indexes kept in `tmp`.
pub fn canali(file: &std::path::Path, tmp: &std::path::Path) -> Result<String> {
    let idx = tmp.join("0.mzidx");
    crate::build_index(file, &idx, DEFAULT_BUDGET)?;
    let mut r = Routes::new(crate::open(&idx)?);
    let nome = file.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned());
    if nome.to_lowercase().ends_with(".mzml") {
        let scans_end = r.idx.scans.iter().map(|s| s.end).max().unwrap_or(0);
        let f = File::open(file).map_err(io)?;
        let size = f.metadata().map_err(io)?.len();
        let mut m = Mzml::new(f, size);
        let chroms = m.chromatograms(scans_end)?;
        let facts = mzlab_core::iteminfo::facts_from(&mut m, scans_end, chroms.len() as i64, true)?;
        r.pon_cromatogrammi(&nome, &chroms, &facts);
    } else {
        r.pon_cromatogrammi(&nome, &[], &Default::default());
    }
    Ok(r.canali().json())
}

pub fn run(args: Vec<String>) -> Outcome {
    let (file, out) = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-canali{k}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp).map_err(io).and_then(|_| canali(&file, &tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    match r {
        Err(e) => Outcome::fail(&format!("{} {:?}", e.key, e.params)),
        Ok(t) => crate::emit(t + "\n", &out),
    }
}
