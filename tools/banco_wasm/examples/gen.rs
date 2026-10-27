//! Synthetic Orbitrap-like profile mzML: gen OUT MS1_SCANS PEAKS_PER_SCAN (every 4th scan is followed by 3 MS2 centroid scans).
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Opzioni};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = &a[1];
    let n1: usize = a[2].parse().unwrap();
    let np: usize = a[3].parse().unwrap();
    let mut seed = 7u64;
    let mut rnd = move || { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (seed >> 11) as f64 / (1u64 << 53) as f64 };
    // fixed feature list: (mz, rt apex, width, height)
    let feats: Vec<(f64, f64, f64, f64)> = (0..np).map(|_| (100.0 + 900.0 * rnd(), 20.0 * rnd(), 0.05 + 0.2 * rnd(), 1e4 + 1e7 * rnd() * rnd())).collect();
    let mut sp = Vec::new();
    let mut idx = 0;
    for i in 0..n1 {
        let rt = 20.0 * i as f64 / n1 as f64;
        let mut pk: Vec<(f64, f64)> = feats.iter().map(|&(m, r, w, h)| (m, h * (-(rt - r) * (rt - r) / (2.0 * w * w)).exp() + 500.0 * rnd())).collect();
        pk.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let (mut mz, mut it) = (Vec::new(), Vec::new());
        for &(m, h) in &pk {
            let s = m / 140000.0 / 2.355 * 2.0; // FWHM at R=70k
            for k in -7i32..=7 { let x = m + k as f64 * s * 0.5; mz.push(x); it.push((h * (-(k as f64 * 0.5).powi(2) / 2.0).exp()) as f32); }
        }
        sp.push(Spectrum { index: idx, id: format!("scan={}", idx + 1), level: 1, rt, polarity: 1, profile: true, mz, intensity: it, ..Default::default() });
        idx += 1;
        if i % 4 == 0 {
            for _ in 0..3 {
                let prec = 150.0 + 700.0 * rnd();
                let mut m: Vec<f64> = (0..80).map(|_| 50.0 + (prec - 50.0) * rnd()).collect();
                m.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let it: Vec<f32> = m.iter().map(|_| (1e5 * rnd() * rnd()) as f32).collect();
                sp.push(Spectrum { index: idx, id: format!("scan={}", idx + 1), level: 2, rt, polarity: 1, profile: false, precursor: Some(prec), mz: m, intensity: it, ..Default::default() });
                idx += 1;
            }
        }
    }
    let mut w = MzmlWriter::con_opzioni(Vec::new(), &Header { run: "x".into(), ..Default::default() }, Opzioni { livello: 1, numpress: false, ..Opzioni::default() }).unwrap();
    w.open_spectra(sp.len()).unwrap();
    for s in &sp { w.spectrum(s).unwrap(); }
    w.close_spectra().unwrap();
    std::fs::write(out, w.finish().unwrap()).unwrap();
}
