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

struct CountingAlloc;

static ALLOC_CUR: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static ALLOC_PEAK: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe impl std::alloc::GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let p = unsafe { std::alloc::System.alloc(layout) };
        if !p.is_null() {
            let cur = ALLOC_CUR.fetch_add(layout.size(), std::sync::atomic::Ordering::Relaxed) + layout.size();
            ALLOC_PEAK.fetch_max(cur, std::sync::atomic::Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        ALLOC_CUR.fetch_sub(layout.size(), std::sync::atomic::Ordering::Relaxed);
        unsafe { std::alloc::System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL_ALLOC: CountingAlloc = CountingAlloc;

fn measure_peak_heap<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let base = ALLOC_CUR.load(std::sync::atomic::Ordering::Relaxed);
    ALLOC_PEAK.store(base, std::sync::atomic::Ordering::Relaxed);
    let out = f();
    let peak = ALLOC_PEAK.load(std::sync::atomic::Ordering::Relaxed);
    (out, peak.saturating_sub(base))
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

type BitsPts = Vec<(Vec<u64>, Vec<u64>)>;

fn pts_bits(pts: &[(Vec<f64>, Vec<f64>)]) -> BitsPts {
    pts.iter()
        .map(|(m, y)| {
            (
                m.iter().copied().map(f64::to_bits).collect(),
                y.iter().copied().map(f64::to_bits).collect(),
            )
        })
        .collect()
}

fn step_bits(s: Option<f64>) -> Option<u64> {
    s.map(f64::to_bits)
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

fn build_idx_from_spectra(spectra: Vec<Spectrum>) -> (Vec<u8>, Vec<u8>) {
    let mut w = MzmlWriter::new(Vec::new(), &Header { run: "mb".into(), ..Default::default() }).unwrap();
    w.open_spectra(spectra.len()).unwrap();
    for sp in &spectra {
        w.spectrum(sp).unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, "mb.mzML").unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap();
    (mzml, idx)
}

/// Multi-block synthetic profile mzML + mzidx where early scans sit in low m/z (100..250 Da) and late scans sit in high m/z (600..750 Da).
fn multiblock_profile_bytes(n_scans: usize, pts_per_scan: usize, sparse_mid: bool) -> (Vec<u8>, Vec<u8>) {
    let mid = n_scans / 2;
    let spectra = (0..n_scans)
        .map(|i| {
            let is_mid = sparse_mid && (mid..mid + 20).contains(&i);
            let np = if is_mid { 4 } else { pts_per_scan };
            let base_mz = if i < mid { 100.0 + (i % 10) as f64 * 0.5 } else { 600.0 + (i % 10) as f64 * 0.5 };
            let mz: Vec<f64> = (0..np).map(|b| base_mz + b as f64 * STEP).collect();
            let intensity: Vec<f32> = (0..np).map(|b| 50.0 + ((i + b) % 97) as f32).collect();
            Spectrum {
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
            }
        })
        .collect();
    build_idx_from_spectra(spectra)
}

#[test]
fn dense_lut_and_block_filter_match_old_code_and_skip_blocks() {
    let (_, idx_bytes) = multiblock_profile_bytes(120, 600, true);
    let len = idx_bytes.len() as u64;
    let mut ref_idx = Index::open(idx_bytes.clone(), len).unwrap();
    let counter = std::rc::Rc::new(std::cell::Cell::new(0usize));
    let mut r_new = Routes::new(
        Index::open(
            CountingReader { data: idx_bytes.clone(), bytes_read: counter.clone() },
            len,
        )
        .unwrap(),
    );
    assert_eq!(step_bits(r_new.point_step(0).unwrap()), step_bits(old_point_step(&mut ref_idx, 0)));
    let full_bytes = r_new.idx.sections[0].n_peaks as usize * 20;
    // Empty positions and duplicate positions (last wins, matching old HashMap):
    for empty_or_dup in [vec![], vec![0, 0, 0], vec![2, 0, 2, 1, 0], vec![119, 0, 119]] {
        let want = old_scan_points(&mut ref_idx, 0, &empty_or_dup);
        let got = r_new.scan_points(0, &empty_or_dup).unwrap();
        assert_eq!(pts_bits(&got), pts_bits(&want), "positions={empty_or_dup:?}");
    }
    for positions in [vec![0, 1, 2], vec![59, 60, 61], vec![110, 115, 119], vec![2, 0, 2, 1, 0], (0..25).collect()] {
        let want = old_scan_points(&mut ref_idx, 0, &positions);
        counter.set(0);
        let got = r_new.scan_points(0, &positions).unwrap();
        let filtered_bytes = counter.get();
        assert_eq!(pts_bits(&got), pts_bits(&want), "positions={positions:?}");
        assert!(
            filtered_bytes < full_bytes * 3 / 4,
            "block filter did not skip non-matching blocks: read {filtered_bytes} of {full_bytes} bytes"
        );
        // Variant without the block filter (`old_scan_points` via `each_peak`) reads the entire section (`full_bytes`),
        // proving the `< full_bytes * 3 / 4` check is not vacuous.
        counter.set(0);
        let _ = old_scan_points(&mut r_new.idx, 0, &positions);
        assert_eq!(counter.get(), full_bytes);
    }
}

fn custom_scan(i: usize, mz: Vec<f64>, filter: &str) -> Spectrum {
    let intensity = (0..mz.len()).map(|b| 10.0 + (i + b) as f32).collect();
    Spectrum {
        index: i,
        id: format!("controllerType=0 controllerNumber=1 scan={}", i + 1),
        level: 1,
        rt: 0.05 * i as f64,
        polarity: 1,
        profile: true,
        filter: filter.into(),
        precursor: None,
        mz,
        intensity,
        wide: None,
    }
}

#[test]
fn profspec_equivalence_covers_single_scan_sparse_identical_mz_and_f4() {
    // 1. n_scans = 1 and a single scan (dense and sparse)
    for np in [25usize, 5] {
        let mz: Vec<f64> = (0..np).map(|b| 100.0 + b as f64 * 0.0423).collect();
        let (_, idx) = build_idx_from_spectra(vec![custom_scan(0, mz, "ITMS + p ESI Full ms [50.00-500.00]")]);
        let len = idx.len() as u64;
        let mut ref_ix = Index::open(idx.clone(), len).unwrap();
        let mut r = Routes::new(Index::open(idx, len).unwrap());
        assert_eq!(step_bits(r.point_step(0).unwrap()), step_bits(old_point_step(&mut ref_ix, 0)));
        for pos in [vec![], vec![0], vec![0, 0]] {
            assert_eq!(pts_bits(&r.scan_points(0, &pos).unwrap()), pts_bits(&old_scan_points(&mut ref_ix, 0, &pos)));
        }
    }
    // 2. All scans sparse (<= 20 points, forces `used == 0` fallback) and sparse middle with dense elsewhere (`used > 0`)
    for dense in [0usize, 6] {
        let mut r_new = routes(dense);
        let mut r_ref = routes(dense);
        assert_eq!(step_bits(r_new.point_step(0).unwrap()), step_bits(old_point_step(&mut r_ref.idx, 0)));
        for pos in [vec![], vec![0, 1, 2], vec![50, 51, 52], vec![5, 0, 5, 99, 0]] {
            assert_eq!(pts_bits(&r_new.scan_points(0, &pos).unwrap()), pts_bits(&old_scan_points(&mut r_ref.idx, 0, &pos)));
        }
    }
    // 3. Scans with identical m/z and gaps <= 1e-4 skipped, plus all-identical m/z returning None
    let dup_mz = vec![200.0, 200.0, 200.00005, 200.04235, 200.04235, 200.08465];
    let (_, idx_dup) = build_idx_from_spectra(vec![custom_scan(0, dup_mz, "ITMS + p ESI Full ms [100.00-500.00]")]);
    let mut ref_dup = Index::open(idx_dup.clone(), idx_dup.len() as u64).unwrap();
    let mut r_dup = Routes::new(Index::open(idx_dup.clone(), idx_dup.len() as u64).unwrap());
    assert_eq!(step_bits(r_dup.point_step(0).unwrap()), step_bits(old_point_step(&mut ref_dup, 0)));
    assert_eq!(r_dup.point_step(0).unwrap(), Some(0.0423));
    assert_eq!(pts_bits(&r_dup.scan_points(0, &[0, 0]).unwrap()), pts_bits(&old_scan_points(&mut ref_dup, 0, &[0, 0])));
    let (_, idx_flat) = build_idx_from_spectra(vec![custom_scan(0, vec![200.0, 200.0, 200.00005], "ITMS + p ESI Full ms")]);
    let mut ref_flat = Index::open(idx_flat.clone(), idx_flat.len() as u64).unwrap();
    let mut r_flat = Routes::new(Index::open(idx_flat.clone(), idx_flat.len() as u64).unwrap());
    assert_eq!(r_flat.point_step(0).unwrap(), None);
    assert_eq!(old_point_step(&mut ref_flat, 0), None);
    // 4. An f4 (single-precision HR profile) section still returns None from profile_level
    let hr_mz: Vec<f64> = (0..100).map(|b| 100.0 + b as f64 * 0.0015).collect();
    let (_, idx_f4) = build_idx_from_spectra(vec![custom_scan(0, hr_mz, "FTMS + p ESI Full ms [100.0000-500.0000]")]);
    let mut r_f4 = Routes::new(Index::open(idx_f4.clone(), idx_f4.len() as u64).unwrap());
    assert!(r_f4.idx.sections[0].f4, "expected single-precision section");
    assert_eq!(r_f4.profile_level(1).unwrap(), None);
}

#[test]
fn point_step_and_routes_stay_under_small_device_memory_and_match_across_targets() {
    // 120 scans * ~600 points = ~70K peaks across multiple 16K blocks, with sparse middle scans.
    let (mzml, idx_bytes) = multiblock_profile_bytes(120, 600, true);
    let len = idx_bytes.len() as u64;
    let mut ref_ix = Index::open(idx_bytes.clone(), len).unwrap();
    let mut step_r = Routes::new(Index::open(idx_bytes.clone(), len).unwrap());
    let _ = ref_ix.arrays(0).unwrap();
    let _ = step_r.idx.arrays(0).unwrap();
    let (s_old, old_peak_heap) = measure_peak_heap(|| old_point_step(&mut ref_ix, 0));
    let (s_new, new_peak_heap) = measure_peak_heap(|| step_r.point_step(0).unwrap());
    assert_eq!(step_bits(s_new), step_bits(s_old));
    assert!(
        new_peak_heap < (2 << 20) && new_peak_heap * 2 < old_peak_heap,
        "new_peak_heap={new_peak_heap} old_peak_heap={old_peak_heap}"
    );
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
    assert!(counter.get() < (16 << 20), "excessive bytes read: {}", counter.get());
    let _ = std::fs::remove_dir_all(&dir);
}
