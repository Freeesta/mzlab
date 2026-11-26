//! Tests of `spectrum get --peak auto` at a profile level and at a high-resolution centroid level: a known apex over a flat background, the net line
//! kept, the flat line gone, and the cell and the tolerance named in the answer.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::idxread::Index;
use crate::json::{parse, Value};
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};
use crate::routes::{Query, Routes};

const APEX: f64 = 2.0;
const SIGMA: f64 = 0.05;
const HEIGHT: f64 = 1000.0;
/// The flat background (cps) of every point or line.
const BG: f64 = 20.0;
const FILTER: &str = "FTMS + c ESI Full ms2 250.3@hcd30.00 [50.00-500.00]";

fn gauss(t: f64) -> f64 {
    (HEIGHT * (-(t - APEX).powi(2) / (2.0 * SIGMA * SIGMA)).exp()).round()
}

/// The flat neighbour of the ion at a high-resolution level: 8 ppm above it and strong, so a tolerance of 10 ppm matches it and one of 5 ppm does not.
const NEIGHBOUR: f64 = 200.0;

/// 300 product-ion scans (precursor 250.3) every 0.01 min. `profile`: points every 0.02 from 100.00 to 100.60 (the ion rises at 100.30) and the flat band
/// 150.00 .. 150.60; else three centroid lines: the ion at 100.3 (rises with the peak), its flat neighbour and a flat line at 150.3.
fn routes(profile: bool) -> Routes<Vec<u8>> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "r".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(300).unwrap();
    for i in 0..300 {
        let rt = 0.5 + i as f64 * 0.01;
        let rise = gauss(rt);
        let (mz, y): (Vec<f64>, Vec<f64>) = if profile {
            let grid = (0..31).map(|k| 100.0 + k as f64 * 0.02);
            let band = (0..31).map(|k| 150.0 + k as f64 * 0.02);
            grid.chain(band)
                .map(|m| (m, BG + if (m - 100.3).abs() < 0.03 { rise } else { 0.0 }))
                .unzip()
        } else {
            (
                vec![100.3, 100.3 * (1.0 + 8e-6), 150.3],
                vec![BG + rise, NEIGHBOUR, BG],
            )
        };
        let s = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 2,
            rt,
            polarity: 1,
            profile,
            filter: if profile {
                String::new()
            } else {
                FILTER.into()
            },
            precursor: Some(250.3),
            mz,
            intensity: y.iter().map(|v| *v as f32).collect(),
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    let b = w.finish().unwrap();
    let mut src = b.clone();
    let p = crate::upload::prepare(&mut src, b.len() as u64, "r.mzML").unwrap();
    let idx = crate::mzidx::build_mzml_to_vec(&p.table, &b, 64 << 20)
        .unwrap()
        .0;
    let n = idx.len() as u64;
    Routes::new(Index::open(idx, n).unwrap())
}

fn ask(r: &mut Routes<Vec<u8>>, extra: &str) -> Value {
    let q = format!("rt0=1&rt1=3&level=2&precursor=250.3{extra}");
    parse(
        crate::rawinfo::handle(r, "/api/specpeak", &Query::parse(&q))
            .unwrap()
            .unwrap()
            .as_bytes(),
    )
    .unwrap()
}

fn lines(v: &Value, k: &str) -> Vec<f64> {
    v.get(k)
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().unwrap())
        .collect()
}

fn cell<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get("cell").unwrap().get(k).unwrap()
}

#[test]
fn a_high_resolution_level_gives_the_net_line_and_not_the_flat_one() {
    let mut r = routes(false);
    let v = ask(&mut r, "");
    assert_eq!(v.get("verdict").unwrap().str(), Some("peak"), "{v:?}");
    let (mz, y) = (lines(&v, "mz"), lines(&v, "y"));
    assert_eq!(mz.len(), 1, "{mz:?}");
    assert!((mz[0] - 100.3).abs() < 1e-3);
    let w = v.get("window_min").unwrap().arr();
    let (a, b) = (w[0].f64().unwrap(), w[1].f64().unwrap());
    let want: f64 = (0..300)
        .map(|i| 0.5 + i as f64 * 0.01)
        .filter(|t| *t >= a && *t <= b)
        .map(gauss)
        .sum();
    assert!((y[0] - want).abs() < 0.01 * want, "{} against {want}", y[0]);
    assert_eq!(cell(&v, "kind").str(), Some("hr_log"));
    assert_eq!(cell(&v, "tol_ppm").f64(), Some(5.0));
    assert_eq!(
        v.get("params").unwrap().get("tol_ppm").unwrap().f64(),
        Some(5.0)
    );
}

#[test]
fn the_tolerance_is_a_parameter_and_decides_what_the_sides_cancel() {
    let mut r = routes(false);
    let (tight, wide) = (ask(&mut r, ""), ask(&mut r, "&tol_ppm=10"));
    assert_eq!(cell(&wide, "tol_ppm").f64(), Some(10.0));
    // within 10 ppm the strong neighbour of the sides is taken off the ion in every scan of the window
    let n = tight.get("n_scans").unwrap().f64().unwrap();
    let drop = lines(&tight, "y")[0] - lines(&wide, "y")[0];
    assert!(
        (drop - (NEIGHBOUR - BG) * n).abs() < 1.0,
        "{drop} for {n} scans"
    );
    let e = crate::rawinfo::handle(
        &mut r,
        "/api/specpeak",
        &Query::parse("rt0=1&rt1=3&level=2&precursor=250.3&tol_ppm=0"),
    )
    .unwrap_err();
    assert_eq!(e.key, "err.spec.peak");
}

#[test]
fn a_profile_level_gives_the_net_points_by_cell_and_not_the_flat_band() {
    let mut r = routes(true);
    let v = ask(&mut r, "");
    assert_eq!(v.get("verdict").unwrap().str(), Some("peak"), "{v:?}");
    assert_eq!(cell(&v, "kind").str(), Some("profile_step"));
    assert!((cell(&v, "step_da").f64().unwrap() - 0.02).abs() < 1e-9);
    let mz = lines(&v, "mz");
    // only the points of the ion carry a net; the flat points of both bands cancel against the sides
    assert!(
        !mz.is_empty() && mz.iter().all(|m| (m - 100.3).abs() < 0.04),
        "{mz:?}"
    );
    assert!(lines(&v, "y").iter().all(|y| *y > 0.0));
}
