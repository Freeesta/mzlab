//! `/api/spectrum` on a profile level whose scans are mostly a few points (a converter that leaves the zero bins out, as the `.wiff` reader does):
//! the spacing of the points is read from the scans that have enough of them, wherever they are (else from the shortest gap of all the scans), so
//! the route answers instead of giving the level to Python (the page has no Python: the spectrum panel stayed blank). Synthetic data only.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::routes::{Query, Routes};
use mzlab_core::upload::prepare;

const STEP: f64 = 0.06;

fn scan(i: usize, prec: f64, dense: bool) -> Spectrum {
    let n = if dense { 30 } else { 1 + i % 4 };
    let first = 100 + (i % 7);
    let mz: Vec<f64> = (0..n).map(|b| 50.0 + (first + b) as f64 * STEP).collect();
    let intensity: Vec<f32> = (0..n).map(|b| 10.0 + (i + b) as f32).collect();
    Spectrum {
        index: i,
        id: format!("controllerType=0 controllerNumber=1 scan={}", i + 1),
        level: 2,
        rt: 0.05 * i as f64,
        polarity: 1,
        profile: true,
        filter: String::new(),
        precursor: Some(prec),
        mz,
        intensity,
        wide: None,
    }
}

/// Two precursors alternating; the only scans with more than 20 points are the first `dense` ones, far from the middle of the run.
fn routes(dense: usize) -> Routes<Vec<u8>> {
    let mut w = MzmlWriter::new(Vec::new(), &Header { run: "sparse".into(), ..Default::default() }).unwrap();
    let total = 100;
    w.open_spectra(total).unwrap();
    for i in 0..total {
        w.spectrum(&scan(i, if i % 5 == 1 { 305.0 } else { 229.1 }, i < dense)).unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, "sparse.mzML").unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap();
    let len = idx.len() as u64;
    Routes::new(Index::open(idx, len).unwrap())
}

#[test]
fn sparse_profile_level_still_answers_the_spectrum() {
    // with a few dense scans, and with none (the product-ion and EPI scans of a 3200 QTRAP keep a handful of points each)
    for dense in [6, 0] {
        answers(routes(dense));
    }
}

fn answers(mut r: Routes<Vec<u8>>) {
    for (prec, from, to) in [("229.1", 1.0, 4.0), ("305", 1.0, 4.0), ("229.1", 0.0, 5.0)] {
        let q = format!("k=0&rt0={from}&rt1={to}&level=2&precursor={prec}");
        let got = r.handle("/api/spectrum", &Query::parse(&q)).unwrap();
        let text = got.unwrap_or_else(|| panic!("{q}: gave the spectrum to Python"));
        let j: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(!j["mz"].as_array().unwrap().is_empty(), "{q}: empty spectrum");
        assert!(j["scans"].as_i64().unwrap() > 0, "{q}: no scans");
    }
}

struct CountingReader {
    data: Vec<u8>,
    bytes_read: std::rc::Rc<std::cell::Cell<usize>>,
}

impl mzlab_core::idxread::ReadAt for CountingReader {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        self.bytes_read.set(self.bytes_read.get() + buf.len());
        self.data.read_at(offset, buf)
    }
}

fn old_scan_points<R: mzlab_core::idxread::ReadAt>(
    idx: &mut Index<R>,
    si: usize,
    positions: &[usize],
) -> Vec<(Vec<f64>, Vec<f64>)> {
    let mut out = vec![(Vec::new(), Vec::new()); positions.len()];
    let at: std::collections::HashMap<usize, usize> =
        positions.iter().enumerate().map(|(k, &p)| (p, k)).collect();
    let n = idx.sections[si].n_peaks;
    idx.each_peak(si, 0, n, |p, mz, y| {
        if let Some(&k) = at.get(&p) {
            out[k].0.push(mz);
            out[k].1.push(y);
        }
    })
    .unwrap();
    out
}

fn old_point_step<R: mzlab_core::idxread::ReadAt>(idx: &mut Index<R>, si: usize) -> Option<f64> {
    let n_scans = idx.arrays(si).unwrap().scan_ids.len();
    let mid = n_scans / 2;
    let pos: Vec<usize> = (mid..n_scans.min(mid + 20)).collect();
    let min_gap = |mz: &[f64]| {
        mz.windows(2)
            .map(|w| w[1] - w[0])
            .filter(|d| *d > 1e-4)
            .fold(f64::INFINITY, f64::min)
    };
    let quant = |g: f64| {
        g.is_finite()
            .then(|| (g * 1e4).round_ties_even() / 1e4)
            .filter(|s| *s > 0.0)
    };
    for pts in old_scan_points(idx, si, &pos) {
        if pts.0.len() > 20 {
            return quant(min_gap(&pts.0));
        }
    }
    let all: Vec<usize> = (0..n_scans).collect();
    let (mut gap, mut sparse, mut used) = (f64::INFINITY, f64::INFINITY, 0);
    for pts in old_scan_points(idx, si, &all) {
        let g = min_gap(&pts.0);
        if pts.0.len() > 20 && used < 20 {
            used += 1;
            gap = gap.min(g);
        }
        sparse = sparse.min(g);
    }
    quant(if used > 0 { gap } else { sparse })
}

/// Multi-block synthetic profile mzML + mzidx where early scans sit in low m/z (100..250 Da) and late scans sit in high m/z (600..750 Da).
fn multiblock_profile_bytes(n_scans: usize, pts_per_scan: usize, sparse_mid: bool) -> (Vec<u8>, Vec<u8>) {
    let mut w = MzmlWriter::new(Vec::new(), &Header { run: "mb".into(), ..Default::default() }).unwrap();
    w.open_spectra(n_scans).unwrap();
    let mid = n_scans / 2;
    for i in 0..n_scans {
        let is_mid = sparse_mid && (mid..mid + 20).contains(&i);
        let np = if is_mid { 4 } else { pts_per_scan };
        let base_mz = if i < mid { 100.0 + (i % 10) as f64 * 0.5 } else { 600.0 + (i % 10) as f64 * 0.5 };
        let mz: Vec<f64> = (0..np).map(|b| base_mz + b as f64 * STEP).collect();
        let intensity: Vec<f32> = (0..np).map(|b| 50.0 + ((i + b) % 97) as f32).collect();
        let sp = Spectrum {
            index: i,
            id: format!("controllerType=0 controllerNumber=1 scan={}", i + 1),
            level: 1,
            rt: 0.02 * i as f64,
            polarity: 1,
            profile: true,
            filter: "ITMS + p ESI Full ms [100.00-800.00]".into(),
            precursor: None,
            mz,
            intensity,
            wide: None,
        };
        w.spectrum(&sp).unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, "mb.mzML").unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap();
    (mzml, idx)
}

#[test]
fn dense_lut_and_block_filter_match_old_code_and_skip_blocks() {
    let (_, idx_bytes) = multiblock_profile_bytes(120, 600, true);
    let len = idx_bytes.len() as u64;
    let mut ref_idx = Index::open(idx_bytes.clone(), len).unwrap();
    let counter = std::rc::Rc::new(std::cell::Cell::new(0usize));
    let mut new_idx = Index::open(
        CountingReader { data: idx_bytes.clone(), bytes_read: counter.clone() },
        len,
    )
    .unwrap();
    assert_eq!(old_point_step(&mut ref_idx, 0), Some(STEP));
    for positions in [vec![0, 1, 2], vec![59, 60, 61], vec![110, 115, 119], (0..25).collect()] {
        let want = old_scan_points(&mut ref_idx, 0, &positions);
        let n_scans = new_idx.sections[0].n_scans;
        let mut lut = vec![u32::MAX; n_scans];
        for (k, &p) in positions.iter().enumerate() {
            lut[p] = k as u32;
        }
        let mut got = vec![(Vec::new(), Vec::new()); positions.len()];
        counter.set(0);
        new_idx
            .each_peak_for(0, &lut, |k, mz, y| {
                got[k].0.push(mz);
                got[k].1.push(y);
            })
            .unwrap();
        assert_eq!(got, want, "positions={positions:?}");
        // Low-m/z scans and high-m/z scans live in separate peak blocks,
        // so the block filter reads strictly less than the full 20 bytes/peak of the section.
        let full_bytes = new_idx.sections[0].n_peaks as usize * 20;
        assert!(
            counter.get() < full_bytes * 3 / 4,
            "block filter did not skip non-matching blocks: read {} of {} bytes",
            counter.get(),
            full_bytes
        );
    }
}

#[test]
fn point_step_and_routes_stay_under_small_device_memory_and_match_across_targets() {
    // 120 scans * ~600 points = ~70K peaks across multiple 16K blocks, with sparse middle scans.
    let (mzml, idx_bytes) = multiblock_profile_bytes(120, 600, true);
    let len = idx_bytes.len() as u64;
    let dir = std::env::temp_dir().join(format!("mzlab-prof-par-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (mzml_path, idx_path) = (dir.join("mb.mzML"), dir.join("mb.mzidx"));
    std::fs::write(&mzml_path, &mzml).unwrap();
    std::fs::write(&idx_path, &idx_bytes).unwrap();
    let counter = std::rc::Rc::new(std::cell::Cell::new(0usize));
    let mut r_mem = Routes::new(
        Index::open(
            CountingReader { data: idx_bytes.clone(), bytes_read: counter.clone() },
            len,
        )
        .unwrap(),
    );
    let mut r_file = Routes::new(Index::open(std::fs::File::open(&idx_path).unwrap(), len).unwrap());
    let mut r_mzml = Routes::new(Index::open(std::fs::File::open(&idx_path).unwrap(), len).unwrap());
    let mut mzml_file = std::fs::File::open(&mzml_path).unwrap();
    for (path, q) in [
        ("/api/spectrum", "k=0&rt0=0.0&rt1=0.3&level=1"),
        ("/api/spectrum", "k=0&rt0=1.1&rt1=1.4&level=1"),
        ("/api/spectrum", "k=0&rt0=2.0&rt1=2.35&level=1"),
        ("/api/spectra", "k=0&level=1&i0=0&i1=9"),
        ("/api/spectra", "k=0&level=1&i0=55&i1=65"),
        ("/api/spectra", "k=0&level=1&i0=105&i1=119"),
    ] {
        let query = Query::parse(q);
        let a = r_mem.handle(path, &query).unwrap().expect("site route");
        let b = r_file.handle(path, &query).unwrap().expect("cli route");
        let c = r_mzml
            .mzml_route(path, &query, &mut mzml_file)
            .unwrap()
            .or_else(|| r_mzml.handle(path, &query).unwrap())
            .expect("bridge route");
        assert_eq!(a, b, "site vs cli mismatch on {path}?{q}");
        assert_eq!(a, c, "site vs bridge mismatch on {path}?{q}");
    }
    // Even after answering 3 spectrum windows and 3 spectra ranges (including sparse-middle point_step),
    // total I/O stays well under the 32 MB small-device limit (and 64 MB worker limit).
    assert!(
        counter.get() < (16 << 20),
        "excessive bytes read: {}",
        counter.get()
    );
    let _ = std::fs::remove_dir_all(&dir);
}


