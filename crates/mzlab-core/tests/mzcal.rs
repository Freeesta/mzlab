//! The m/z offset on an open file (`mzcal`): the estimate recovers the shift of the axis, `mzoff=` of `/api/xic` moves the windows and keeps the m/z
//! asked, `/api/mzcal` answers the evidence, a bad spec is `err.mz.offset.spec` and the offset of a request does not outlive it.
#![allow(clippy::indexing_slicing)]

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::routes::{Query, Routes};
use mzlab_core::upload::prepare;

/// 60 centroid scans of the ions 200.2 (apex scan 20) and 250.2 (apex scan 40) with the m/z axis moved by `shift`.
fn routes(shift: f64, ions: bool) -> Routes<Vec<u8>> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "cal".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(60).unwrap();
    for i in 0..60 {
        let (mz, intensity): (Vec<f64>, Vec<f32>) = if ions {
            [(200.2, 20.0), (250.2, 40.0)]
                .iter()
                .map(|&(m, apex)| {
                    let x = (i as f64 - apex) / 2.5;
                    (m + shift, (5e4 * (-0.5 * x * x).exp() + 30.0) as f32)
                })
                .unzip()
        } else {
            (vec![111.1, 333.3], vec![30.0, 25.0])
        };
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: i as f64 * 0.05,
            polarity: 1,
            filter: "+ c ESI Q1MS [50.00-400.00]".into(),
            mz,
            intensity,
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

fn get(r: &mut Routes<Vec<u8>>, path: &str, q: &str) -> String {
    r.handle(path, &Query::parse(q)).unwrap().unwrap()
}

/// The `"y":[..]` of the trace (without what rides on it: the offset members).
fn y_of(json: &str) -> &str {
    let at = json.find("\"y\":").unwrap();
    &json[at..at + json[at..].find(']').unwrap() + 1]
}

fn number(json: &str, key: &str) -> f64 {
    let at = json.find(&format!("\"{key}\":")).unwrap() + key.len() + 3;
    json[at..]
        .split([',', '}'])
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn the_route_recovers_the_shift_of_the_axis_within_20_mda() {
    for shift in [0.19, 0.24, 0.29, -0.31] {
        let mut r = routes(shift, true);
        for (mz, rt) in [("200.2", ""), ("250.2", ""), ("250.2", "&rt=2.0")] {
            let j = get(&mut r, "/api/mzcal", &format!("mz={mz}{rt}"));
            assert!(j.contains("\"status\":\"ok\""), "{shift} {mz}: {j}");
            assert!(
                (number(&j, "mz_offset_da") - shift).abs() <= 0.02,
                "{shift} {mz}: {j}"
            );
        }
    }
}

#[test]
fn an_ion_that_is_not_there_is_not_found_with_offset_zero() {
    let mut r = routes(0.24, false);
    let j = get(&mut r, "/api/mzcal", "mz=200.2");
    assert!(j.contains("\"status\":\"not_found\""), "{j}");
    assert_eq!(number(&j, "mz_offset_da"), 0.0);
    // the ion is there but far from the RT asked: nothing to measure
    let mut r = routes(0.24, true);
    let j = get(&mut r, "/api/mzcal", "mz=200.2&rt=2.9");
    assert!(j.contains("\"status\":\"not_found\""), "{j}");
}

#[test]
fn mzoff_moves_the_window_keeps_the_mz_and_does_not_outlive_the_request() {
    let mut r = routes(0.24, true);
    // `smooth=3` keeps the plain request on this route (without it, or `filt`, or `mzoff`, `/api/xic` is not answered here)
    let plain = get(&mut r, "/api/xic", "mz=200.2&tol=0.1&smooth=3");
    let fixed = get(&mut r, "/api/xic", "mz=200.2&tol=0.1&smooth=3&mzoff=0.24");
    let moved = get(&mut r, "/api/xic", "mz=200.44&tol=0.1&smooth=3");
    assert_eq!(y_of(&fixed), y_of(&moved));
    assert_ne!(y_of(&fixed), y_of(&plain));
    assert!(!fixed.contains("200.44"), "{fixed}");
    assert!(fixed.contains("\"mz_offset_da\":0.24"), "{fixed}");
    assert!(!plain.contains("mz_offset"), "{plain}");
    // auto is the number it measured
    let auto = get(
        &mut r,
        "/api/xic",
        "mz=200.2&tol=0.1&smooth=3&mzoff=auto:200.2",
    );
    assert_eq!(y_of(&auto), y_of(&fixed));
    assert!(auto.contains("\"mode\":\"auto\""), "{auto}");
    // the offset was only for those requests
    assert_eq!(get(&mut r, "/api/xic", "mz=200.2&tol=0.1&smooth=3"), plain);
}

#[test]
fn a_bad_spec_is_err_mz_offset_spec_and_a_big_offset_is_err_mz_offset_range() {
    let mut r = routes(0.24, true);
    for (spec, key) in [
        ("abc", "err.mz.offset.spec"),
        ("auto:", "err.mz.offset.spec"),
        ("0.7", "err.mz.offset.range"),
    ] {
        let e = r
            .handle(
                "/api/xic",
                &Query::parse(&format!("mz=200.2&tol=0.3&mzoff={spec}")),
            )
            .unwrap_err();
        assert_eq!(e.key, key, "{spec}");
    }
    let e = r.handle("/api/mzcal", &Query::parse("rt=1")).unwrap_err();
    assert_eq!(e.key, "err.mz.offset.spec");
}
