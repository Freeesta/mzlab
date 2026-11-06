//! `features.detect` on degenerate HR indexes (no peaks, one scan, all-zero, NaN/inf/negative intensities, one m/z repeated, one compound,
//! unequal peak counts): the estimate, the detection and the weak candidates end with bytes or the expected clean refusal, never a panic, and
//! give the same bytes twice and with or without spare cores.

use mzlab_core::idxread::Index;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_feat_wasm::run;

const FULL: &str = "FTMS + p ESI Full ms [100.0000-1000.0000]";

/// An HR index of the scans `(m/z, intensity)`, one every 0.01 min, or the refusal of the upload.
fn index(scans: &[(Vec<f64>, Vec<f32>)]) -> Result<Index<Vec<u8>>, String> {
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(scans.len()).unwrap();
    for (s, (mz, it)) in scans.iter().enumerate() {
        w.spectrum(&Spectrum {
            index: s,
            id: format!("scan={}", s + 1),
            level: 1,
            rt: s as f64 * 0.01,
            polarity: 1,
            filter: FULL.into(),
            mz: mz.clone(),
            intensity: it.clone(),
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let b = w.finish().unwrap();
    let mut src = b.clone();
    let p = mzlab_core::upload::prepare(&mut src, b.len() as u64, "d.mzML")
        .map_err(|e| e.to_string())?;
    let idx = mzlab_core::mzidx::build_mzml_to_vec(&p.table, &b, 64 << 20)
        .map_err(|e| e.to_string())?
        .0;
    let n = idx.len() as u64;
    Index::open(idx, n).map_err(|e| e.to_string())
}

/// Estimate, detection and weak candidates: their bytes, or the error key of the first step that refused.
fn detect(scans: &[(Vec<f64>, Vec<f32>)]) -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut ix = index(scans)?;
    let r = run::recipe_for_estimated("", &mut ix).map_err(|e| e.to_string())?;
    let a = run::detect(&mut ix, "d", &r).map_err(|e| e.to_string())?;
    let b = run::detect_weak(&mut ix, &r).map_err(|e| e.to_string())?;
    Ok((a, b))
}

fn gauss(s: usize, apex: f64, h: f64) -> f32 {
    (h * (-0.5 * ((s as f64 - apex) / 4.0).powi(2)).exp()) as f32
}

/// The degenerate series, by name.
fn cases() -> Vec<(&'static str, Vec<(Vec<f64>, Vec<f32>)>)> {
    let noise = |s: usize, n: usize| -> (Vec<f64>, Vec<f32>) {
        let mz: Vec<f64> = (0..n)
            .map(|k| 100.0 + 900.0 * ((k * 7919 + s * 104_729) % 100_003) as f64 / 100_003.0)
            .collect();
        let mut mz = mz;
        mz.sort_by(f64::total_cmp);
        (
            mz,
            (0..n)
                .map(|k| (1e3 + ((k * 31 + s * 17) % 997) as f64) as f32)
                .collect(),
        )
    };
    let peak = |s: usize| {
        (
            vec![250.1234, 251.1268],
            vec![gauss(s, 30.0, 1e7), gauss(s, 30.0, 1e6)],
        )
    };
    vec![
        ("no peaks", (0..60).map(|_| (vec![], vec![])).collect()),
        ("one scan", vec![noise(0, 200)]),
        ("one scan one peak", vec![(vec![300.0], vec![5e6])]),
        (
            "all zero",
            (0..60).map(|s| (noise(s, 50).0, vec![0.0; 50])).collect(),
        ),
        (
            "nan and inf",
            (0..60)
                .map(|s| {
                    let (mz, mut it) = noise(s, 50);
                    it[s % 50] = f32::NAN;
                    it[(s + 7) % 50] = f32::INFINITY;
                    it[(s + 13) % 50] = -5.0;
                    (mz, it)
                })
                .collect(),
        ),
        (
            "nan only",
            (0..60)
                .map(|s| {
                    let (mz, mut it) = noise(s, 50);
                    it[3] = f32::NAN;
                    (mz, it)
                })
                .collect(),
        ),
        (
            "inf only",
            (0..60)
                .map(|s| {
                    let (mz, mut it) = noise(s, 50);
                    it[3] = f32::INFINITY;
                    (mz, it)
                })
                .collect(),
        ),
        (
            "negative only",
            (0..60)
                .map(|s| {
                    let (mz, mut it) = noise(s, 50);
                    it[3] = -5.0;
                    (mz, it)
                })
                .collect(),
        ),
        (
            "one m/z repeated",
            (0..60)
                .map(|s| (vec![400.0; 30], vec![gauss(s, 30.0, 1e6); 30]))
                .collect(),
        ),
        ("one compound", (0..60).map(peak).collect()),
        (
            "unequal counts",
            (0..60)
                .map(|s| {
                    let (mut mz, mut it) = noise(s, s * 13 % 300);
                    let (pm, pi) = peak(s);
                    mz.extend(pm);
                    it.extend(pi);
                    let mut o: Vec<usize> = (0..mz.len()).collect();
                    o.sort_by(|&a, &b| mz[a].total_cmp(&mz[b]));
                    (
                        o.iter().map(|&k| mz[k]).collect(),
                        o.iter().map(|&k| it[k]).collect(),
                    )
                })
                .collect(),
        ),
    ]
}

/// The refusal a case must end with (`None`: it must give bytes). A non-finite intensity makes a non-finite TIC, which the upload refuses.
fn refusal(name: &str) -> Option<&'static str> {
    ["nan and inf", "nan only", "inf only"]
        .contains(&name)
        .then_some("err.file.nonFinite")
}

#[test]
fn degenerate_series_end_cleanly_and_give_the_same_bytes_twice() {
    for (name, scans) in cases() {
        let first =
            std::panic::catch_unwind(|| detect(&scans)).unwrap_or_else(|_| panic!("{name}: panic"));
        assert_eq!(first, detect(&scans), "{name}: two runs differ");
        match (refusal(name), &first) {
            (None, Ok(_)) => {}
            (Some(key), Err(e)) => assert!(e.contains(key), "{name}: {e}"),
            (want, got) => panic!(
                "{name}: want {want:?}, got {:?}",
                got.as_ref().map(|_| "bytes")
            ),
        }
    }
}

#[test]
fn the_bytes_do_not_depend_on_the_spare_cores() {
    // 600 scans of 400 peaks: slabs large enough to be grouped by scan on several threads
    let scans: Vec<(Vec<f64>, Vec<f32>)> = (0..600)
        .map(|s| {
            let mut p: Vec<(f64, f32)> = (0..380)
                .map(|k| {
                    (
                        100.0 + 900.0 * ((k * 7919 + s * 104_729) % 100_003) as f64 / 100_003.0,
                        (2e3 + ((k * 31 + s) % 997) as f64) as f32,
                    )
                })
                .collect();
            for c in 0..20 {
                let m = 150.0 + c as f64 * 37.7;
                let h = 1e6 * (1.0 + c as f64);
                p.push((m, gauss(s, 25.0 + c as f64 * 28.0, h)));
                p.push((m + 1.003355, gauss(s, 25.0 + c as f64 * 28.0, h * 0.1)));
            }
            p.sort_by(|a, b| a.0.total_cmp(&b.0));
            (
                p.iter().map(|x| x.0).collect(),
                p.iter().map(|x| x.1).collect(),
            )
        })
        .collect();
    let free = detect(&scans).unwrap();
    let all = std::thread::available_parallelism().map_or(1, |n| n.get());
    let held = mzlab_core::spare::busy(all);
    let alone = detect(&scans).unwrap();
    drop(held);
    let empty = detect(&vec![(vec![], vec![]); 600]).unwrap();
    assert!(free.0.len() > empty.0.len(), "the series has features");
    assert_eq!(free, alone);
}
