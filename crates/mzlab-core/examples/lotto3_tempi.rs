//! Times what the writer and the single-scan routes do per spectrum, on synthetic centroid scans, and prints a checksum of every output so that two builds can be compared byte for byte:
//! `cargo run --release --example lotto3_tempi -p mzlab-core -- [SCANS] [PEAKS]` (writer with and without numpress, `upload::prepare`, `/api/spectra`, `/api/scanavg`).
use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Opzioni, SpectrumExtra};
use mzlab_core::routes::{Query, Routes};
use mzlab_core::{mzidx, upload};

fn spectra(scans: usize, peaks: usize, hr: bool) -> Vec<Spectrum> {
    let mut seed = 12345u64;
    let mut rnd = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as f64 / (1u64 << 31) as f64
    };
    (0..scans)
        .map(|i| {
            let (mut mz, mut mzs, mut it) =
                (100.0, Vec::with_capacity(peaks), Vec::with_capacity(peaks));
            for _ in 0..peaks {
                mz += 0.01 + rnd() * if hr { 0.9 } else { 4.0 };
                mzs.push(if hr {
                    (mz * 1e5).round() / 1e5
                } else {
                    (mz * 10.0).round() / 10.0
                });
                it.push((rnd() * rnd() * 1e5) as f32);
            }
            Spectrum {
                index: i,
                id: format!("scan={}", i + 1),
                level: 1,
                rt: i as f64 * 0.01,
                polarity: 1,
                filter: if hr {
                    "FTMS + p ESI Full ms [100-1500]".into()
                } else {
                    "+ c ESI Full ms [100-1500]".into()
                },
                mz: mzs,
                intensity: it,
                ..Default::default()
            }
        })
        .collect()
}

/// FNV-1a: enough to tell two outputs apart.
fn fnv(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf29ce484222325, |h, x| {
        (h ^ u64::from(*x)).wrapping_mul(0x100000001b3)
    })
}

fn write(sp: &[Spectrum], opz: Opzioni) -> Vec<u8> {
    let mut w = MzmlWriter::con_opzioni(
        Vec::new(),
        &Header {
            run: "x".into(),
            ..Default::default()
        },
        opz,
    )
    .unwrap();
    w.open_spectra(sp.len()).unwrap();
    for s in sp {
        w.spectrum_with(s, &SpectrumExtra::default()).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let scans: usize = a.get(1).and_then(|x| x.parse().ok()).unwrap_or(2000);
    let peaks: usize = a.get(2).and_then(|x| x.parse().ok()).unwrap_or(1500);
    for (name, hr) in [("HR", true), ("LR", false)] {
        let sp = spectra(scans, peaks, hr);
        let mut file = Vec::new();
        for (label, opz) in [
            (
                "plain",
                Opzioni {
                    thread: 1,
                    ..Opzioni::default()
                },
            ),
            (
                "numpress",
                Opzioni {
                    thread: 1,
                    numpress: true,
                    ..Opzioni::default()
                },
            ),
        ] {
            let t = Instant::now();
            let out = write(&sp, opz);
            println!(
                "{name} write {label:<8} {scans} x {peaks}: {:>8.1} ms  fnv {:016x}",
                t.elapsed().as_secs_f64() * 1e3,
                fnv(&out)
            );
            if label == "plain" {
                file = out;
            }
        }
        let size = file.len() as u64;
        let t = Instant::now();
        let p = upload::prepare(&mut file.clone(), size, "x.mzML").unwrap();
        println!(
            "{name} prepare              : {:>8.1} ms  probe {}",
            t.elapsed().as_secs_f64() * 1e3,
            p.probe
        );
        let (idx, _) = mzidx::build_mzml_to_vec(&p.table, &file, 192 << 20).unwrap();
        let n = idx.len() as u64;
        let mut routes = Routes::new(Index::open(idx, n).unwrap());
        let sids = (0..200.min(scans))
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        for (label, path, q) in [
            (
                "spectra",
                "/api/spectra",
                "i0=0&i1=59&level=1&bin=0.1".to_string(),
            ),
            ("scanavg", "/api/scanavg", format!("sids={sids}")),
        ] {
            let reps = 5;
            let (mut best, mut h) = (f64::MAX, 0);
            for _ in 0..reps {
                let mut f = file.clone();
                let t = Instant::now();
                let r = routes.mzml_route(path, &Query::parse(&q), &mut f).unwrap();
                best = best.min(t.elapsed().as_secs_f64() * 1e3);
                h = r.map_or(0, |s| fnv(s.as_bytes()));
            }
            println!(
                "{name} {label:<8}            : {best:>8.1} ms (best of {reps})  fnv {h:016x}"
            );
        }
    }
}
