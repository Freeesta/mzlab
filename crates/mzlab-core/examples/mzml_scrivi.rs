//! Writer benchmark: `mzml_scrivi [SCANS] [PEAKS]` writes synthetic profile-like spectra at each zlib level (and with numpress) and prints size and time.
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Opzioni};
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let scans: usize = a.get(1).and_then(|x| x.parse().ok()).unwrap_or(2000);
    let peaks: usize = a.get(2).and_then(|x| x.parse().ok()).unwrap_or(5000);
    let mut seed = 12345u64;
    let mut rnd = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as f64 / (1u64 << 31) as f64
    };
    let spectra: Vec<Spectrum> = (0..scans)
        .map(|i| {
            let mut mz = 100.0;
            let mut mzs = Vec::with_capacity(peaks);
            let mut it = Vec::with_capacity(peaks);
            for _ in 0..peaks {
                mz += 0.01 + rnd() * 0.3;
                mzs.push(mz);
                it.push((rnd() * rnd() * 1e5) as f32);
            }
            Spectrum {
                index: i,
                id: format!("scan={}", i + 1),
                level: 1,
                rt: i as f64 * 0.01,
                polarity: 1,
                profile: false,
                mz: mzs,
                intensity: it,
                ..Default::default()
            }
        })
        .collect();
    for (nome, opz) in [
        (
            "zlib1",
            Opzioni {
                livello: 1,
                numpress: false,
            },
        ),
        (
            "zlib3",
            Opzioni {
                livello: 3,
                numpress: false,
            },
        ),
        (
            "zlib6",
            Opzioni {
                livello: 6,
                numpress: false,
            },
        ),
        (
            "numpress",
            Opzioni {
                livello: 1,
                numpress: true,
            },
        ),
    ] {
        let t = Instant::now();
        let mut w = MzmlWriter::con_opzioni(
            Vec::new(),
            &Header {
                run: "x".into(),
                ..Default::default()
            },
            opz,
        )
        .unwrap();
        w.open_spectra(spectra.len()).unwrap();
        for s in &spectra {
            w.spectrum(s).unwrap();
        }
        w.close_spectra().unwrap();
        let n = w.finish().unwrap().len();
        println!(
            "{nome:9} {:8.1} MB {:6.2} s",
            n as f64 / 1e6,
            t.elapsed().as_secs_f64()
        );
    }
}
