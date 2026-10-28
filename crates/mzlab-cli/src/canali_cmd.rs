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

usage: mzlab-cli canali FILE [--canale ID [--rt T0[,T1]] [--n N]] [--out PATH]

  FILE        an mzML, or any single file the app reads (.raw, mzXML, ...); SRM, SIC, UV and analog channels only for an mzML
  --canale ID the trace of a UV or analog channel (uv:twc, uv:<nm>±<bw>, an:<i>) instead of the list; with --rt (minutes) the UV spectrum of uv:twc
  --n N       points at most per extreme of an analog trace (default 2000)
  --out PATH  write there instead of the standard output";

/// What was asked: the file, the channel to read (`--canale`, `--rt`, `--n`) and where to write.
struct Chiesta {
    file: PathBuf,
    out: Option<PathBuf>,
    canale: Option<String>,
    rt: Option<(f64, f64)>,
    n: i64,
}

fn parse(args: Vec<String>) -> std::result::Result<Option<Chiesta>, String> {
    let (mut file, mut out, mut canale, mut rt, mut n) = (None, None, None, None, 2000);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            "--canale" => canale = Some(it.next().ok_or("--canale needs a channel id")?),
            "--rt" => {
                let v = it.next().ok_or("--rt needs minutes")?;
                let num = |x: &str| x.parse::<f64>().ok().filter(|x| x.is_finite());
                let (a, b) = v.split_once(',').unwrap_or((&v, &v));
                rt = Some((
                    num(a).ok_or("--rt needs minutes")?,
                    num(b).ok_or("--rt needs minutes")?,
                ));
            }
            "--n" => {
                n = (it.next().ok_or("--n needs a number")?)
                    .parse()
                    .map_err(|_| "--n needs a number")?
            }
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ if file.is_none() => file = Some(PathBuf::from(a)),
            _ => return Err("canali takes one file".into()),
        }
    }
    if canale.is_none() && (rt.is_some() || n != 2000) {
        return Err("--rt and --n need --canale".into());
    }
    Ok(Some(Chiesta {
        file: file.ok_or("canali needs a file")?,
        out,
        canale,
        rt,
        n,
    }))
}

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// The JSON of `api/canali` for `file`, indexes kept in `tmp`; with `--canale`, the trace (or with `--rt` the UV spectrum) of that UV or analog channel
/// of an mzML: what `api/chrom?canale=` and `api/uvspec?canale=` answer.
fn leggi(c: &Chiesta, tmp: &std::path::Path) -> Result<String> {
    let file = &c.file;
    let idx = tmp.join("0.mzidx");
    crate::build_index(file, &idx, DEFAULT_BUDGET)?;
    let mut r = Routes::new(crate::open(&idx)?);
    let nome = file
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let mzml = nome.to_lowercase().ends_with(".mzml");
    if let (Some(id), false) = (&c.canale, mzml) {
        return Err(Error::new("err.canali.unknown").with("id", id));
    }
    if mzml {
        let scans_end = r.idx.scans.iter().map(|s| s.end).max().unwrap_or(0);
        let f = File::open(file).map_err(io)?;
        let size = f.metadata().map_err(io)?.len();
        let mut m = Mzml::new(f, size);
        let chroms = m.chromatograms(scans_end)?;
        if let Some(id) = &c.canale {
            return match c.rt {
                Some((a, b)) => mzlab_core::canali_uv::spettro(&mut m, id, a, b),
                None => mzlab_core::canali_uv::traccia(&mut m, &chroms, id, c.n),
            };
        }
        let facts = mzlab_core::iteminfo::facts_from(&mut m, scans_end, chroms.len() as i64, true)?;
        r.pon_cromatogrammi(&nome, &chroms, &facts);
    } else {
        r.pon_cromatogrammi(&nome, &[], &Default::default());
    }
    Ok(r.canali().json())
}

pub fn run(args: Vec<String>) -> Outcome {
    let c = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-canali{k}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp)
        .map_err(io)
        .and_then(|_| leggi(&c, &tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    match r {
        Err(e) => Outcome::fail(&format!("{} {:?}", e.key, e.params)),
        Ok(t) => crate::emit(t + "\n", &c.out),
    }
}

#[cfg(test)]
#[path = "canali_tests.rs"]
mod tests;
