//! Bench harness: real mzlab-core functions behind tiny wasm-bindgen entry points; JS times each call.
use mzlab_core::{centroidi, composition, entropy, idxread, mzidx, mzml, peaks, routes, upload};
use std::cell::RefCell;
use std::io::Cursor;
use wasm_bindgen::prelude::*;

thread_local! {
    static FILE: RefCell<Vec<u8>> = RefCell::new(Vec::new());
    static RUN: RefCell<Option<mzml::Run>> = RefCell::new(None);
    static IDX: RefCell<Vec<u8>> = RefCell::new(Vec::new());
    static Z: RefCell<(Vec<u8>, usize)> = RefCell::new((Vec::new(), 0));
}

#[wasm_bindgen]
pub fn load(b: &[u8]) { FILE.with(|f| *f.borrow_mut() = b.to_vec()); }

/// XML + base64 + zlib of every spectrum.
#[wasm_bindgen]
pub fn parse() -> f64 {
    let run = FILE.with(|f| mzml::read(Cursor::new(f.borrow().as_slice())).unwrap());
    let n: usize = run.spectra.iter().map(|s| s.mz.len()).sum();
    RUN.with(|r| *r.borrow_mut() = Some(run));
    n as f64
}

/// Scan table + .mzidx in memory (the index pool's job for one file).
#[wasm_bindgen]
pub fn index() -> f64 {
    FILE.with(|f| {
        let mut v = f.borrow().clone();
        let size = v.len() as u64;
        let p = upload::prepare(&mut v, size, "x.mzML").unwrap();
        let (out, _) = mzidx::build_mzml_to_vec(&p.table, &f.borrow(), 256 << 20).unwrap();
        let n = out.len();
        IDX.with(|i| *i.borrow_mut() = out);
        n as f64
    })
}

/// `k` XICs from the index, m/z spread over the range.
#[wasm_bindgen]
pub fn xic(k: u32, lo: f64, hi: f64, tol: f64) -> f64 {
    IDX.with(|i| {
        let v = i.borrow().clone();
        let size = v.len() as u64;
        let mut r = routes::Routes::new(idxread::Index::open(v, size).unwrap());
        let mut s = 0.0;
        for j in 0..k {
            let mz = lo + (hi - lo) * (j as f64 + 0.5) / k as f64;
            let (_, y) = r.xic_raw(mz, tol, 1).unwrap();
            s += y.iter().sum::<f64>();
        }
        s
    })
}

/// Centroids of every MS1 spectrum (continuous mode).
#[wasm_bindgen]
pub fn centroid(step: f64, w: f64, maxn: u32) -> f64 {
    let p = centroidi::Params { mode: centroidi::Mode::Continuous, unit: 1.0, step, w, k: 3.0, vmax: 0.0, area: false };
    let mut buf = centroidi::Buf::default();
    RUN.with(|r| {
        let r = r.borrow();
        let mut n = 0usize;
        for s in r.as_ref().unwrap().spectra.iter().filter(|s| s.level == 1).take(maxn as usize) {
            let y: Vec<f64> = s.intensity.iter().map(|&x| x as f64).collect();
            n += centroidi::centroids(&s.mz, &y, &p, &mut buf).len();
        }
        n as f64
    })
}

/// Peak picking (AsLS baseline) on `k` XICs taken from the decoded run.
#[wasm_bindgen]
pub fn pick(k: u32, lo: f64, hi: f64, tol: f64) -> f64 {
    RUN.with(|r| {
        let r = r.borrow();
        let sp: Vec<&mzml::Spectrum> = r.as_ref().unwrap().spectra.iter().filter(|s| s.level == 1).collect();
        let rt: Vec<f64> = sp.iter().map(|s| s.rt).collect();
        let mut n = 0;
        for j in 0..k {
            let mz = lo + (hi - lo) * (j as f64 + 0.5) / k as f64;
            let y: Vec<f64> = sp.iter().map(|s| {
                let a = s.mz.partition_point(|&m| m < mz - tol);
                let b = s.mz.partition_point(|&m| m <= mz + tol);
                s.intensity[a..b].iter().map(|&x| x as f64).sum()
            }).collect();
            n += peaks::detect_peaks(&rt, &y, peaks::Params::default()).len();
        }
        n as f64
    })
}

/// Elemental composition of `k` masses (CHNOPS default, 5 ppm).
#[wasm_bindgen]
pub fn compose(k: u32, max_results: u32) -> f64 {
    let o = composition::Opts { max_results: max_results as usize, ..Default::default() };
    let mut n = 0;
    for j in 0..k {
        let mz = 150.0 + 450.0 * j as f64 / k as f64 + 0.0123;
        n += composition::compose(mz, &o).map(|s| s.len()).unwrap_or(0);
    }
    n as f64
}

/// Spectral entropy similarity: each MS2 spectrum against every other one (library-style matching).
#[wasm_bindgen]
pub fn entropy(maxn: u32) -> f64 {
    RUN.with(|r| {
        let r = r.borrow();
        let sp: Vec<(Vec<f64>, Vec<f64>)> = r.as_ref().unwrap().spectra.iter().filter(|s| s.level == 2 && !s.mz.is_empty())
            .take(maxn as usize).map(|s| (s.mz.clone(), s.intensity.iter().map(|&x| x as f64).collect())).collect();
        let mut t = 0.0;
        for a in &sp { for b in &sp { t += entropy::entropy_similarity(&a.0, &a.1, &b.0, &b.1, 0.02); } }
        t
    })
}

/// zlib inflate of a buffer made by `deflate_setup` (flate2: miniz_oxide, or zlib-rs with the feature).
#[wasm_bindgen]
pub fn deflate_setup(mb: u32) -> f64 {
    use std::io::Write;
    let n = (mb as usize) << 20;
    let mut raw = Vec::with_capacity(n);
    let mut x = 100.0f64; let mut seed = 1u64;
    while raw.len() < n {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        x += 0.001 + (seed >> 40) as f64 * 1e-9;
        raw.extend_from_slice(&x.to_le_bytes());
        raw.extend_from_slice(&(((seed >> 33) % 1000) as f32).to_le_bytes());
    }
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(1));
    e.write_all(&raw).unwrap();
    let z = e.finish().unwrap();
    let l = z.len();
    Z.with(|c| *c.borrow_mut() = (z, raw.len()));
    l as f64
}

#[wasm_bindgen]
pub fn inflate() -> f64 {
    use std::io::Read;
    Z.with(|c| {
        let c = c.borrow();
        let mut out = Vec::with_capacity(c.1);
        flate2::read::ZlibDecoder::new(c.0.as_slice()).read_to_end(&mut out).unwrap();
        out.len() as f64
    })
}

/// Peaks decoded from a Thermo `.raw` by the `raw` reader (bytes given to `load`).
#[cfg(feature = "thermo")]
#[wasm_bindgen]
pub fn raw_parse() -> f64 {
    let mut n = 0usize;
    FILE.with(|f| mzlab_core::raw::parse(Cursor::new(f.borrow().as_slice()), |s| n += s.mz.len()).unwrap());
    n as f64
}

/// `.mzidx` of the `.raw` straight from the reader (`prepare_lettore` + `build_lettore`, the way the site indexes a vendor file).
#[cfg(feature = "thermo")]
#[wasm_bindgen]
pub fn raw_index() -> f64 {
    use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
    let bytes = FILE.with(|f| f.borrow().clone());
    let fs = FileSet { membri: vec![Membro { nome: "x.raw".into(), dim: bytes.len() as u64, dati: Box::new(bytes), sha1: None }] };
    let mut l = lettori::apri(fs, &Limiti::sito()).unwrap();
    let p = upload::prepare_lettore(&mut *l, 0, &[], "x.mzML").unwrap();
    let mut out = Cursor::new(Vec::new());
    mzidx::build_lettore(&p.table, &mut *l, 0, &mut out, 192 << 20).unwrap();
    out.into_inner().len() as f64
}
