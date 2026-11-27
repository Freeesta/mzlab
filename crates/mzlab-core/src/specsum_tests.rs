//! Tests of `specsum` (`spectrum get --sum`): the scans and counts of a window, the ions only when the fill time is stated, the ion quantum as the
//! other count, the refusal of a fill time that is not positive, and the agreement with the summed spectrum of `/api/spectrum`.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::idxread::Index;
use crate::json::{parse, Value};
use crate::mzml::Spectrum;
use crate::mzmlwrite::{CvPar, Header, MzmlWriter, SpectrumExtra};
use crate::routes::{Query, Routes};

const Q: f64 = 200.0;

/// 100 product-ion scans (precursor 250.3) every 0.02 min from 1.0: ions of `Q` cps, 1 at m/z 100.3 and 2 more at m/z 150.3 on every second scan.
fn routes() -> Routes<Vec<u8>> {
    routes_with(|_| None)
}

/// `routes` where scan `i` states the fill time `fill(i)` (ms) as `MS:1000927` of its scan, as a Thermo mzML does.
fn routes_with(fill: impl Fn(usize) -> Option<f64>) -> Routes<Vec<u8>> {
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
        let extra = SpectrumExtra {
            scan_params: fill(i)
                .map(|t| CvPar::new("MS:1000927", "ion injection time", t))
                .into_iter()
                .collect(),
            ..Default::default()
        };
        w.spectrum_with(&s, &extra).unwrap();
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

/// The split of a sum names the scan types as `Scan::key` does (the survey and the product-ion scans of every instrument family), so the two lists agree.
#[test]
fn the_scan_type_of_the_split_is_the_key_of_the_scan() {
    use crate::idxread::Scan;
    let scan =
        |level: u8, filter: &str, act: Option<&str>, path: Option<Vec<(f64, String, f64)>>| Scan {
            level,
            filter: filter.into(),
            act: act.map(String::from),
            path,
            ..Default::default()
        };
    let scans = [
        scan(1, "FTMS + p ESI Full ms [100.00-1000.00]", None, None),
        scan(1, "Q1MS + ESI [50.0-500.0]  ", None, None),
        scan(1, "", None, None),
        scan(
            2,
            "ITMS + c ESI d Full ms2 364.20@cid35.00 [50.00-400.00]",
            Some("CID"),
            None,
        ),
        scan(
            2,
            "ITMS + c ESI d Full ms2 364.20@hcd30.00 [50.00-400.00]",
            None,
            Some(vec![(364.2, "hcd".into(), 30.0)]),
        ),
        scan(2, "", Some("HCD"), None),
        scan(2, "", None, None),
        scan(2, "+ EPI (364.2) CE (35)", Some("cid"), None),
        scan(2, "- p ESI Full ms2 100.0", None, None),
    ];
    for s in &scans {
        assert_eq!(
            super::specsum::scan_type(s),
            s.key(),
            "filter {:?}",
            s.filter
        );
    }
}

/// Scans 25..49 of `routes` are in the window below: 12 even ones of 600 cps and 13 odd ones of 200 cps.
const WIN: &str = "rt0=1.49&rt1=1.99&level=2";

#[test]
fn the_ions_come_from_the_fill_time_each_scan_states_and_fill_ms_overrides_it() {
    let mut r = routes_with(|i| Some(if i % 2 == 0 { 10.0 } else { 50.0 }));
    let v = ask(&mut r, "/api/specsum", WIN).unwrap();
    // 12 scans of 600 cps x 10 ms and 13 of 200 cps x 50 ms
    assert_eq!(v.get("ions").unwrap().f64(), Some(12.0 * 6.0 + 13.0 * 10.0));
    let f = v.get("fill_time").unwrap();
    assert_eq!(f.get("source").unwrap().str(), Some("file"));
    assert_eq!(f.get("note"), Some(&Value::Null));
    assert_eq!(
        f.get("ms").unwrap().f64(),
        Some((12.0 * 10.0 + 13.0 * 50.0) / 25.0)
    );
    assert_eq!(
        (
            f.get("min_ms").unwrap().f64(),
            f.get("max_ms").unwrap().f64(),
            f.get("n_timed").unwrap().f64()
        ),
        (Some(10.0), Some(50.0), Some(25.0))
    );
    let t = &v.get("by_scan_type").unwrap().arr()[0];
    assert_eq!(t.get("ions").unwrap().f64(), Some(202.0));
    // the user's time wins for every scan and says so
    let v = ask(&mut r, "/api/specsum", &format!("{WIN}&fill_ms=100")).unwrap();
    let total = 12.0 * 600.0 + 13.0 * 200.0;
    assert_eq!(v.get("ions").unwrap().f64(), Some(total * 0.1));
    let f = v.get("fill_time").unwrap();
    assert_eq!(
        (f.get("source").unwrap().str(), f.get("ms").unwrap().f64()),
        (Some("user"), Some(100.0))
    );
}

#[test]
fn ions_are_not_counted_when_only_some_scans_state_a_fill_time() {
    let mut r = routes_with(|i| (i % 2 == 0).then_some(10.0));
    let v = ask(&mut r, "/api/specsum", WIN).unwrap();
    assert_eq!(v.get("ions"), Some(&Value::Null));
    let f = v.get("fill_time").unwrap();
    assert_eq!(f.get("source").unwrap().str(), Some("file"));
    assert_eq!(f.get("n_timed").unwrap().f64(), Some(12.0));
    assert_eq!(f.get("note").unwrap().str(), Some(super::specsum::NO_FILL));
}

#[test]
fn an_index_of_version_3_keeps_ions_null_and_asks_for_a_rebuild() {
    // what the writer of version 3 gave: the same rows with no fill-time column, and the version 3 header
    let mut old = routes().idx.r.clone();
    assert_eq!(old[8..12], 4u32.to_le_bytes());
    old[8..12].copy_from_slice(&3u32.to_le_bytes());
    let n = old.len() as u64;
    let mut r = Routes::new(Index::open(old, n).unwrap());
    assert_eq!(r.idx.version, 3);
    let v = ask(&mut r, "/api/specsum", WIN).unwrap();
    assert_eq!(v.get("ions"), Some(&Value::Null));
    let f = v.get("fill_time").unwrap();
    assert_eq!(f.get("source").unwrap().str(), Some("none"));
    assert_eq!(
        f.get("note").unwrap().str(),
        Some(super::specsum::NO_FILL_OLD)
    );
    // and --fill-ms still counts them
    let v = ask(&mut r, "/api/specsum", &format!("{WIN}&fill_ms=50")).unwrap();
    assert!(v.get("ions").unwrap().f64().is_some());
}

#[test]
fn a_file_without_fill_times_has_a_version_4_index_and_the_plain_note() {
    let mut r = routes();
    assert_eq!(r.idx.version, 4);
    let v = ask(&mut r, "/api/specsum", WIN).unwrap();
    let f = v.get("fill_time").unwrap();
    assert_eq!(f.get("note").unwrap().str(), Some(super::specsum::NO_FILL));
}
