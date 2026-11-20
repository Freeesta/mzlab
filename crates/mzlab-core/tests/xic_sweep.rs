//! `Routes::xic_sweep` gives, column by column and bit for bit, what `xic_into` / `xic_raw` give for each centre alone, on a synthetic file with peaks exactly
//! on the window edges, with overlapping windows, with a half-window in ppm (one per centre), smoothed and through the route `api/xic?sweep=`.
#![allow(clippy::indexing_slicing)]

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::routes::{Query, Routes};
use mzlab_core::routes_sweep::parse_sweep;
use mzlab_core::upload::prepare;

/// 60 scans of peaks every 0.25 from 98 to 312 (so every window edge of 0.5 and 0.25 holds a peak), a few of them twice, pseudo-random intensities.
fn routes() -> Routes<Vec<u8>> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "sweep".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(60).unwrap();
    let mut seed = 12345u64;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as f32 / 1000.0
    };
    for i in 0..60 {
        let mut mz = Vec::new();
        let mut it = Vec::new();
        for k in 0..=856 {
            if (k + i) % 7 != 0 {
                mz.push(98.0 + k as f64 * 0.25);
                it.push(next());
            }
        }
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: i as f64 * 0.05,
            polarity: 1,
            filter: "ITMS + c ESI Full ms [50.00-400.00]".into(),
            mz,
            intensity: it,
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let b = w.finish().unwrap();
    let p = prepare(&mut b.clone(), b.len() as u64, "s.mzML").unwrap();
    let idx = build_mzml_to_vec(&p.table, &b, 64 << 20).unwrap().0;
    let n = idx.len() as u64;
    Routes::new(Index::open(idx, n).unwrap())
}

fn same(a: &[f64], b: &[f64], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}");
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        assert_eq!(x.to_bits(), y.to_bits(), "{what} at {i}");
    }
}

#[test]
fn every_column_is_the_single_extraction_bit_for_bit() {
    let mut r = routes();
    let n = r.xic_len(1);
    assert_eq!(n, 60);
    for (spec, tol) in [
        ("100:300:1", 0.5),
        ("100:110:0.3", 0.5),
        ("100:150:0.25", 0.25),
        ("200:201:0.01", 0.005),
        ("99:101:2", 0.0),
        ("300:330:1", 0.5),
        ("90:99:3", 4.0),
    ] {
        let centres = parse_sweep(spec).unwrap();
        let (rt, cols) = r.xic_sweep(&centres, tol, 1).unwrap();
        assert_eq!(cols.len(), centres.len(), "{spec}");
        let mut one = vec![0.0; 2 * n];
        let mut signal = 0.0;
        for (c, col) in centres.iter().zip(&cols) {
            assert_eq!(r.xic_into(*c, tol, 1, &mut one).unwrap(), n);
            same(&rt, &one[..n], "rt");
            same(col, &one[n..], &format!("{spec} centre {c}"));
            signal += col.iter().sum::<f64>();
        }
        assert!(spec == "99:101:2" || signal > 0.0, "{spec} has signal");
    }
}

#[test]
fn centres_in_any_order_and_a_half_window_in_ppm_per_centre() {
    let mut r = routes();
    let centres = [250.0, 120.5, 120.5000001, 180.25, 100.0, 299.75];
    let tols: Vec<f64> = centres.iter().map(|c| c * 10.0e-6 * 2e3).collect();
    let m = r.sweep_matrix(&centres, &tols, 1, None).unwrap();
    for (k, (&c, &t)) in centres.iter().zip(&tols).enumerate() {
        let (_, y) = r.xic_raw(c, t, 1).unwrap();
        same(&m.y[k], &y, &format!("centre {c}"));
    }
    // windows that do not open in order (a negative half-window) fall back to the single sum, never a wrong column
    let odd = r
        .sweep_matrix(&[150.0, 160.0], &[-1.0, 0.5], 1, None)
        .unwrap();
    let (_, y) = r.xic_raw(160.0, 0.5, 1).unwrap();
    same(&odd.y[1], &y, "second");
    assert!(odd.y[0].iter().all(|&v| v == 0.0));
    // a level the file does not have: empty columns, one per centre
    let (rt, cols) = r.xic_sweep(&[100.0, 101.0], 0.5, 2).unwrap();
    assert!(rt.is_empty() && cols.len() == 2 && cols.iter().all(Vec::is_empty));
}

#[test]
fn the_route_and_the_smoothed_columns_are_the_page_routes_per_ion() {
    let mut r = routes();
    for smooth in ["", "&smooth=3", "&smooth=5"] {
        let got = r
            .handle(
                "/api/xic",
                &Query::parse(&format!("sweep=100:110:0.5&tol=0.4&level=1{smooth}")),
            )
            .unwrap()
            .unwrap();
        assert!(
            got.starts_with("{\"schema\":\"trace-set/1\",\"n_traces\":21,"),
            "{got}"
        );
        for c in parse_sweep("100:110:0.5").unwrap() {
            let one = r
                .handle(
                    "/api/xic",
                    &Query::parse(&format!(
                        "mz={c}&tol=0.4&level=1{}",
                        if smooth.is_empty() {
                            "&smooth=1"
                        } else {
                            smooth
                        }
                    )),
                )
                .unwrap();
            // a page route without a filter is the plain trace (`xic`): compare through it when `handle` does not answer
            let one = one.unwrap_or_else(|| r.xic(c, 0.4, 1).unwrap());
            let rt_y = &one[1..one.len() - 1];
            assert!(got.contains(rt_y), "{c} {smooth}");
        }
    }
    // the memo answers the same bytes
    let a = r
        .handle("/api/xic", &Query::parse("sweep=100:110:0.5&tol=0.4"))
        .unwrap();
    let b = r
        .handle("/api/xic", &Query::parse("sweep=100:110:0.5&tol=0.4"))
        .unwrap();
    assert_eq!(a, b);
    // the default half-window is STEP/2
    let d = r
        .handle("/api/xic", &Query::parse("sweep=100:110:0.5"))
        .unwrap()
        .unwrap();
    let e = r
        .handle("/api/xic", &Query::parse("sweep=100:110:0.5&tol=0.25"))
        .unwrap()
        .unwrap();
    assert_eq!(d, e);
    assert!(r.handle("/api/xic", &Query::parse("sweep=1:0:1")).is_err());
    assert!(r
        .handle("/api/xic", &Query::parse("sweep=100:110:1&mz=100"))
        .is_err());
}

#[test]
fn the_spec_is_the_list_mz_would_take() {
    let t = |s: &str| parse_sweep(s).unwrap();
    assert_eq!(&t("100:1000:1")[..3], [100.0, 101.0, 102.0]);
    assert_eq!(t("100:1000:1").len(), 901);
    assert_eq!(t("100:110:0.5").len(), 21);
    assert_eq!(&t("100:110:0.3")[..4], [100.0, 100.3, 100.6, 100.9]);
    assert_eq!(t("200:201:0.01").len(), 101);
    assert_eq!(t("200:201:0.01")[3], 200.03);
    for bad in [
        "1:0:1",
        "a:b:c",
        "100:200:0",
        "100:200",
        "100:200:1:2",
        "100:200:-1",
        "0:5:1",
    ] {
        assert_eq!(
            parse_sweep(bad).unwrap_err().key,
            "err.xic.sweep.spec",
            "{bad}"
        );
    }
    assert_eq!(
        parse_sweep("1:30000:1").unwrap_err().key,
        "err.xic.sweep.size"
    );
    assert_eq!(t("1:20000:1").len(), 20_000);
}

#[test]
fn a_matrix_over_the_budget_is_refused_before_it_is_built() {
    let mut r = routes();
    // 6 M centres x 60 scans x 8 B is far above half of the budget of any target
    let centres: Vec<f64> = (0..6_000_000).map(|i| 100.0 + i as f64 * 1e-6).collect();
    let tols = vec![0.5; centres.len()];
    let e = r.sweep_matrix(&centres, &tols, 1, None).err().unwrap();
    assert_eq!(e.key, "error.memoryBudget");
    assert!(e.params.iter().any(|(k, _)| *k == "fase"));
    assert_eq!(
        e.params.iter().find(|(k, _)| *k == "fase").unwrap().1,
        "xicSweep"
    );
}

/// 1,500 scans of 800 peaks each over 100-1000 (unit resolution), 1.2 M peaks.
fn big() -> Routes<Vec<u8>> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "big".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(1500).unwrap();
    let mut seed = 99u64;
    for i in 0..1500 {
        let mut mz = Vec::new();
        let mut it = Vec::new();
        for k in 0..800 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            mz.push(100.0 + (k as f64 + (seed >> 40) as f64 / 16_777_216.0) * 900.0 / 800.0);
            it.push(((seed >> 33) % 100_000) as f32 / 10.0);
        }
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: i as f64 * 0.002,
            polarity: 1,
            filter: "ITMS + c ESI Full ms [50.00-1100.00]".into(),
            mz,
            intensity: it,
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let b = w.finish().unwrap();
    let p = prepare(&mut b.clone(), b.len() as u64, "b.mzML").unwrap();
    let idx = build_mzml_to_vec(&p.table, &b, 256 << 20).unwrap().0;
    let n = idx.len() as u64;
    Routes::new(Index::open(idx, n).unwrap())
}

/// Timing, not a gate of the suite: `cargo test --release -p mzlab-core --test xic_sweep -- --ignored --nocapture`.
#[test]
#[ignore]
fn bench_900_centres_against_900_single_calls() {
    use std::time::{Duration, Instant};
    let mut r = big();
    let centres = parse_sweep("100:1000:1").unwrap();
    assert_eq!(centres.len(), 901);
    let _ = r.xic_raw(500.0, 0.5, 1).unwrap(); // the arrays of the section are read once
                                               // the best of 5 runs of each (the machine's noise is above a millisecond)
    for tol in [0.5, 5.0] {
        let (mut single, mut sweep) = (Duration::MAX, Duration::MAX);
        let mut keep = Vec::new(); // every result stays alive: both ways pay for fresh memory
        for i in 0..5 {
            let tol = tol + i as f64 * 1e-9; // another key each time: no memo, the walk is what is timed
            let t = Instant::now();
            let singles: Vec<Vec<f64>> = centres
                .iter()
                .map(|&c| r.xic_raw(c, tol, 1).unwrap().1)
                .collect();
            single = single.min(t.elapsed());
            keep.push(singles.clone());
            let t = Instant::now();
            let m = r
                .sweep_matrix(&centres, &vec![tol; centres.len()], 1, None)
                .unwrap();
            sweep = sweep.min(t.elapsed());
            for (k, s) in singles.iter().enumerate() {
                same(&m.y[k], s, "bench column");
            }
        }
        eprintln!("{} centres x 1500 scans, half-window {tol}: {} single calls {single:?}, one sweep {sweep:?}: {:.2}x", centres.len(), centres.len(), single.as_secs_f64() / sweep.as_secs_f64());
    }
}
