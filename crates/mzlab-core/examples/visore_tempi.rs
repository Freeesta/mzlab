//! Native timings of the viewer gestures on an `.mzidx`: opening, XIC (arrays and JSON), one spectrum, with the size of every answer.
//! `cargo run --release -p mzlab-core --example visore_tempi -- genera DIR [SCANS] [PEAKS]` writes a synthetic mzML and its `.mzidx` in DIR (timed);
//! `cargo run --release -p mzlab-core --example visore_tempi -- indicizza FILE.mzML OUT.mzidx` indexes any mzML the way the desktop does (timed);
//! `cargo run --release -p mzlab-core --example visore_tempi -- FILE.mzidx [N] [--json]` times N m/z (default 200) spread over the base peaks of the file.
//! Used by `tools/bench_visore.py` and `docs/agenti/visore-veloce.md`.
#[path = "../tests/lettore_finto/mod.rs"]
mod finto;

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_core::routes::{Query, Routes};
use mzlab_core::{lettori_mzml, mzidx, upload};

/// Median and 95th percentile of a list of samples (sorted in place).
fn stat(v: &mut [f64]) -> (f64, f64) {
    mzlab_core::sortc::slice_by(v, |a, b| a.partial_cmp(b).unwrap());
    let at = |q: f64| v[((v.len() as f64 * q) as usize).min(v.len() - 1)];
    (at(0.5), at(0.95))
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a.first().map(String::as_str) {
        Some("genera") => genera(&a),
        Some("indicizza") => indicizza(
            std::path::Path::new(a.get(1).expect("usage: indicizza FILE.mzML OUT.mzidx")),
            std::path::Path::new(a.get(2).expect("usage: indicizza FILE.mzML OUT.mzidx")),
        ),
        Some(p) => misura(p, &a[1..]),
        None => panic!("usage: visore_tempi genera DIR [SCANS] [PEAKS] | FILE.mzidx [N] [--json]"),
    }
}

/// `genera DIR [SCANS] [PEAKS]`: the mzML of `mzml_tempi genera` and its `.mzidx` (with a spool, as the desktop builds it), every step timed.
fn genera(a: &[String]) {
    let dir = std::path::Path::new(a.get(1).expect("usage: genera DIR [SCANS] [PEAKS]"));
    std::fs::create_dir_all(dir).expect("dir");
    let n: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(8000);
    let peaks: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(4000);
    let (mzml, idx) = (dir.join("visore.mzML"), dir.join("visore.mzidx"));
    let mut l = finto::Finto::vario(n, peaks, 12345);
    let t = Instant::now();
    lettori_mzml::scrivi(
        &mut l,
        0,
        BufWriter::new(File::create(&mzml).expect("create")),
        &mut |_, _| {},
    )
    .expect("write");
    let size = std::fs::metadata(&mzml).expect("size").len();
    println!(
        "mzml_write_s={:.1} mzml_bytes={size} scans={n} max_peaks_per_scan={peaks}",
        t.elapsed().as_secs_f64()
    );
    drop(l);
    indicizza(&mzml, &idx);
}

/// The scan table and the `.mzidx` of an mzML with a spool next to it (the way the desktop builds it), every step timed.
fn indicizza(mzml: &std::path::Path, idx: &std::path::Path) {
    let size = std::fs::metadata(mzml).expect("size").len();
    let t = Instant::now();
    let p = upload::prepare(&mut File::open(mzml).expect("open"), size, "visore.mzML")
        .expect("prepare");
    println!("prepare_s={:.1}", t.elapsed().as_secs_f64());
    let spool_path = idx.with_extension("spool");
    let mut spool = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&spool_path)
        .expect("spool");
    let mut out = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(idx)
        .expect("idx");
    let t = Instant::now();
    let st = mzidx::build_mzml_spill(
        &p.table,
        BufReader::new(File::open(mzml).expect("open")),
        &mut out,
        192 << 20,
        Some(&mut spool as &mut dyn mzlab_core::mzidx_spill::Spill),
    )
    .expect("index");
    println!(
        "mzidx_build_s={:.1} mzidx_bytes={} passes={}",
        t.elapsed().as_secs_f64(),
        st.bytes,
        st.passes
    );
    let _ = std::fs::remove_file(&spool_path);
}

/// Times `xic_raw` (arrays) and `xic` (JSON) for every m/z of a set: two passes, the first one warms the caches and is not reported.
fn xic_rows(
    r: &mut Routes<File>,
    set: &str,
    mzs: &[f64],
    name: &str,
    tol_of: fn(f64) -> f64,
) -> Vec<(String, f64, f64, f64)> {
    let n = mzs.len();
    let mut rows = Vec::new();
    for pass in 0..2 {
        let (mut raw, mut js) = (Vec::new(), Vec::new());
        let (mut raw_b, mut js_b) = (0usize, 0usize);
        for &m in mzs {
            let tol = tol_of(m);
            let t = Instant::now();
            let (rt, y) = r.xic_raw(m, tol, 1).expect("xic_raw");
            raw.push(t.elapsed().as_secs_f64() * 1e3);
            raw_b += (rt.len() + y.len()) * 8;
            let t = Instant::now();
            let s = r.xic(m, tol, 1).expect("xic");
            js.push(t.elapsed().as_secs_f64() * 1e3);
            js_b += s.len();
        }
        if pass == 0 {
            continue; // the first pass warms the page cache and the per-scan arrays
        }
        let (r50, r95) = stat(&mut raw);
        let (j50, j95) = stat(&mut js);
        rows.push((
            format!("xic_raw {name} [{set}]"),
            r50,
            r95,
            (raw_b / n) as f64,
        ));
        rows.push((
            format!("xic JSON {name} [{set}]"),
            j50,
            j95,
            (js_b / n) as f64,
        ));
    }
    rows
}

/// Times `api/spectrum` for one scan at each of the given retention times.
fn spectrum_row(r: &mut Routes<File>, rts: &[f64]) -> (String, f64, f64, f64) {
    let n = rts.len();
    let (mut sp, mut sp_b, mut none) = (Vec::new(), 0usize, 0usize);
    for &rt in rts {
        let q = Query::parse(&format!("rt0={}&rt1={}&level=1", rt - 1e-4, rt + 1e-4));
        let t = Instant::now();
        let s = r.handle("/api/spectrum", &q).expect("spectrum");
        sp.push(t.elapsed().as_secs_f64() * 1e3);
        sp_b += s.as_ref().map_or(0, String::len);
        none += s.is_none() as usize;
    }
    let (s50, s95) = stat(&mut sp);
    let spec = if none == 0 {
        "spectrum (api/spectrum, one scan)".to_string()
    } else {
        format!("spectrum (api/spectrum: {none}/{n} answered None, not Rust's)")
    };
    (spec, s50, s95, (sp_b / n) as f64)
}

/// Times the gestures on `path`: opening (5 times), then for N m/z `xic_raw` (arrays), `xic` (JSON) and for N scans one spectrum.
fn misura(path: &str, a: &[String]) {
    let n: usize = a.iter().find_map(|s| s.parse().ok()).unwrap_or(200);
    let json = a.iter().any(|s| s == "--json");
    let size = std::fs::metadata(path).expect("file").len();
    let mut open = Vec::new();
    for _ in 0..5 {
        let t = Instant::now();
        let ix = Index::open(File::open(path).expect("open"), size).expect("index");
        open.push(t.elapsed().as_secs_f64() * 1e3);
        drop(ix);
    }
    let mut r = Routes::new(Index::open(File::open(path).expect("open"), size).expect("index"));
    let si = r.idx.section_of(1, None).expect("MS1 section (1/all)");
    let (peaks, nscans) = (r.idx.sections[si].n_peaks, r.idx.sections[si].n_scans);
    // m/z spread over the quantiles of the base-peak m/z of the scans and the retention times of N scans along the run
    let ar = r.idx.arrays(si).expect("arrays").clone();
    let mut bp: Vec<f64> = ar.bp_mz.iter().copied().filter(|m| *m > 0.0).collect();
    mzlab_core::sortc::slice_by(&mut bp, |a, b| a.partial_cmp(b).unwrap());
    let mzs: Vec<f64> = (0..n)
        .map(|i| bp[i * (bp.len() - 1) / (n - 1).max(1)])
        .collect();
    let rts: Vec<f64> = (0..n)
        .map(|i| ar.rt[i * (nscans - 1) / (n - 1).max(1)])
        .collect();
    // the second set is a regular grid over the m/z range of the base peaks: the first follows the signal (and the densest m/z of a made-up file), this one does not
    let uni: Vec<f64> = (0..n)
        .map(|i| bp[0] + (bp[bp.len() - 1] - bp[0]) * (i as f64 + 0.5) / n as f64)
        .collect();
    let mut rows: Vec<(String, f64, f64, f64)> = Vec::new();
    for (set, mzs, name, tol_of) in [
        ("bp", &mzs, "LR 0.5 Da", (|_: f64| 0.5) as fn(f64) -> f64),
        ("bp", &mzs, "HR 5 ppm", |m: f64| m * 5e-6),
        ("grid", &uni, "LR 0.5 Da", |_: f64| 0.5),
        ("grid", &uni, "HR 5 ppm", |m: f64| m * 5e-6),
    ] {
        rows.extend(xic_rows(&mut r, set, mzs, name, tol_of));
    }
    rows.push(spectrum_row(&mut r, &rts));
    let (o50, o95) = stat(&mut open);
    if json {
        let r: Vec<String> = rows
            .iter()
            .map(|(k, a, b, c)| {
                format!("{{\"k\":\"{k}\",\"p50\":{a:.4},\"p95\":{b:.4},\"bytes\":{c:.0}}}")
            })
            .collect();
        println!("{{\"file_bytes\":{size},\"ms1_peaks\":{peaks},\"ms1_scans\":{nscans},\"n\":{n},\"open_p50\":{o50:.3},\"open_p95\":{o95:.3},\"rows\":[{}]}}", r.join(","));
        return;
    }
    println!(
        "{path}: {:.1} MB, MS1 section {peaks} peaks, {nscans} scans; n={n}",
        size as f64 / 1e6
    );
    println!("open (Index::open, 5x)  p50 {o50:.3} ms  p95 {o95:.3} ms");
    for (k, a, b, c) in rows {
        println!("{k:<44} p50 {a:>9.4} ms  p95 {b:>9.4} ms  {c:>10.0} B/answer");
    }
}
