//! A synthetic unit-resolution LC-MS run of known truth for the tests of `ticbg` in the CLI and the app (no lab data): six background lines
//! (141, 149, 158, 198, 214, 236) with a 60 % drift and 5 % noise, eight random noise ions per scan, and a peak at 14.3 min with ions at
//! 198 and 158 (on two lines) and 305. `blank` = the same lines, no peak. Arithmetic only (no `ln`/`exp` of the platform) for the same bytes everywhere.
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};

const LINES: [(f64, f64); 6] = [
    (141.0, 30e3),
    (149.0, 80e3),
    (158.0, 25e3),
    (198.0, 40e3),
    (214.0, 20e3),
    (236.0, 50e3),
];
const PEAK: [(f64, f64); 3] = [(198.0, 50e3), (158.0, 30e3), (305.0, 40e3)];

/// The mzML of `n` scans over 20 min.
pub fn mzml(n: usize, seed: u64, blank: bool) -> Vec<u8> {
    let h = Header {
        run: "background".into(),
        sources: vec!["background.mzML".into()],
        software: ("mzLab".into(), "0".into()),
        ..Default::default()
    };
    let mut w = MzmlWriter::new(Vec::new(), &h).expect("memory");
    let mut s = seed;
    let mut u = || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((s >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    };
    w.open_spectra(n).expect("memory");
    for j in 0..n {
        let t = 20.0 * j as f64 / n as f64;
        let mut pts: Vec<(f64, f64)> = Vec::new();
        for (mz, h) in LINES {
            let g = (u() + u() + u() + u() - 2.0) * 1.732;
            pts.push((
                mz + 0.1 * (u() - 0.5),
                h * (1.0 + 0.6 * t / 20.0) * (1.0 + 0.05 * g),
            ));
        }
        for _ in 0..8 {
            pts.push((
                (100.0 + 300.0 * u()).round() + 0.2 * (u() - 0.5),
                500.0 * u(),
            ));
        }
        // a peak of sigma 0.08 min as a raised-cosine bump of the same area (no exp)
        let d = (t - 14.3) / 0.08;
        if !blank && d.abs() < 2.5 {
            let g = 0.5 * (1.0 + libm::cos(std::f64::consts::PI * d / 2.5)) * 1.0026;
            for (mz, h) in PEAK {
                pts.push((mz + 0.1 * (u() - 0.5), h * g));
            }
        }
        crate::sortc::slice_by(&mut pts, |a, b| a.0.total_cmp(&b.0));
        let sp = Spectrum {
            index: j,
            id: format!("scan={}", j + 1),
            level: 1,
            rt: t,
            polarity: 1,
            mz: pts.iter().map(|p| p.0).collect(),
            intensity: pts.iter().map(|p| p.1 as f32).collect(),
            ..Default::default()
        };
        w.spectrum(&sp).expect("memory");
    }
    w.close_spectra().expect("memory");
    w.finish().expect("memory")
}
