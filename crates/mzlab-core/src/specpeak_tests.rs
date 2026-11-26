//! Tests of `specpeak` (`spectrum get --peak auto`): a Gaussian product-ion peak of known apex and width over a flat background, the window of
//! `factor` x FWHM, the sides outside it, the background taken from both sides, and the verdict without an apex (no spectrum is made).
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::idxread::Index;
use crate::json::{parse, Value};
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};
use crate::routes::{Query, Routes};

const APEX: f64 = 2.0;
const SIGMA: f64 = 0.05;
const HEIGHT: f64 = 1000.0;
/// The flat background (cps) of the two masses: the one of the ion (m/z 100.3) and the one that never rises (m/z 150.3).
const BG: [f64; 2] = [20.0, 50.0];
const FWHM: f64 = 2.354_820_045 * SIGMA;

fn gauss(t: f64) -> f64 {
    (HEIGHT * (-(t - APEX).powi(2) / (2.0 * SIGMA * SIGMA)).exp()).round()
}

/// 300 product-ion scans (precursor 250.3) every 0.01 min from 0.5: m/z 100.3 rises with the peak, m/z 150.3 stays flat. `peak` false: both flat.
fn routes(peak: bool) -> Routes<Vec<u8>> {
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
        let rise = if peak { gauss(rt) } else { 0.0 };
        let s = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 2,
            rt,
            polarity: 1,
            precursor: Some(250.3),
            mz: vec![100.3, 150.3],
            intensity: vec![(BG[0] + rise) as f32, BG[1] as f32],
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

fn ask(r: &mut Routes<Vec<u8>>, q: &str) -> crate::Result<Value> {
    let t = crate::rawinfo::handle(r, "/api/specpeak", &Query::parse(q))?.unwrap();
    Ok(parse(t.as_bytes()).unwrap())
}

fn num(v: &Value, path: &[&str]) -> f64 {
    path.iter().fold(v, |v, k| v.get(k).unwrap()).f64().unwrap()
}

fn pair(v: &Value, k: &str) -> (f64, f64) {
    let a = v.get(k).unwrap().arr();
    (a[0].f64().unwrap(), a[1].f64().unwrap())
}

#[test]
fn the_apex_fwhm_and_window_of_a_known_peak() {
    let mut r = routes(true);
    let v = ask(&mut r, "rt0=1&rt1=3&level=2&precursor=250.3").unwrap();
    assert_eq!(v.get("verdict").unwrap().str(), Some("peak"));
    assert!(
        (num(&v, &["apex", "rt_min"]) - APEX).abs() <= 0.011,
        "{v:?}"
    );
    let fwhm = num(&v, &["fwhm_min"]);
    assert!((fwhm - FWHM).abs() < 0.015, "fwhm {fwhm} against {FWHM}");
    let (a, b) = pair(&v, "window_min");
    assert!(((b - a) - 2.0 * fwhm).abs() < 2e-4, "{a} {b}");
    assert!(((a + b) / 2.0 - num(&v, &["apex", "rt_min"])).abs() < 2e-4);
}

#[test]
fn the_sides_lie_outside_the_window_and_the_peak_and_the_background_is_flat() {
    let mut r = routes(true);
    let v = ask(&mut r, "rt0=1&rt1=3&level=2&precursor=250.3").unwrap();
    let (a, b) = pair(&v, "window_min");
    let sides = v.get("sides").unwrap();
    let (l, rr) = (
        pair(sides.get("left").unwrap(), "window_min"),
        pair(sides.get("right").unwrap(), "window_min"),
    );
    let (start, end) = (
        num(&v, &["apex", "start_min"]),
        num(&v, &["apex", "end_min"]),
    );
    assert!(l.1 <= a - 0.05 + 1e-3 && l.1 <= start, "{l:?} {a} {start}");
    assert!(rr.0 >= b + 0.05 - 1e-3 && rr.0 >= end, "{rr:?} {b} {end}");
    assert!((l.1 - l.0 - 0.3).abs() < 1e-3 && (rr.1 - rr.0 - 0.3).abs() < 1e-3);
    // the two ions of the background add to 70 cps per scan; the tails of the peak are outside the sides
    let cps = num(&v, &["background", "per_scan_cps"]);
    assert!((cps - (BG[0] + BG[1])).abs() < 1.0, "{cps}");
    assert_eq!(
        v.get("background").unwrap().get("applied").unwrap().str(),
        Some("sides")
    );
    assert_eq!(
        v.get("sum")
            .unwrap()
            .get("background")
            .unwrap()
            .get("applied")
            .unwrap()
            .str(),
        Some("sides")
    );
}

#[test]
fn the_spectrum_keeps_the_ion_and_drops_the_flat_mass() {
    let mut r = routes(true);
    let v = ask(&mut r, "rt0=1&rt1=3&level=2&precursor=250.3&fill_ms=20").unwrap();
    let mz: Vec<f64> = v
        .get("mz")
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().unwrap())
        .collect();
    let y: Vec<f64> = v
        .get("y")
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().unwrap())
        .collect();
    assert_eq!(mz.len(), 1, "{mz:?}");
    assert!((mz[0] - 100.3).abs() < 1e-3);
    // the net of the ion: the sum of the peak over the scans of the window (the background of 20 cps per scan is subtracted)
    let (a, b) = pair(&v, "window_min");
    let want: f64 = (0..300)
        .map(|i| 0.5 + i as f64 * 0.01)
        .filter(|t| *t >= a && *t <= b)
        .map(gauss)
        .sum();
    assert!((y[0] - want).abs() < 0.01 * want, "{} against {want}", y[0]);
    // n_scans and the ions are those of the sum evidence of the same window
    let n = num(&v, &["n_scans"]);
    assert_eq!(n, num(&v, &["sum", "n_scans"]));
    assert_eq!(num(&v, &["total_ions"]), num(&v, &["sum", "ions"]));
    assert!(num(&v, &["total_ions"]) > 0.0);
}

#[test]
fn the_window_factor_and_the_side_width_are_parameters() {
    let mut r = routes(true);
    let a = ask(&mut r, "rt0=1&rt1=3&level=2&precursor=250.3").unwrap();
    let b = ask(
        &mut r,
        "rt0=1&rt1=3&level=2&precursor=250.3&factor=3&side_min=0.2&gap_min=0.1",
    )
    .unwrap();
    let (w1, w2) = (pair(&a, "window_min"), pair(&b, "window_min"));
    assert!(((w2.1 - w2.0) / (w1.1 - w1.0) - 1.5).abs() < 0.01);
    let left = pair(b.get("sides").unwrap().get("left").unwrap(), "window_min");
    assert!((left.1 - left.0 - 0.2).abs() < 1e-3);
    assert_eq!(num(&b, &["params", "factor"]), 3.0);
    assert_eq!(num(&a, &["params", "factor"]), 2.0);
}

#[test]
fn a_flat_trace_has_no_apex_and_no_spectrum_is_made() {
    let mut r = routes(false);
    let v = ask(&mut r, "rt0=1&rt1=3&level=2&precursor=250.3").unwrap();
    assert_eq!(v.get("verdict").unwrap().str(), Some("no_apex"), "{v:?}");
    assert!(v.get("mz").unwrap().arr().is_empty() && v.get("y").unwrap().arr().is_empty());
    assert!(v.get("window_min").unwrap().is_null());
    assert!(v.get("rule").unwrap().str().unwrap().contains("no_apex"));
    assert_eq!(num(&v, &["trace", "n_scans_searched"]), 201.0);
}

#[test]
fn a_range_without_scans_is_no_data_and_bad_parameters_are_refused() {
    let mut r = routes(true);
    let v = ask(&mut r, "rt0=50&rt1=60&level=2&precursor=250.3").unwrap();
    assert_eq!(v.get("verdict").unwrap().str(), Some("no_data"));
    for bad in [
        "factor=0",
        "factor=x",
        "side_min=0",
        "gap_min=-1",
        "min_points=2.5",
        "min_sn=-1",
    ] {
        let e = ask(
            &mut r,
            &format!("rt0=1&rt1=3&level=2&precursor=250.3&{bad}"),
        )
        .unwrap_err();
        assert!(
            e.key == "err.spec.peak" || e.key == "err.request.badNumber",
            "{bad}: {}",
            e.key
        );
    }
    assert_eq!(
        ask(&mut r, "rt0=1&rt1=3&level=2&fill_ms=0")
            .unwrap_err()
            .key,
        "err.spec.fill"
    );
}
