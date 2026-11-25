//! Tests of `specsum` (`spectrum get --sum`): the scans and counts of a window, the ions only when the fill time is stated, the ion quantum as the
//! other count, the refusal of a fill time that is not positive, and the agreement with the summed spectrum of `/api/spectrum`.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::idxread::Index;
use crate::json::{parse, Value};
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};
use crate::routes::{Query, Routes};

const Q: f64 = 200.0;

/// 100 product-ion scans (precursor 250.3) every 0.02 min from 1.0: ions of `Q` cps, 1 at m/z 100.3 and 2 more at m/z 150.3 on every second scan.
fn routes() -> Routes<Vec<u8>> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "r".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(100).unwrap();
    for i in 0..100 {
        let s = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 2,
            rt: 1.0 + i as f64 * 0.02,
            polarity: 1,
            precursor: Some(250.3),
            mz: vec![100.3, 150.3],
            intensity: vec![Q as f32, if i % 2 == 0 { 2.0 * Q as f32 } else { 0.0 }],
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

fn ask(r: &mut Routes<Vec<u8>>, path: &str, q: &str) -> crate::Result<Value> {
    let t = crate::rawinfo::handle(r, path, &Query::parse(q))?.unwrap();
    Ok(parse(t.as_bytes()).unwrap())
}

#[test]
fn the_scans_counts_and_ions_of_a_window_with_and_without_a_fill_time() {
    let mut r = routes();
    // scans 1.50..1.98 min: 25 scans, 12 of them with 600 cps and 13 with 200
    let v = ask(&mut r, "/api/specsum", "rt0=1.49&rt1=1.99&level=2").unwrap();
    assert_eq!(v.get("n_scans").unwrap().f64(), Some(25.0));
    let total = 12.0 * 600.0 + 13.0 * 200.0;
    assert_eq!(v.get("counts_cps").unwrap().f64(), Some(total));
    // no fill time in the index: ions null, with the key of the reason
    assert_eq!(v.get("ions"), Some(&Value::Null));
    assert_eq!(
        v.get("fill_time").unwrap().get("note").unwrap().str(),
        Some(super::specsum::NO_FILL)
    );
    assert_eq!(v.get("fill_time").unwrap().get("ms"), Some(&Value::Null));
    assert_eq!(
        v.get("background").unwrap().get("applied").unwrap().str(),
        Some("none")
    );
    // the quantum is the 200 cps of one ion: the count of ions the intensities themselves give
    assert_eq!(
        v.get("ion_quantum").unwrap().get("cps").unwrap().f64(),
        Some(Q)
    );
    assert_eq!(v.get("ions_by_quantum").unwrap().f64(), Some(total / Q));
    let types = v.get("by_scan_type").unwrap().arr();
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].get("n_scans").unwrap().f64(), Some(25.0));
    assert_eq!(types[0].get("ions"), Some(&Value::Null));
    // a fill time the user states: ions = cps x s
    let v = ask(
        &mut r,
        "/api/specsum",
        "rt0=1.49&rt1=1.99&level=2&fill_ms=50",
    )
    .unwrap();
    assert_eq!(v.get("ions").unwrap().f64(), Some(total * 0.05));
    assert_eq!(
        v.get("fill_time").unwrap().get("source").unwrap().str(),
        Some("user")
    );
    assert_eq!(v.get("fill_time").unwrap().get("note"), Some(&Value::Null));
    assert_eq!(
        v.get("by_scan_type").unwrap().arr()[0]
            .get("ions")
            .unwrap()
            .f64(),
        Some(total * 0.05)
    );
}

#[test]
fn a_fill_time_that_is_not_positive_is_refused_and_an_empty_window_has_no_scans() {
    let mut r = routes();
    for bad in ["0", "-5"] {
        let e = ask(
            &mut r,
            "/api/specsum",
            &format!("rt0=1.49&rt1=1.99&level=2&fill_ms={bad}"),
        )
        .unwrap_err();
        assert_eq!(e.key, "err.spec.fill", "{bad}");
    }
    let v = ask(&mut r, "/api/specsum", "rt0=50&rt1=51&level=2").unwrap();
    assert_eq!(v.get("n_scans").unwrap().f64(), Some(0.0));
    assert_eq!(v.get("counts_cps").unwrap().f64(), Some(0.0));
}

#[test]
fn the_evidence_agrees_with_the_summed_spectrum_of_the_same_window() {
    let mut r = routes();
    let sum = ask(
        &mut r,
        "/api/spectrum",
        "rt0=1.49&rt1=1.99&level=2&mode=sum",
    )
    .unwrap();
    let ev = ask(&mut r, "/api/specsum", "rt0=1.49&rt1=1.99&level=2").unwrap();
    assert_eq!(
        sum.get("scans").unwrap().f64(),
        ev.get("n_scans").unwrap().f64()
    );
    let y: f64 = sum
        .get("y")
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().unwrap())
        .sum();
    assert_eq!(Some(y), ev.get("counts_cps").unwrap().f64());
    // the mean of the same window is the sum over the scans
    let mean = ask(&mut r, "/api/spectrum", "rt0=1.49&rt1=1.99&level=2").unwrap();
    let ym: f64 = mean
        .get("y")
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().unwrap())
        .sum();
    assert!((ym * 25.0 - y).abs() < 1.0, "{ym} {y}");
}
