//! `api/tic_background`: the TIC or BPC of one level with its chemical background removed (`ticbg`), from the index of the file, with the
//! background ions, the canonical parameters (for the run record and the audit log) and the raw trace. Same code for the `.wasm`, the app and the CLI.
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::json::{round, Out};
use crate::routes::{Query, Routes};
use crate::ticbg::{blank_profile, clean, flagged, presence, Action, BlankProfile, Mode, Params};

/// The parameters of a request (names of `docs/agenti/tic-fondo.md`); unknown values are `err.request.badValue`.
#[inline(never)]
pub fn params_of(q: &Query, hr_file: bool) -> Result<Params> {
    let bad = |k: &str| Error::new("err.request.badValue").with("name", k);
    let mut p = Params { hr: hr_file, ..Params::default() };
    p.mode = match q.some("mode").unwrap_or("auto") {
        "auto" => Mode::Auto,
        "quiet" => Mode::Quiet,
        "blank" => Mode::Blank,
        _ => return Err(bad("mode")),
    };
    p.action = match q.some("action").unwrap_or("subtract") {
        "subtract" => Action::Subtract,
        "exclude" => Action::Exclude,
        _ => return Err(bad("action")),
    };
    p.hr = match q.some("res") {
        None | Some("auto") => hr_file,
        Some("unit") => false,
        Some("hr") => true,
        _ => return Err(bad("res")),
    };
    let pos = |k: &str, v: Option<f64>, lo: f64, hi: f64| -> Result<Option<f64>> {
        match v {
            Some(x) if !(x > lo && x <= hi) => Err(bad(k)),
            o => Ok(o),
        }
    };
    if let Some(v) = pos("persist", q.num("persist")?, 0.0, 1.0)? { p.persist = v; }
    if let Some(v) = pos("bin", q.num("bin")?, 0.0, 10.0)? { p.bin_da = v; }
    if let Some(v) = pos("ppm", q.num("ppm")?, 0.0, 1000.0)? { p.bin_ppm = v; }
    if let Some(v) = pos("block", q.num("block")?, 0.0, 0.5)? { p.block = v; }
    if let Some(v) = pos("window", q.num("window")?, 0.0, 999.0)? { p.window = v as usize | 1; }
    if p.mode == Mode::Quiet {
        match (q.num("rt0")?, q.num("rt1")?) {
            (Some(a), Some(b)) if a < b => p.quiet = Some((a, b)),
            _ => return Err(bad("rt0")),
        }
    }
    let list = |k: &str| -> Result<Vec<f64>> {
        q.some(k).map_or(Ok(Vec::new()), |s| s.split(',').map(|x| x.trim().parse::<f64>().map_err(|_| bad(k))).collect())
    };
    // `ex=MZ:TOL,MZ:TOL`: manual windows, TOL in Da (default 0.5)
    for w in q.some("ex").map(|s| s.split(',').collect::<Vec<_>>()).unwrap_or_default() {
        let (m, t) = w.split_once(':').unwrap_or((w, "0.5"));
        let (m, t) = (m.trim().parse::<f64>().map_err(|_| bad("ex"))?, t.trim().parse::<f64>().map_err(|_| bad("ex"))?);
        if !(m > 0.0 && t >= 0.0) {
            return Err(bad("ex"));
        }
        p.windows.push((m, t));
    }
    p.keep = list("keep")?;
    p.floor = match q.some("floor").unwrap_or("1") {
        "1" => true,
        "0" => false,
        _ => return Err(bad("floor")),
    };
    Ok(p)
}

/// The canonical text of the parameters: keys sorted, every default written, so a record replays the same trace.
#[inline(never)]
pub fn params_json(p: &Params) -> String {
    let mut o = Out::default();
    o.raw("{\"action\":").str(if p.action == Action::Exclude { "exclude" } else { "subtract" });
    o.raw(",\"bin\":").num(p.bin_da).raw(",\"block\":").num(p.block);
    o.raw(",\"ex\":[");
    for (i, (m, t)) in p.windows.iter().enumerate() {
        o.raw(if i > 0 { ",[" } else { "[" }).num(*m).raw(",").num(*t).raw("]");
    }
    o.raw("],\"floor\":").raw(if p.floor { "true" } else { "false" });
    o.raw(",\"keep\":").floats(&p.keep);
    o.raw(",\"mode\":").str(match p.mode { Mode::Auto => "auto", Mode::Quiet => "quiet", Mode::Blank => "blank" });
    o.raw(",\"persist\":").num(p.persist).raw(",\"ppm\":").num(p.bin_ppm);
    o.raw(",\"quiet\":");
    match p.quiet {
        Some((a, b)) => o.raw("[").num(a).raw(",").num(b).raw("]"),
        None => o.raw("null"),
    };
    o.raw(",\"res\":").str(if p.hr { "hr" } else { "unit" }).raw(",\"window\":").int(p.window as i64).raw("}");
    o.0
}

fn section<R: ReadAt>(r: &Routes<R>, q: &Query) -> Option<usize> {
    r.idx.section_of(q.level(), r.pol)
}

/// Streams every peak of a section as (scan position, m/z, intensity) in m/z order; the first read error is kept.
#[inline(never)]
fn stream<R: ReadAt>(r: &mut Routes<R>, si: usize, err: &mut Option<Error>, f: &mut dyn FnMut(usize, f64, f64)) {
    let n = r.idx.sections[si].n_peaks;
    if let Err(e) = r.idx.each_peak(si, 0, n, f) {
        err.get_or_insert(e);
    }
}

/// The blank profile of `mode=blank`, from the blank file's index (its own resolution unless `res` says).
#[inline(never)]
pub fn blank_of<R: ReadAt>(r: &mut Routes<R>, q: &Query) -> Result<BlankProfile> {
    let p = params_of(&strip_mode(q), r.idx.hr1())?;
    let Some(si) = section(r, q) else { return Ok(Vec::new()) };
    let n = r.idx.sections[si].n_scans;
    let mut err = None;
    let out = blank_profile(n, &p, &mut |f| stream(r, si, &mut err, f));
    err.map_or(Ok(out), Err)
}

fn strip_mode(q: &Query) -> Query {
    Query(q.0.iter().filter(|(k, _)| k != "mode").cloned().collect())
}

/// The answer: `{"rt","y","raw","kind","ions":[{mz,lo,hi,persistence,cv,level,share,confidence}],"removed","windows","params"}`.
/// `None` = not this route's (a scan filter or a precursor: those traces are not whole levels). `blank` is required by `mode=blank`.
#[inline(never)]
pub fn tic_background<R: ReadAt>(r: &mut Routes<R>, blank: Option<&BlankProfile>, q: &Query) -> Result<Option<String>> {
    if q.some("filt").is_some() || q.some("prec").is_some() {
        return Ok(None);
    }
    let kind = q.some("kind").unwrap_or("tic");
    if kind != "tic" && kind != "bpc" {
        return Err(Error::new("err.request.badValue").with("name", "kind"));
    }
    let p = params_of(q, r.idx.hr1())?;
    if p.mode == Mode::Blank && blank.is_none() {
        return Err(Error::new("err.request.badValue").with("name", "blank"));
    }
    let Some(si) = section(r, q) else { return Ok(Some("{\"rt\":[],\"y\":[],\"raw\":[],\"ions\":[]}".into())) };
    let (rt, raw) = {
        let a = r.idx.arrays(si)?;
        (a.rt.clone(), if kind == "tic" { a.sumint.clone() } else { a.bp_inten.clone() })
    };
    let n = rt.len();
    let mut err = None;
    let pres = presence(n, &p, &mut |f| stream(r, si, &mut err, f));
    let flags = flagged(n, &p, &pres, blank);
    drop(pres);
    let c = clean(&rt, &p, &flags, blank, &mut |f| stream(r, si, &mut err, f));
    if let Some(e) = err {
        return Err(e);
    }
    let y = if kind == "tic" { &c.tic } else { &c.bpc };
    let mut o = Out::default();
    o.raw("{\"rt\":").rounded(&rt, 4).raw(",\"y\":").rounded(y, 1).raw(",\"raw\":").rounded(&raw, 1);
    o.raw(",\"kind\":").str(kind).raw(",\"ions\":[");
    for (i, ion) in c.ions.iter().enumerate() {
        o.raw(if i > 0 { ",{" } else { "{" });
        o.raw("\"mz\":").num(round(ion.mz, 4)).raw(",\"lo\":").num(round(ion.lo, 4)).raw(",\"hi\":").num(round(ion.hi, 4));
        o.raw(",\"persistence\":").num(round(ion.persistence, 3)).raw(",\"cv\":").num(round(ion.cv, 3));
        o.raw(",\"level\":").num(round(ion.level, 1)).raw(",\"share\":").num(round(ion.share, 4));
        o.raw(",\"confidence\":").num(round(ion.confidence, 3)).raw("}");
    }
    o.raw("],\"removed\":").num(round(c.removed, 4)).raw(",\"windows\":").num(round(c.windows, 4));
    o.raw(",\"floor\":").num(round(c.floor, 4));
    o.raw(",\"params\":").raw(&params_json(&p)).raw("}");
    Ok(Some(o.0))
}

/// The route over one file and its blank (`mode=blank`): the lazy `.wasm` `rotte`, which opens both indexes for the request.
#[inline(never)]
pub fn route_pair<R: ReadAt>(r: &mut Routes<R>, blank: Option<&mut Routes<R>>, q: &Query) -> Result<Option<String>> {
    let prof = match blank {
        Some(b) => Some(blank_of(b, q)?),
        None => None,
    };
    tic_background(r, prof.as_ref(), q)
}

/// The route over the files of the app (`ticbg` of `rpc.rs`): the file at `slot`, the blank at `blank` (`mode=blank`; < 0 = none).
/// `None` = a slot without index (not served here).
#[inline(never)]
pub fn route_files<R: ReadAt>(files: &mut std::collections::HashMap<u32, Routes<R>>, slot: u32, blank: i64, q: &Query) -> Result<Option<String>> {
    let prof = match u32::try_from(blank).ok().and_then(|b| files.get_mut(&b)) {
        Some(b) => Some(blank_of(b, q)?),
        None => None,
    };
    match files.get_mut(&slot) {
        Some(r) => tic_background(r, prof.as_ref(), q),
        None => Ok(None),
    }
}
