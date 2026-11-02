//! The spectrum of ONE scan of a high-resolution centroid file is answered without walking the whole section: `Routes::handle("/api/spectrum")` and
//! `spectrum_exact` give byte for byte what the former implementation gave (kept here as `old_*`, built on the public index API) for every scan of
//! a synthetic file written by the test and of `HR_DDA-Exploris-t30` (`tools/dati_sintetici.py`; skipped without python3).

#![allow(clippy::disallowed_methods, clippy::too_many_arguments)]

#[path = "lettore_finto/mod.rs"]
mod finto;

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::Command;

use finto::{Finto, Scheda};
use mzlab_core::idxread::Index;
use mzlab_core::iteminfo::hr_profile;
use mzlab_core::json::{round, Out};
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::SpectrumExtra;
use mzlab_core::routes::{Query, Routes};
use mzlab_core::sorgente::Scansioni;
use mzlab_core::{lettori_mzml, mzidx, upload};

const EMPTY: &str =
    "{\"mode\":\"centroid\",\"mz\":[],\"y\":[],\"scans\":0,\"i0\":null,\"i1\":null,\"n\":0}";

/// The former `binned_cells` + `hr_window` + `binned_json` for a high-resolution file: the answer text and the number of scans of the window.
struct Old {
    mz: Vec<f64>,
    y: Vec<f64>,
    n: usize,
    sel: Vec<usize>,
    in_win: Vec<usize>,
    scan_ids: Vec<i64>,
}

fn old_window(
    r: &mut Routes<File>,
    level: u8,
    (lo, hi): (f64, f64),
    precursor: Option<f64>,
    filt: Option<&str>,
    ppm: f64,
    somma: bool,
) -> Option<Old> {
    let tol = match precursor {
        Some(p) if level > 1 && p != 0.0 => (p * ppm * 1e-6).max(1e-3),
        _ => 0.6,
    };
    let cell = (3e-6f64).ln_1p();
    let si = r.idx.section_of(level, r.pol)?;
    let ar = r.idx.arrays(si).unwrap().clone();
    let want_prec = if level > 1 { precursor } else { None };
    let keep = |j: usize, idx: &Index<File>| {
        let id = ar.scan_ids[j] as usize;
        let s = &idx.scans[id];
        filt.is_none_or(|f| s.filter == f || s.key() == f)
            && want_prec.is_none_or(|p| (s.prec(-1.0) - p).abs() <= tol)
    };
    let ok: Vec<bool> = (0..ar.rt.len())
        .map(|j| ar.rt[j] >= lo && ar.rt[j] <= hi && keep(j, &r.idx))
        .collect();
    let n = ok.iter().filter(|b| **b).count();
    let sel: Vec<usize> = (0..ar.rt.len()).filter(|&j| keep(j, &r.idx)).collect();
    let in_win: Vec<usize> = sel
        .iter()
        .enumerate()
        .filter(|&(_, &j)| ar.rt[j] >= lo && ar.rt[j] <= hi)
        .map(|(i, _)| i)
        .collect();
    let (mut mz, mut y) = (vec![], vec![]);
    let div = if somma { 1.0 } else { n as f64 };
    if n > 0 {
        let (mut c0, mut sy, mut smy) = (i64::MIN, 0.0f64, 0.0f64);
        let (mut cm, mut cy) = (vec![], vec![]);
        let total = r.idx.sections[si].n_peaks;
        r.idx
            .each_peak(si, 0, total, |pos, m, v| {
                if !ok[pos] {
                    return;
                }
                let c = (m.max(1e-9).ln() / cell).floor() as i64;
                if c != c0 {
                    if c0 != i64::MIN {
                        cm.push(smy / sy.max(1e-12));
                        cy.push(sy / div);
                    }
                    c0 = c;
                    sy = 0.0;
                    smy = 0.0;
                }
                sy += v;
                smy += v * m;
            })
            .unwrap();
        if c0 != i64::MIN {
            cm.push(smy / sy.max(1e-12));
            cy.push(sy / div);
        }
        (mz, y) = (cm, cy);
    }
    Some(Old {
        mz,
        y,
        n,
        sel,
        in_win,
        scan_ids: ar.scan_ids,
    })
}

fn old_json(m: Old, nd: usize) -> String {
    let mut o = Out::default();
    let Old {
        mz,
        y,
        n,
        sel,
        in_win,
        scan_ids,
    } = m;
    o.raw("{\"mode\":\"centroid\",\"mz\":[");
    for (i, v) in mz.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).num(round(*v, nd));
    }
    o.raw("],\"y\":[");
    for (i, v) in y.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).num(round(*v, 1));
    }
    o.raw("],\"scans\":").int(n as i64).raw(",\"i0\":");
    match in_win.first() {
        Some(i) => o.int(*i as i64),
        None => o.raw("null"),
    };
    o.raw(",\"i1\":");
    match in_win.last() {
        Some(i) => o.int(*i as i64),
        None => o.raw("null"),
    };
    o.raw(",\"n\":").int(sel.len() as i64);
    if in_win.len() == 1 {
        o.raw(",\"sid\":").int(scan_ids[sel[in_win[0]]]);
    }
    o.raw("}");
    o.0
}

/// What the file asks of the two functions: level, window, precursor, filter, and `modo=somma`.
fn params(q: &Query) -> Option<(u8, (f64, f64), Option<f64>, Option<String>, bool)> {
    let (rt0, rt1) = (q.num("rt0").unwrap()?, q.num("rt1").unwrap()?);
    Some((
        q.get("level").and_then(|v| v.parse().ok()).unwrap_or(1),
        (rt0.min(rt1), rt0.max(rt1)),
        q.num("precursor").unwrap(),
        q.some("filt").map(str::to_string),
        q.get("modo") == Some("somma"),
    ))
}

/// The former `/api/spectrum` of a high-resolution centroid file: `None` for a window of one scan.
fn old_route(r: &mut Routes<File>, q: &Query) -> Option<String> {
    let (level, win, precursor, filt, somma) = params(q)?;
    let hr = hr_profile(&r.idx.scans, level == 1, q.get("hr") != Some("0"));
    assert!(hr.on);
    match old_window(r, level, win, precursor, filt.as_deref(), hr.ppm, somma) {
        None => Some(EMPTY.into()),
        Some(m) if m.n == 1 => None,
        Some(m) => Some(old_json(m, hr.dec + 1)),
    }
}

/// The former `spectrum_exact`: the index pass first, the scan read only when that pass found a peak.
fn old_spectrum_exact(r: &mut Routes<File>, q: &Query, mzml: &mut File) -> Option<String> {
    let (level, win, precursor, filt, somma) = params(q)?;
    let hr = hr_profile(&r.idx.scans, level == 1, q.get("hr") != Some("0"));
    if q.some("bgk").is_some()
        || !hr.on
        || r.idx.scans.iter().any(|s| s.level == level && s.profile)
    {
        return None;
    }
    let mut m = old_window(r, level, win, precursor, filt.as_deref(), hr.ppm, somma)?;
    if m.n != 1 || m.in_win.len() != 1 {
        return None;
    }
    let sid = m.scan_ids[m.sel[m.in_win[0]]] as usize;
    let sc = r.idx.scans.get(sid)?;
    if !m.mz.is_empty() {
        (m.mz, m.y) = mzml.spettro(sc).unwrap();
    }
    Some(old_json(m, hr.dec + 1))
}

/// The mzML written from `schede`, its index and a `Routes` on it.
fn open(
    dir: &Path,
    name: &str,
    schede: Vec<Scheda>,
) -> (Routes<File>, File, Vec<(u8, f64, Option<f64>, String)>) {
    let mut l = Finto::da_schede(schede);
    let scans: Vec<_> = l
        .schede
        .iter()
        .map(|s| {
            (
                s.spettro.level,
                s.spettro.rt,
                s.spettro.precursor,
                s.spettro.filter.clone(),
            )
        })
        .collect();
    let mzml = dir.join(format!("{name}.mzML"));
    lettori_mzml::scrivi(
        &mut l,
        0,
        BufWriter::new(File::create(&mzml).unwrap()),
        &mut |_, _| {},
    )
    .unwrap();
    (index(dir, &mzml), File::open(&mzml).unwrap(), scans)
}

fn index(dir: &Path, mzml: &Path) -> Routes<File> {
    let size = std::fs::metadata(mzml).unwrap().len();
    let p = upload::prepare(&mut File::open(mzml).unwrap(), size, "x.mzML").unwrap();
    let idx = mzml.with_extension("mzidx");
    let mut out = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&idx)
        .unwrap();
    mzidx::build_mzml_spill(
        &p.table,
        BufReader::new(File::open(mzml).unwrap()),
        &mut out,
        64 << 20,
        None,
    )
    .unwrap();
    let _ = dir;
    Routes::new(
        Index::open(
            File::open(&idx).unwrap(),
            std::fs::metadata(&idx).unwrap().len(),
        )
        .unwrap(),
    )
}

/// Every query is answered by the two implementations; a difference lists the query.
fn same(r: &mut Routes<File>, mzml: &mut File, queries: &[String], seen: &mut [usize; 3]) {
    for text in queries {
        let q = Query::parse(text);
        let (a, b) = (old_route(r, &q), r.handle("/api/spectrum", &q).unwrap());
        assert_eq!(a, b, "route: {text}");
        let (a, b) = (
            old_spectrum_exact(r, &q, &mut mzml.try_clone().unwrap()),
            r.spectrum_exact(&q, mzml).unwrap(),
        );
        assert_eq!(a, b, "spectrum_exact: {text}");
        // the way the worker asks: the exact answer, else the route
        let via = r.mzml_route("/api/spectrum", &q, mzml).unwrap();
        assert_eq!(
            via,
            b.clone().or_else(|| old_route(r, &q)),
            "mzml_route: {text}"
        );
        seen[0] += 1;
        seen[1] += b.is_some() as usize;
        seen[2] += old_route(r, &q).is_none() as usize;
    }
}

fn queries_of(scans: &[(u8, f64, Option<f64>, String)]) -> Vec<String> {
    let mut qs = vec![];
    for (i, (level, rt, prec, filter)) in scans.iter().enumerate() {
        let (a, b) = (rt - 1e-6, rt + 1e-6);
        let p = prec.map_or(String::new(), |p| format!("&precursor={p}"));
        qs.push(format!("rt0={a}&rt1={b}&level={level}{p}"));
        qs.push(format!("rt0={b}&rt1={a}&level={level}{p}"));
        qs.push(format!("rt0={a}&rt1={b}&level={level}{p}&modo=somma"));
        qs.push(format!(
            "rt0={a}&rt1={b}&level={level}&filt={}",
            filter
                .replace(' ', "%20")
                .replace('+', "%2B")
                .replace('[', "%5B")
                .replace(']', "%5D")
        ));
        // the same scan with a precursor that matches no scan, and with a wrong filter
        qs.push(format!("rt0={a}&rt1={b}&level={level}&precursor=12345.678"));
        qs.push(format!("rt0={a}&rt1={b}&level={level}&filt=none"));
        // two scans of the level, and the whole run
        if let Some((_, rt2, _, _)) = scans.iter().skip(i + 1).find(|s| s.0 == *level) {
            qs.push(format!("rt0={a}&rt1={}&level={level}", rt2 + 1e-6));
            qs.push(format!(
                "rt0={a}&rt1={}&level={level}&modo=somma",
                rt2 + 1e-6
            ));
        }
    }
    let last = scans.last().map_or(0.0, |s| s.1);
    for level in [1, 2] {
        qs.push(format!("rt0=-5&rt1=-4&level={level}"));
        qs.push(format!("rt0=0&rt1={}&level={level}", last + 1.0));
        qs.push(format!(
            "rt0={}&rt1={}&level=3",
            last / 2.0,
            last / 2.0 + 1.0
        ));
    }
    qs
}

/// A small file in the style of `hrc`: Orbitrap centroids, a cycle of MS1 + two MS2 with a precursor, an empty MS1, an empty MS2 and a scan whose
/// peaks all have zero intensity.
fn synthetic(n: usize) -> Vec<Scheda> {
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..n)
        .map(|k| {
            let level: u8 = if k % 3 == 0 { 1 } else { 2 };
            let empty = k == 6 || k == 8;
            let np = if empty {
                0
            } else {
                5 + (next() * 40.0) as usize
            };
            let mut mz: Vec<f64> = (0..np).map(|_| 100.0 + next() * 900.0).collect();
            mz.sort_by(f64::total_cmp);
            let zero = k == 12;
            let it: Vec<f32> = (0..np)
                .map(|_| {
                    if zero {
                        0.0
                    } else {
                        (1e3 + next() * 1e6) as f32
                    }
                })
                .collect();
            let filter = if level == 1 {
                "FTMS + p ESI Full ms [100.0000-1000.0000]".to_string()
            } else {
                format!(
                    "FTMS + c ESI Full ms2 {:.4}@hcd30.00 [50.0000-500.0000]",
                    300.0 + (k % 7) as f64
                )
            };
            let precursor = (level == 2).then(|| 300.0 + (k % 7) as f64);
            let sp = Spectrum {
                index: k,
                id: format!("controllerType=0 controllerNumber=1 scan={}", k + 1),
                level,
                rt: 0.3 + k as f64 * 0.0123,
                polarity: 1,
                profile: false,
                filter,
                precursor,
                mz,
                intensity: it,
                wide: None,
            };
            Scheda {
                spettro: sp,
                extra: SpectrumExtra::default(),
            }
        })
        .collect()
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab-spettro1-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn one_scan_synthetic_file_like_before() {
    let dir = scratch("sint");
    let (mut r, mut mzml, scans) = open(&dir, "hrc", synthetic(240));
    assert!(
        hr_profile(&r.idx.scans, true, true).on,
        "the made-up file must be high resolution"
    );
    let mut seen = [0; 3];
    same(&mut r, &mut mzml, &queries_of(&scans), &mut seen);
    // one-scan windows exist (the route answered None) and the exact answer covers them
    assert!(seen[2] > 100 && seen[1] >= seen[2], "{seen:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn one_scan_dda_file_like_before() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = scratch("dda");
    let run = Command::new("python3")
        .current_dir(&root)
        .arg("tools/dati_sintetici.py")
        .arg(&dir)
        .output();
    let mzml = dir.join("HR_DDA-Exploris-t30.mzML");
    if !matches!(&run, Ok(o) if o.status.success()) || !mzml.exists() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "the generator did not run: {run:?}"
        );
        eprintln!("python3 not available: test skipped");
        return;
    }
    let mut r = index(&dir, &mzml);
    let mut file = File::open(&mzml).unwrap();
    let scans: Vec<_> = r
        .idx
        .scans
        .iter()
        .map(|s| {
            (
                s.level,
                s.rt,
                (s.level > 1).then(|| s.prec(-1.0)).filter(|p| *p >= 0.0),
                s.filter.clone(),
            )
        })
        .collect();
    let mut seen = [0; 3];
    same(&mut r, &mut file, &queries_of(&scans), &mut seen);
    assert!(seen[2] > 50, "{seen:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
