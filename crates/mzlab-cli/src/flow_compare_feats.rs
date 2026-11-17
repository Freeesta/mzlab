//! Comparison metrics and JSON output for aligned feature tables (`.mzfeat`) and per-file feature lists (`.mzfeatf`).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::{parse, round, Out, Value};
use mzlab_core::pairing::abbina;
use mzlab_core::sortc::{by as sort_by, slice_by};
use mzlab_core::stats::rankdata;
use mzlab_core::step::DataType;
use mzlab_feat::recipe::is_unit_text;
use mzlab_feat::store::{origin_from_meta, read_file, read_table};
use mzlab_feat::table::no_peak;

use crate::flow::{all_inputs, io, read_all, write_made, FileRunner};
use crate::flow_read::{load, Loaded};

/// Matching tolerances for comparing aligned tables (`DataType::Table`) and feature lists (`DataType::Peaks`).
#[derive(Clone, Copy)]
pub struct Tol {
    pub ppm: f64,
    pub da: f64,
    pub rt: f64,
}

impl Default for Tol {
    fn default() -> Self {
        Self {
            ppm: 5.0,
            da: 0.3,
            rt: 0.2,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Json,
    Table,
    Peaks,
}

struct Feats {
    mz: Vec<f64>,
    rt: Vec<f64>,
    area: Vec<f64>,
    unit: bool,
}

fn blob_kind(bytes: &[u8]) -> Result<Kind> {
    if bytes.len() < 16 || &bytes[..8] != b"MZFEAT01" {
        return Ok(Kind::Json);
    }
    let n = usize::try_from(u64::from_le_bytes(bytes[8..16].try_into().unwrap()))
        .map_err(|_| Error::new("err.flow.compare.type"))?;
    let meta = bytes
        .get(16..16 + n)
        .ok_or_else(|| Error::new("err.flow.compare.type"))?;
    match parse(meta)?.get("format").and_then(Value::str) {
        Some("table") => Ok(Kind::Table),
        Some("file") => Ok(Kind::Peaks),
        _ => Err(Error::new("err.flow.compare.type")),
    }
}

pub(super) fn detect_kind(wrapped: &str, groups: &[&[(&str, &[u8])]]) -> Result<Kind> {
    let mut kind: Option<Kind> = None;
    for g in groups {
        for &(_, bytes) in *g {
            let k = blob_kind(bytes)?;
            if kind.is_some_and(|prev| prev != k) {
                return Err(Error::new("err.flow.compare.type"));
            }
            kind = Some(k);
        }
    }
    let kind = match kind {
        Some(k) => k,
        None if wrapped.starts_with("features.detect") => Kind::Peaks,
        None => return Err(Error::new("err.flow.compare.variants")),
    };
    if kind != Kind::Peaks && groups.iter().any(|g| g.len() != 1) {
        return Err(Error::new("err.flow.compare.variants"));
    }
    Ok(kind)
}

fn is_unit(meta: &Value) -> bool {
    meta.get("recipe")
        .and_then(Value::str)
        .is_some_and(is_unit_text)
}

fn load_table(bytes: &[u8]) -> Result<Feats> {
    let (t, meta) = read_table(bytes)?;
    let area = (0..t.len())
        .map(|r| {
            (0..t.n_files)
                .filter_map(|f| {
                    let c = t.cell(r, f);
                    let ok = t.state.get(c).is_none_or(|&s| !no_peak(s));
                    let a = t.area.get(c).copied().unwrap_or(0.0);
                    (ok && a.is_finite() && a > 0.0).then_some(a)
                })
                .sum()
        })
        .collect();
    Ok(Feats {
        mz: t.mz,
        rt: t.rt,
        area,
        unit: is_unit(&meta),
    })
}

fn stem(s: &str) -> &str {
    let b = s.rsplit('/').next().unwrap_or(s);
    b.rsplit_once('.').map_or(b, |p| p.0)
}

fn load_peaks(name: &str, bytes: &[u8], idx: usize) -> Result<(String, Feats)> {
    let (f, meta) = read_file(bytes)?;
    let key = origin_from_meta(&meta)
        .inputs
        .into_iter()
        .map(|(n, _)| n)
        .find(|n| !n.is_empty())
        .map(|n| stem(&n).to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| (!name.is_empty()).then(|| stem(name).to_string()))
        .unwrap_or_else(|| idx.to_string());
    Ok((
        key,
        Feats {
            mz: f.mz,
            rt: f.rt.into_iter().map(f64::from).collect(),
            area: f.area,
            unit: is_unit(&meta),
        },
    ))
}

fn match_da(a: &Feats, b: &Feats, da: f64, rt: f64) -> (Vec<usize>, Vec<usize>) {
    if a.mz.is_empty()
        || b.mz.is_empty()
        || !da.is_finite()
        || !rt.is_finite()
        || da <= 0.0
        || rt <= 0.0
    {
        return (Vec::new(), Vec::new());
    }
    let mut ob: Vec<usize> = (0..b.mz.len()).collect();
    sort_by(&mut ob, |&i, &j| b.mz[i].total_cmp(&b.mz[j]));
    let mut cands: Vec<(f64, usize, usize)> = Vec::new();
    for (ia, (&ma, &ra)) in a.mz.iter().zip(&a.rt).enumerate() {
        if !ma.is_finite() || !ra.is_finite() {
            continue;
        }
        let lo = ob.partition_point(|&ib| b.mz[ib] < ma - da);
        let hi = ob.partition_point(|&ib| b.mz[ib] <= ma + da);
        for &ib in &ob[lo..hi] {
            let (mb, rb) = (b.mz[ib], b.rt[ib]);
            if rb.is_finite() && (rb - ra).abs() <= rt {
                cands.push((((mb - ma) / da).hypot((rb - ra) / rt), ia, ib));
            }
        }
    }
    sort_by(&mut cands, |x, y| {
        x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2))
    });
    let (mut ua, mut ub, mut out) = (vec![false; a.mz.len()], vec![false; b.mz.len()], Vec::new());
    for (_, ia, ib) in cands {
        if !ua[ia] && !ub[ib] {
            (ua[ia], ub[ib]) = (true, true);
            out.push((ia, ib));
        }
    }
    sort_by(&mut out, |x, y| x.0.cmp(&y.0));
    out.into_iter().unzip()
}

fn area_rank_corr(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() < 2 || a.len() != b.len() {
        return None;
    }
    let (rx, ry) = (rankdata(a), rankdata(b));
    let n = rx.len() as f64;
    let (mx, my) = (rx.iter().sum::<f64>() / n, ry.iter().sum::<f64>() / n);
    let (mut vx, mut vy, mut cov) = (0.0, 0.0, 0.0);
    for (x, y) in rx.iter().zip(&ry) {
        let (dx, dy) = (x - mx, y - my);
        vx += dx * dx;
        vy += dy * dy;
        cov += dx * dy;
    }
    (vx > 0.0 && vy > 0.0).then(|| (cov / (vx * vy).sqrt()).clamp(-1.0, 1.0))
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    sort_by(&mut v, f64::total_cmp);
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    })
}

struct Stats {
    na: usize,
    nb: usize,
    both: usize,
    ov: Option<f64>,
    rc: Option<f64>,
    dmz: Vec<f64>,
    drt: Vec<f64>,
    log2: Vec<f64>,
}

fn stats_of(a: &Feats, b: &Feats, tol: Tol) -> Stats {
    let (ma, mb) = if !a.unit && !b.unit {
        abbina(&a.mz, &a.rt, &b.mz, &b.rt, tol.ppm, tol.rt)
    } else {
        match_da(a, b, tol.da, tol.rt)
    };
    let (na, nb, both) = (a.mz.len(), b.mz.len(), ma.len());
    let u = na + nb - both;
    let (mut aa, mut ab, mut dmz, mut drt, mut log2) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (&i, &j) in ma.iter().zip(&mb) {
        let (va, vb) = (a.area[i], b.area[j]);
        if va.is_finite() && vb.is_finite() {
            aa.push(va);
            ab.push(vb);
        }
        if a.mz[i] > 0.0 {
            dmz.push((b.mz[j] - a.mz[i]) / a.mz[i] * 1e6);
        }
        drt.push(b.rt[j] - a.rt[i]);
        if va > 0.0 && vb > 0.0 && va.is_finite() && vb.is_finite() {
            log2.push((vb / va).log2().abs());
        }
    }
    Stats {
        na,
        nb,
        both,
        ov: (u > 0).then(|| both as f64 / u as f64),
        rc: area_rank_corr(&aa, &ab),
        dmz,
        drt,
        log2,
    }
}

fn write_stats(
    o: &mut Out,
    ovk: &str,
    rck: &str,
    s: &Stats,
    dmz: &[f64],
    drt: &[f64],
    log2: &[f64],
) {
    for (k, n) in [
        ("count_a", s.na),
        ("count_b", s.nb),
        ("both", s.both),
        ("only_a", s.na - s.both),
        ("only_b", s.nb - s.both),
    ] {
        o.raw(",\"").raw(k).raw("\":").int(n as i64);
    }
    for (k, v) in [
        (ovk, s.ov),
        (rck, s.rc),
        ("median_dmz_ppm", median(dmz.to_vec())),
        ("median_drt_min", median(drt.to_vec())),
        ("median_abs_log2_area", median(log2.to_vec())),
    ] {
        o.raw(",\"").raw(k).raw("\":").opt(v.map(|m| round(m, 4)));
    }
}

pub(super) fn start_json(wrapped: &str, kind: &str, by: &str, labels: &[&str]) -> Out {
    let mut o = Out::default();
    o.raw("{\"format\":\"comparison/1\",\"step\":").str(wrapped);
    o.raw(",\"type\":")
        .str(kind)
        .raw(",\"by\":")
        .str(by)
        .raw(",\"variants\":[");
    for (i, l) in labels.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(l);
    }
    o.raw("],\"pairs\":[");
    o
}

fn write_peaks_pair(
    o: &mut Out,
    order: &[String],
    a: &HashMap<String, Feats>,
    b: &HashMap<String, Feats>,
    tol: Tol,
) {
    let empty = Feats {
        mz: Vec::new(),
        rt: Vec::new(),
        area: Vec::new(),
        unit: false,
    };
    let mean = |v: &[f64]| (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64);
    let (mut na, mut nb, mut both) = (0, 0, 0);
    let (mut ovs, mut rcs, mut dmz, mut drt, mut log2, mut pf) = (
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Out::default(),
    );
    for key in order {
        let (fa, fb) = (a.get(key), b.get(key));
        if fa.is_none() && fb.is_none() {
            continue;
        }
        let s = stats_of(fa.unwrap_or(&empty), fb.unwrap_or(&empty), tol);
        (na, nb, both) = (na + s.na, nb + s.nb, both + s.both);
        ovs.extend(s.ov);
        rcs.extend(s.rc);
        dmz.extend_from_slice(&s.dmz);
        drt.extend_from_slice(&s.drt);
        log2.extend_from_slice(&s.log2);
        pf.raw(if pf.0.is_empty() { "" } else { "," })
            .raw("{\"file\":")
            .str(key);
        write_stats(&mut pf, "overlap", "rank_corr", &s, &s.dmz, &s.drt, &s.log2);
        pf.raw("}");
    }
    let agg = Stats {
        na,
        nb,
        both,
        ov: mean(&ovs),
        rc: mean(&rcs),
        dmz: Vec::new(),
        drt: Vec::new(),
        log2: Vec::new(),
    };
    write_stats(o, "mean_overlap", "mean_rank_corr", &agg, &dmz, &drt, &log2);
    o.raw(",\"per_file\":[").raw(&pf.0).raw("]}");
}

pub(super) fn compare_feats(
    wrapped: &str,
    labels: &[&str],
    kind: Kind,
    groups: &[&[(&str, &[u8])]],
    tol: Tol,
) -> Result<String> {
    let (tname, by) = if kind == Kind::Table {
        ("table", "feature")
    } else {
        ("peaks", "file")
    };
    let mut o = start_json(wrapped, tname, by, labels);
    let mut first = true;
    if kind == Kind::Table {
        let tabs = groups
            .iter()
            .map(|g| load_table(g[0].1))
            .collect::<Result<Vec<_>>>()?;
        for i in 0..tabs.len() {
            for j in i + 1..tabs.len() {
                o.raw(if first { "" } else { "," });
                first = false;
                let s = stats_of(&tabs[i], &tabs[j], tol);
                o.raw("{\"a\":")
                    .str(labels[i])
                    .raw(",\"b\":")
                    .str(labels[j]);
                write_stats(
                    &mut o,
                    "mean_overlap",
                    "mean_rank_corr",
                    &s,
                    &s.dmz,
                    &s.drt,
                    &s.log2,
                );
                o.raw("}");
            }
        }
    } else {
        let (mut vars, mut seen, mut order) =
            (Vec::with_capacity(groups.len()), HashSet::new(), Vec::new());
        for g in groups {
            let mut map = HashMap::new();
            for (k, (name, bytes)) in g.iter().enumerate() {
                let (key, f) = load_peaks(name, bytes, k)?;
                if seen.insert(key.clone()) {
                    order.push(key.clone());
                }
                map.entry(key).or_insert(f);
            }
            vars.push(map);
        }
        for i in 0..vars.len() {
            for j in i + 1..vars.len() {
                o.raw(if first { "" } else { "," });
                first = false;
                o.raw("{\"a\":")
                    .str(labels[i])
                    .raw(",\"b\":")
                    .str(labels[j]);
                write_peaks_pair(&mut o, &order, &vars[i], &vars[j], tol);
            }
        }
    }
    o.raw("],\"keys\":[]}");
    Ok(o.0)
}

fn dir_peaks(dir: &Path) -> Result<Vec<Loaded>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(io)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("mzfeatf"))
        .collect();
    slice_by(&mut paths, |a, b| a.file_name().cmp(&b.file_name()));
    paths.into_iter().map(load).collect()
}

pub(super) fn compare_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let text_of = |k: &str| {
        job.resolved
            .params
            .iter()
            .find(|(n, _)| n == k)
            .and_then(|(_, v)| v.str())
            .unwrap_or("")
            .to_string()
    };
    let num_of = |k: &str, d: f64| {
        job.resolved
            .params
            .iter()
            .find(|(n, _)| n == k)
            .and_then(|(_, v)| v.f64())
            .unwrap_or(d)
    };
    let (step, labels_str) = (text_of("step"), text_of("labels"));
    let labels: Vec<&str> = labels_str.split(',').collect();
    let tol = Tol {
        ppm: num_of("mz_tolerance_ppm", 5.0),
        da: num_of("mz_tolerance_da", 0.3),
        rt: num_of("rt_tolerance_min", 0.2),
    };
    let paths = all_inputs(r, job, "variants");
    let groups: Vec<Vec<Loaded>> = if paths.iter().any(|p| p.is_dir()) {
        paths
            .into_iter()
            .map(|p| {
                if p.is_dir() {
                    dir_peaks(&p)
                } else {
                    Ok(vec![load(p)?])
                }
            })
            .collect::<Result<_>>()?
    } else {
        let files = read_all(r, job, "variants")?;
        if files.is_empty() || labels.is_empty() || !files.len().is_multiple_of(labels.len()) {
            return Err(Error::new("err.flow.compare.variants"));
        }
        files
            .chunks(files.len() / labels.len())
            .map(|c| c.to_vec())
            .collect()
    };
    let named: Vec<Vec<(&str, &[u8])>> = groups
        .iter()
        .map(|g| g.iter().map(|(n, b)| (n.as_str(), b.as_slice())).collect())
        .collect();
    let refs: Vec<&[(&str, &[u8])]> = named.iter().map(Vec::as_slice).collect();
    let text = super::compare_groups(&step, &labels, &refs, tol)?;
    write_made(r, job, text, DataType::Comparison)
}
