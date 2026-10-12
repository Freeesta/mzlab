//! The map routes of the session over unit-resolution centroid files, from their indexes: `api/map`, `api/mapsnap`, `api/mappunti`,
//! `api/mappunti3d` (`_map`, `_map_points`, `_map_sticks` of `mzlab/api_aiuti.py`), for unit-resolution and high-resolution files, centroids
//! or profile points. `Ok(None)` = Python's: a parameter that Python must reject with its own error.

use std::collections::HashMap;

use base64::Engine;

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::iteminfo::group_is_hr;
use crate::json::Value;
use crate::jsonb::{floats, opt, text, Obj};
use crate::mapgrid::{diff_tolerant, map_cells, map_cells_hr, psum32, py_round, rt_groups, session_grid, subtract_constant, Grid, Point};
use crate::originroute::ItemMeta;
use crate::routes::{Query, Routes};

const STICKS_MAX: usize = 200_000;

type Table<R> = HashMap<u32, Routes<R>>;

/// What the page asks of `api/map`.
struct Ask {
    level: u8,
    mean: bool,
    bg: bool,
    stretch: Option<(f64, f64)>,
    /// (rt0, rt1, mz0, mz1, nrt, nmz) of a region; None = the grid of the session.
    region: Option<(f64, f64, f64, f64, usize, usize)>,
    /// `mzscale=log` was asked; in force (cells of equal ppm) only for a region of a file read as high resolution.
    log: bool,
}

fn number(q: &Query, k: &str) -> Option<f64> {
    q.num(k).ok().flatten()
}

fn index(q: &Query, k: &str) -> Option<i64> {
    q.some(k)?.parse().ok()
}

fn b64(bytes: Vec<u8>) -> Value {
    text(&base64::engine::general_purpose::STANDARD.encode(bytes))
}

fn f32s(a: &[f32]) -> Value {
    b64(a.iter().flat_map(|v| v.to_le_bytes()).collect())
}

/// Every file of the session has its index (an item without a route is an MRM file).
fn indexed<R: ReadAt>(items: &[ItemMeta], routes: &Table<R>) -> bool {
    items.iter().all(|it| routes.contains_key(&it.slot) || !it.full)
}

fn route<'a, R: ReadAt>(items: &[ItemMeta], routes: &'a mut Table<R>, k: i64) -> Option<&'a mut Routes<R>> {
    let it = items.get(usize::try_from(k).ok()?)?;
    routes.get_mut(&it.slot)
}

/// `Session.grid(level)` over the files of the session.
fn grid_of<R: ReadAt>(items: &[ItemMeta], routes: &mut Table<R>, level: u8) -> Result<Grid> {
    let (mut rts, mut mzs, mut hr) = (Vec::new(), Vec::new(), false);
    for it in items {
        let Some(r) = routes.get_mut(&it.slot) else { continue };
        let at: Vec<f64> = r.idx.scans.iter().filter(|s| s.level == level).map(|s| s.rt).collect();
        if at.is_empty() {
            continue;
        }
        hr |= group_is_hr(&r.idx.scans, level == 1);
        rts.push(at.iter().cloned().fold(f64::INFINITY, f64::min));
        rts.push(at.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
        if let Some(si) = r.idx.section_of(level, None) {
            if let Some((lo, hi)) = r.idx.mz_range(si)? {
                mzs.extend([lo, hi]);
            }
        }
    }
    Ok(session_grid(&rts, &mzs, hr))
}

fn parse_ask(q: &Query, with_ref: bool) -> Option<Ask> {
    let mean = match q.some("agg").unwrap_or(if with_ref { "mean" } else { "max" }) {
        "max" => false,
        "mean" => true,
        _ => return None,
    };
    if !matches!(q.some("mzscale").unwrap_or("lin"), "lin" | "log") {
        return None;
    }
    let bg = matches!(q.get("bg"), Some("1" | "true"));
    let stretch = match (q.some("bgrt0"), q.some("bgrt1")) {
        (Some(a), Some(b)) => Some((a.parse().ok()?, b.parse().ok()?)),
        _ => None,
    };
    let region = if q.some("rt0").is_some() {
        let (rt0, rt1, mz0, mz1) = (number(q, "rt0")?, number(q, "rt1")?, number(q, "mz0")?, number(q, "mz1")?);
        if !(rt1 > rt0 && mz1 > mz0 && mz0 > 0.0) {
            return None;
        }
        let axis = |k: &str| q.get(k).map_or(Some(400), |v| v.parse::<i64>().ok()).map(|v| v.clamp(1, 1024) as usize);
        Some((rt0, rt1, mz0, mz1, axis("nrt")?, axis("nmz")?))
    } else {
        None
    };
    Some(Ask { level: q.level(), mean, bg, stretch, region, log: q.some("mzscale") == Some("log") })
}

/// The map of file `k` on the grid `g` (`mz1` = end of the m/z axis), with the background taken away when asked.
fn one_map<R: ReadAt>(r: &mut Routes<R>, a: &Ask, g: &Grid, mz1: f64) -> Result<Vec<f32>> {
    let m = match a.region {
        Some(_) => r.region_map(a.level, (g.rt0, g.rt1), g.nrt, (g.mz0, mz1), (g.nmz, a.log), a.mean)?,
        None => r.ion_map(a.level, g, a.mean)?.to_vec(),
    };
    if !(a.bg || a.stretch.is_some()) {
        return Ok(m);
    }
    let bg = match a.stretch {
        Some(s) => r.stretch_background(a.level, s, (g.mz0, mz1), (g.nmz, a.log))?,
        None => r.run_background(a.level, (g.mz0, mz1), (g.nmz, a.log), a.mean)?,
    };
    Ok(subtract_constant(&m, g.nmz, &bg))
}

/// `_map_factor`: the normalisation of a whole map before a comparison.
fn norm_factor<R: ReadAt>(r: &mut Routes<R>, a: &Ask, sg: &Grid, norm: &str) -> Result<f32> {
    if norm != "max" && norm != "tic" {
        return Ok(1.0);
    }
    let m = r.ion_map(a.level, sg, a.mean)?;
    let t = if norm == "max" { m.iter().cloned().fold(0.0f32, f32::max) as f64 } else { psum32(&m) as f64 };
    Ok(if t > 0.0 { ((if norm == "max" { 1.0 } else { 1000.0 }) / t) as f32 } else { 1.0 })
}

fn extent_value(e: Option<((f64, f64), (f64, f64))>) -> Value {
    match e {
        Some((rt, mz)) => Obj::new().put("rt", floats(&[rt.0, rt.1])).put("mz", floats(&[mz.0, mz.1])).done(),
        None => Obj::new().put("rt", Value::Null).put("mz", Value::Null).done(),
    }
}

/// The JSON of a map: the grid (with `dlog` for cells of equal ppm), what was asked, and (on the grid of the session) the extent of the data.
fn map_head(a: &Ask, g: &Grid, mz1: f64, hr: bool) -> Obj {
    let mut o = Obj::new().n("rt0", g.rt0).n("rt1", g.rt1).i("nrt", g.nrt as i64).n("mz0", g.mz0);
    o = if a.region.is_none() { o.n("dmz", g.dmz).i("nmz", g.nmz as i64) } else { o.n("mz1", mz1).i("nmz", g.nmz as i64) };
    if a.region.is_some() {
        o = if a.log { o.n("dlog", (mz1 / g.mz0).ln() / g.nmz as f64) } else { o.n("dmz", g.dmz) };
    }
    o = o.i("level", a.level as i64).b("region", a.region.is_some()).put("agg", text(if a.mean { "mean" } else { "max" })).b("log", a.log);
    o.b("bg", a.bg || a.stretch.is_some()).put("bgstretch", a.stretch.map_or(Value::Null, |s| floats(&[s.0, s.1]))).b("hr", hr)
}

/// `api/map`: grid of the session or a region, optionally minus a reference file.
fn map<R: ReadAt>(items: &[ItemMeta], routes: &mut Table<R>, q: &Query) -> Result<Option<String>> {
    let (Some(k), refk) = (index(q, "k"), index(q, "ref")) else { return Ok(None) };
    if q.some("ref").is_some() && refk.is_none() {
        return Ok(None);
    }
    let Some(mut a) = parse_ask(q, refk.is_some()) else { return Ok(None) };
    let Some(r) = route(items, routes, k) else { return Ok(None) };
    let hr = group_is_hr(&r.idx.scans, a.level == 1);
    let (g, mz1) = match a.region {
        Some((rt0, rt1, mz0, mz1, nrt, nmz)) => {
            a.log &= hr;
            let (m0, m1, n) = if hr { (mz0, mz1, map_cells_hr(mz0, mz1, nmz, a.log)) } else { map_cells(mz0, mz1, nmz) };
            (Grid { rt0, rt1, nrt, mz0: m0, dmz: (m1 - m0) / n as f64, nmz: n }, m1)
        }
        None => {
            a.log = false;
            let g = grid_of(items, routes, a.level)?;
            (g, g.mz0 + g.nmz as f64 * g.dmz)
        }
    };
    let Some(r) = route(items, routes, k) else { return Ok(None) };
    let mut m = one_map(r, &a, &g, mz1)?;
    let mut o = map_head(&a, &g, mz1, hr);
    if a.region.is_none() {
        o = o.put("extent", extent_value(r.data_extent(a.level)?));
    }
    if let Some(rk) = refk {
        let norm = q.get("norm").unwrap_or("abs");
        let sg = grid_of(items, routes, a.level)?;
        let rttol = number(q, "rttol").unwrap_or(0.1);
        let w = py_round(rttol / ((g.rt1 - g.rt0) / g.nrt as f64).max(1e-9)).max(0.0) as usize;
        let mut wm = 0;
        if a.log {
            let mztol = match q.num("mztol") {
                Ok(v) => v.unwrap_or(5.0),
                Err(_) => return Ok(None),
            };
            let dlog = (mz1 / g.mz0).ln() / g.nmz as f64;
            wm = py_round(mztol * 1e-6 / dlog.max(1e-12)).clamp(0.0, 20.0) as usize;
        }
        let Some(rr) = route(items, routes, rk) else { return Ok(None) };
        let b = one_map(rr, &a, &g, mz1)?;
        let fb = norm_factor(rr, &a, &sg, norm)?;
        let Some(r) = route(items, routes, k) else { return Ok(None) };
        let fa = norm_factor(r, &a, &sg, norm)?;
        m = diff_tolerant(&m.iter().map(|v| v * fa).collect::<Vec<_>>(), &b.iter().map(|v| v * fb).collect::<Vec<_>>(), g.nrt, g.nmz, w, wm);
        o = o.i("ref", rk).n("rttol", rttol).i("w", w as i64).i("wm", wm as i64);
    }
    Ok(Some(crate::jsonb::write(&o.put("data", f32s(&m)).done())))
}

/// `api/mapsnap`: the local maximum near a click, `{}` when there is none.
fn snap<R: ReadAt>(items: &[ItemMeta], routes: &mut Table<R>, q: &Query) -> Result<Option<String>> {
    let (Some(k), Some(rt), Some(mz)) = (index(q, "k"), number(q, "rt"), number(q, "mz")) else { return Ok(None) };
    let Some(r) = route(items, routes, k) else { return Ok(None) };
    let found = r.snap(q.level(), rt, mz, number(q, "dmz").unwrap_or(1.0))?;
    let o = found.map_or_else(Obj::new, |(rt, mz)| Obj::new().n("rt", rt).n("mz", mz));
    Ok(Some(crate::jsonb::write(&o.done())))
}

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    match s.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => s[n / 2],
        n => (s[n / 2 - 1] + s[n / 2]) / 2.0,
    }
}

/// The marked points "rt,mz;rt,mz;..."; None when one is not two numbers.
fn parse_points(s: &str) -> Option<Vec<(f64, f64)>> {
    let mut out = Vec::new();
    for p in s.split(';').filter(|p| !p.trim().is_empty()) {
        let (rt, mz) = p.split_once(',')?;
        out.push((rt.trim().parse().ok()?, mz.trim().parse().ok()?));
    }
    Some(out)
}

/// `map_measure` on this file: intensity at the apex (A) and S/N from the XIC of `c ± tol`.
fn measure<R: ReadAt>(r: &mut Routes<R>, level: u8, rt: f64, c: f64, tol: f64) -> Result<(f64, Option<f64>)> {
    let (rts, ya) = r.xic_arrays(c, tol, level)?;
    if rts.is_empty() {
        return Ok((0.0, None));
    }
    let i = rts.iter().enumerate().fold(0, |b, (j, v)| if (v - rt).abs() < (rts[b] - rt).abs() { j } else { b });
    let ia = ya[i];
    let med = median(&ya);
    let mad = median(&ya.iter().map(|v| (v - med).abs()).collect::<Vec<_>>()) * 1.4826;
    Ok((ia, (mad > 0.0).then(|| (ia - med) / mad)))
}

/// `api/mappunti`: the measures of the marked points, in groups of RT.
fn marked<R: ReadAt>(items: &[ItemMeta], routes: &mut Table<R>, q: &Query) -> Result<Option<String>> {
    let (Some(k), level) = (index(q, "k"), q.level()) else { return Ok(None) };
    let refk = index(q, "ref");
    let Some(pts) = parse_points(q.get("pts").unwrap_or("")) else { return Ok(None) };
    let ppm = number(q, "ppm").unwrap_or(0.0);
    let win = |mz: f64| if ppm > 0.0 { (mz, mz * ppm * 1e-6) } else { (py_round(mz) + 0.3, 0.5) };
    let Some(r) = route(items, routes, k) else { return Ok(None) };
    let mut rows = Vec::new();
    for &(rt, mz) in &pts {
        let (c, tol) = win(mz);
        let (ia, sn) = measure(r, level, rt, c, tol)?;
        rows.push(Point { rt, mz, ia, ib: None, sn, g: 0, dm: 0.0 });
    }
    if let Some(rk) = refk {
        let Some(rr) = route(items, routes, rk) else { return Ok(None) };
        for p in rows.iter_mut() {
            let (c, tol) = win(p.mz);
            let (rb, yb) = rr.xic_arrays(c, tol, level)?;
            p.ib = Some((0..rb.len()).filter(|&i| (rb[i] - p.rt).abs() <= 0.1).map(|i| yb[i]).fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v)))).unwrap_or(0.0));
        }
    }
    rt_groups(&mut rows, number(q, "grp").unwrap_or(0.05));
    let out = rows.iter().map(|p| {
        let d = p.ib.map(|b| p.ia - b);
        let ratio = p.ib.filter(|&b| b != 0.0).map(|b| p.ia / b);
        Obj::new().n("rt", p.rt).n("mz", p.mz).n("ia", p.ia).put("ib", opt(p.ib)).put("sn", opt(p.sn)).i("g", p.g as i64).n("dm", p.dm).put("d", opt(d)).put("ratio", opt(ratio)).done()
    });
    Ok(Some(crate::jsonb::write(&Obj::new().put("rows", Value::Arr(out.collect())).done())))
}

/// `api/mappunti3d`: the true points of a region for the sticks of the 3D viewer.
fn sticks<R: ReadAt>(items: &[ItemMeta], routes: &mut Table<R>, q: &Query) -> Result<Option<String>> {
    let (Some(k), Some(rt0), Some(rt1), Some(mz0), Some(mz1)) = (index(q, "k"), number(q, "rt0"), number(q, "rt1"), number(q, "mz0"), number(q, "mz1")) else { return Ok(None) };
    let cap = match q.get("max") {
        None => STICKS_MAX,
        Some(v) => match v.parse::<i64>() {
            Ok(v) => v.clamp(1, STICKS_MAX as i64) as usize,
            Err(_) => return Ok(None),
        },
    };
    if !(rt1 > rt0 && mz1 > mz0) {
        return Ok(None);
    }
    let level = q.level();
    let Some(r) = route(items, routes, k) else { return Ok(None) };
    let s = r.sticks(level, (rt0, rt1), (mz0, mz1), cap)?;
    let o = Obj::new().i("k", k).i("level", level as i64).n("rt0", rt0).n("rt1", rt1).n("mz0", mz0).n("mz1", mz1).i("n", s.rt.len() as i64).i("total", s.total as i64);
    let mz: Vec<u8> = s.mz.iter().flat_map(|v| v.to_le_bytes()).collect();
    Ok(Some(crate::jsonb::write(&o.put("rt", f32s(&s.rt)).put("mz", b64(mz)).put("inten", f32s(&s.inten)).done())))
}

/// Answer for one of the map routes, or `None` when Python must answer.
pub fn map_route<R: ReadAt>(path: &str, items: &[ItemMeta], routes: &mut Table<R>, q: &Query) -> Result<Option<String>> {
    if !matches!(path, "/api/map" | "/api/mapsnap" | "/api/mappunti" | "/api/mappunti3d") || !indexed(items, routes) {
        return Ok(None);
    }
    match path {
        "/api/map" => map(items, routes, q),
        "/api/mapsnap" => snap(items, routes, q),
        "/api/mappunti" => marked(items, routes, q),
        _ => sticks(items, routes, q),
    }
}
