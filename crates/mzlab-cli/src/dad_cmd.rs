//! `mzlab-cli dad subtract|spectrum|trace|ms-at|delay`: the DAD / PDA matrix of a file for agents, the same engine routes as the site and the app (`itemroutes`,
//! `wiff::dad_ops`). Every answer is JSON on stdout with stable keys; progress and errors go to stderr; with `--json` an error is also an `error/1` on stdout.
//! Exit codes: 0 done, 1 data error (no DAD, nothing overlaps), 2 usage.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::itemroutes::{pda_dad, pda_sub_args, pda_subtract_data, PdaView};
use mzlab_core::mzmlhead::{Chrom, Mzml};
use mzlab_core::routes::{Query, Routes};
use mzlab_core::wiff::dad::DadData;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli dad: the optical (DAD / PDA) matrix of a file as JSON, for agents.

usage: mzlab-cli dad subtract A B [--view report|map|spectrum|trace] [--scale none|max|tic|fit|FACTOR] [--shift MIN] [--rt T0[,T1]] [--nm N0[,N1]]
       mzlab-cli dad spectrum FILE --rt T0[,T1] [--minus B [--scale S] [--shift MIN]]
       mzlab-cli dad trace FILE --nm N0[,N1] [--minus B [--scale S] [--shift MIN]]
       mzlab-cli dad ms-at FILE --rt T [--offset MIN] [--level N]
       mzlab-cli dad delay FILE

  subtract    A minus B: B is interpolated onto the grid of A, what B does not cover is dropped; the answer's `sub` says every step (ranges, steps, rows and
              columns kept, shift, scale, warnings). The default view is the report alone; map, spectrum and trace add the data of the difference
  spectrum    the UV spectrum (absorbance against nm) at the retention time(s) --rt; with --minus, of the difference
  trace       the UV trace (absorbance against minutes) at the wavelength(s) --nm; with --minus, of the difference
  ms-at       the MS1 spectrum and the TIC of the same file at the UV time + --offset (minutes, signed, default 0; the MS time is the UV time plus the offset)
  delay       the offset the engine estimates by matching the UV peaks with the TIC (`delay`, `peaks`, `corr`): evidence to accept or not, never applied
  --sample ID the sample of a file with several (.wiff); --minus-sample ID the same for B
  --json      accepted; every answer is JSON, and an error is also an error/1 document on stdout
  --out PATH  write there instead of the standard output
exit codes: 0 done, 1 data error (no DAD, nothing overlaps), 2 usage";

struct Req {
    verb: String,
    files: Vec<PathBuf>,
    sample: Option<u32>,
    minus_sample: Option<u32>,
    minus: Option<PathBuf>,
    view: String,
    scale: String,
    shift: f64,
    rt: Option<(f64, f64)>,
    nm: Option<(f64, f64)>,
    offset: f64,
    level: u8,
    json: bool,
    out: Option<PathBuf>,
}

fn pair(v: &str, what: &str) -> std::result::Result<(f64, f64), String> {
    let num = |x: &str| x.parse::<f64>().ok().filter(|x| x.is_finite());
    let (a, b) = v.split_once(',').unwrap_or((v, v));
    Ok((
        num(a).ok_or(format!("{what} needs numbers"))?,
        num(b).ok_or(format!("{what} needs numbers"))?,
    ))
}

/// A number of minutes (or any finite float) given to `--shift` and `--offset`.
fn minutes(v: String, what: &str) -> std::result::Result<f64, String> {
    v.parse()
        .ok()
        .filter(|x: &f64| x.is_finite())
        .ok_or(format!("{what} needs minutes"))
}

fn sample_id(v: String, what: &str) -> std::result::Result<u32, String> {
    v.parse().map_err(|_| format!("{what} needs a sample id"))
}

/// One option and its value into `r`; `Ok(false)` for `--help`.
fn option(
    r: &mut Req,
    a: &str,
    it: &mut impl Iterator<Item = String>,
) -> std::result::Result<bool, String> {
    let mut val = |what: &str| it.next().ok_or(format!("{what} needs a value"));
    match a {
        "--help" | "-h" => return Ok(false),
        "--json" => r.json = true,
        "--out" => r.out = Some(PathBuf::from(val("--out")?)),
        "--minus" => r.minus = Some(PathBuf::from(val("--minus")?)),
        "--view" => r.view = val("--view")?,
        "--scale" => r.scale = val("--scale")?,
        "--shift" => r.shift = minutes(val("--shift")?, "--shift")?,
        "--offset" => r.offset = minutes(val("--offset")?, "--offset")?,
        "--level" => {
            r.level = val("--level")?
                .parse()
                .map_err(|_| "--level needs a number")?
        }
        "--rt" => r.rt = Some(pair(&val("--rt")?, "--rt")?),
        "--nm" => r.nm = Some(pair(&val("--nm")?, "--nm")?),
        "--sample" => r.sample = Some(sample_id(val("--sample")?, "--sample")?),
        "--minus-sample" => {
            r.minus_sample = Some(sample_id(val("--minus-sample")?, "--minus-sample")?)
        }
        o if o.starts_with("--") => return Err(format!("unknown option {o}")),
        _ => r.files.push(PathBuf::from(a)),
    }
    Ok(true)
}

/// The checks that need the whole command line: the verb, the number of files, and the options that belong to one verb only.
fn check(r: &mut Req) -> std::result::Result<(), String> {
    if !["subtract", "spectrum", "trace", "ms-at", "delay"].contains(&r.verb.as_str()) {
        return Err(format!("unknown dad verb {}", r.verb));
    }
    let want = if r.verb == "subtract" { 2 } else { 1 };
    if r.files.len() != want {
        return Err(format!(
            "dad {} takes {want} file{}",
            r.verb,
            if want == 1 { "" } else { "s" }
        ));
    }
    if r.verb == "subtract" {
        r.minus = r.files.pop();
    }
    if r.verb != "subtract" && r.view != "report" {
        return Err("--view is for dad subtract".into());
    }
    if r.minus.is_some() && !["subtract", "spectrum", "trace"].contains(&r.verb.as_str()) {
        return Err("--minus is for spectrum and trace".into());
    }
    Ok(())
}

fn parse(args: Vec<String>) -> std::result::Result<Option<Req>, String> {
    let mut it = args.into_iter();
    let verb = it
        .next()
        .ok_or("dad needs a verb: subtract, spectrum, trace, ms-at or delay")?;
    if verb == "--help" || verb == "-h" {
        return Ok(None);
    }
    let mut r = Req {
        verb,
        files: vec![],
        sample: None,
        minus_sample: None,
        minus: None,
        view: "report".into(),
        scale: "none".into(),
        shift: 0.0,
        rt: None,
        nm: None,
        offset: 0.0,
        level: 1,
        json: false,
        out: None,
    };
    while let Some(a) = it.next() {
        if !option(&mut r, &a, &mut it)? {
            return Ok(None);
        }
    }
    check(&mut r)?;
    Ok(Some(r))
}

fn idx_err(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// The DAD matrix of `file`: an mzML through the engine's own route (`pda_dad`), any other file through its reader, the same matrix the site and the app serve.
fn dad_of(file: &Path, sample: Option<u32>) -> Result<Arc<DadData>> {
    if file.to_string_lossy().to_lowercase().ends_with(".mzml") {
        let f = File::open(file).map_err(idx_err)?;
        let size = f.metadata().map_err(idx_err)?.len();
        return pda_dad(&mut Mzml::new(f, size));
    }
    let name = file
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let label = sample.map_or(String::new(), |id| format!("{name}@{id}"));
    let mut l = mzlab_lettori::apri(
        crate::flow_set::file_set(file)?,
        &mzlab_core::readers::Limits::desktop(),
    )?;
    let ids: Vec<u32> = l.samples().iter().map(|s| s.id).collect();
    let k = crate::flow_set::sample_index(&label, &ids)?;
    l.dad_data(k)?
        .map(Arc::new)
        .ok_or_else(|| Error::new("err.pda.none"))
}

/// The routes and the mzML (or its shell) of `file`, for the MS side (`ms-at`, `delay`); `body` runs with them while the temporary index lives.
fn with_ms<T>(
    file: &Path,
    sample: Option<u32>,
    body: &mut dyn FnMut(&mut dyn MsSide) -> Result<T>,
) -> Result<T> {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-dad{k}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(idx_err)?;
    let r = ms_inner(file, sample, &tmp, body);
    let _ = std::fs::remove_dir_all(&tmp);
    r
}

trait MsSide {
    fn spectrum(&mut self, q: &Query) -> Result<String>;
    fn tic(&mut self, q: &Query) -> Result<String>;
    fn delay(&mut self) -> Result<String>;
}

struct Side<'a, R: ReadAt, M: ReadAt> {
    r: &'a mut Routes<R>,
    m: &'a mut Mzml<M>,
    chroms: &'a [Chrom],
}

impl<R: ReadAt, M: ReadAt> MsSide for Side<'_, R, M> {
    fn spectrum(&mut self, q: &Query) -> Result<String> {
        Ok(self
            .r
            .handle("/api/spectrum", q)?
            .unwrap_or_else(|| "null".into()))
    }
    fn tic(&mut self, q: &Query) -> Result<String> {
        Ok(self
            .r
            .handle("/api/chrom", q)?
            .unwrap_or_else(|| "null".into()))
    }
    fn delay(&mut self) -> Result<String> {
        self.r.delay(self.m, self.chroms)
    }
}

fn ms_inner<T>(
    file: &Path,
    sample: Option<u32>,
    tmp: &Path,
    body: &mut dyn FnMut(&mut dyn MsSide) -> Result<T>,
) -> Result<T> {
    let name = file
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let label = sample.map_or(String::new(), |id| format!("{name}@{id}"));
    let mzml = name.to_lowercase().ends_with(".mzml");
    let idx = tmp.join("0.mzidx");
    crate::flow_pace::build_index_of(
        file,
        &label,
        &idx,
        mzlab_core::budget_auto::native_index_budget(),
    )?;
    let mut r = Routes::new(crate::open(&idx)?);
    let end = r.idx.scans.iter().map(|s| s.end).max().unwrap_or(0);
    if mzml {
        let f = File::open(file).map_err(idx_err)?;
        let size = f.metadata().map_err(idx_err)?.len();
        let mut m = Mzml::new(f, size);
        let chroms = m.chromatograms(end)?;
        return body(&mut Side {
            r: &mut r,
            m: &mut m,
            chroms: &chroms,
        });
    }
    let mut l = mzlab_lettori::apri(
        crate::flow_set::file_set(file)?,
        &mzlab_core::readers::Limits::desktop(),
    )?;
    let ids: Vec<u32> = l.samples().iter().map(|s| s.id).collect();
    let k = crate::flow_set::sample_index(&label, &ids)?;
    let shell = mzlab_core::readers_mzml::guscio(l.as_mut(), k)?;
    let size = shell.len() as u64;
    let mut m = Mzml::new(shell, size);
    let chroms = m.chromatograms(end)?;
    body(&mut Side {
        r: &mut r,
        m: &mut m,
        chroms: &chroms,
    })
}

fn run_req(c: &Req) -> Result<String> {
    let a = &c.files[0];
    let window = |p: Option<(f64, f64)>, what: &str| {
        p.ok_or_else(|| Error::new("err.pda.window").with("need", what))
    };
    match c.verb.as_str() {
        "subtract" | "spectrum" | "trace" => {
            let da = dad_of(a, c.sample)?;
            let view = match (c.verb.as_str(), c.view.as_str()) {
                ("spectrum", _) | ("subtract", "spectrum") => {
                    let (x, y) = window(c.rt, "--rt")?;
                    ("spectrum", x, y)
                }
                ("trace", _) | ("subtract", "trace") => {
                    let (x, y) = window(c.nm, "--nm")?;
                    ("trace", x, y)
                }
                (_, "map") => ("map", 0.0, 0.0),
                _ => ("report", 0.0, 0.0),
            };
            let Some(b) = &c.minus else {
                return match view.0 {
                    "spectrum" => mzlab_core::wiff::dad_uv::dad_spectrum_json(&da, view.1, view.2),
                    _ => mzlab_core::wiff::dad_uv::dad_wavelength_json(&da, view.1, view.2),
                }
                .ok_or_else(|| Error::new("err.pda.none"));
            };
            let db = dad_of(b, c.minus_sample)?;
            let (o, pv) = pda_sub_args(
                if view.0 == "report" { "map" } else { view.0 },
                view.1,
                view.2,
                &c.scale,
                c.shift,
            )?;
            if view.0 == "report" {
                let (_, rep) = mzlab_core::wiff::dad_ops::dad_subtract(&da, &db, &o)?;
                return Ok(format!("{{\"sub\":{}}}", rep.to_json()));
            }
            debug_assert!(matches!(
                pv,
                PdaView::Map | PdaView::Spectrum(..) | PdaView::Wavelength(..)
            ));
            pda_subtract_data(&da, &db, &o, pv)
        }
        "ms-at" => {
            let (t, _) = window(c.rt, "--rt")?;
            let ms = t + c.offset;
            let spec = Query::parse(&format!("rt0={ms}&rt1={ms}&level={}", c.level));
            let tic = Query::parse(&format!("kind=tic&level={}", c.level));
            with_ms(a, c.sample, &mut |s| {
                Ok(format!(
                    "{{\"rt_uv\":{t},\"rt_ms\":{ms},\"offset_min\":{},\"level\":{},\"ms_file\":{},\"spectrum\":{},\"tic\":{}}}",
                    c.offset,
                    c.level,
                    json_str(&a.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned())),
                    s.spectrum(&spec)?,
                    s.tic(&tic)?
                ))
            })
        }
        _ => with_ms(a, c.sample, &mut |s| s.delay()),
    }
}

fn json_str(s: &str) -> String {
    let mut o = mzlab_core::json::Out::default();
    o.str(s);
    o.0
}

pub fn run(args: Vec<String>) -> Outcome {
    let c = match parse(args) {
        Ok(Some(c)) => c,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    match run_req(&c) {
        Ok(t) => crate::emit(t + "\n", &c.out),
        Err(e) => {
            let code =
                if e.key == "err.pda.scale" || e.key == "err.pda.view" || e.key == "err.pda.window"
                {
                    crate::exit::USAGE
                } else {
                    crate::exit::DATA
                };
            Outcome::fail_error(&e)
                .with_code(code)
                .with_hint("see mzlab-cli dad --help")
        }
    }
}

#[cfg(test)]
#[path = "dad_tests.rs"]
mod tests;
