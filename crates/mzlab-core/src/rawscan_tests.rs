//! Tests of `rawscan`: the mean spectrum of a window against its sides on a synthetic index (a steady background ion, a short burst of ions, a lone
//! ion), the bins, the offset, the cut and the refusals with their keys.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;
use crate::idxread::Index;
use crate::json::{parse, Value};
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};

/// 200 MS1 scans every 0.02 min from 1.0 min: an ion of 200 cps at m/z 150.3 in every scan (the background), three ions (600 cps) at 250.3 in the five
/// scans from about 2.7 min, and a single ion at 320.0 in one scan of that burst.
fn mzidx() -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "r".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(200).unwrap();
    for i in 0..200 {
        let burst = (85..90).contains(&i);
        let s = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 1.0 + i as f64 * 0.02,
            polarity: 1,
            mz: vec![150.3, 250.3, 320.0],
            intensity: vec![
                200.0,
                if burst { 600.0 } else { 0.0 },
                if i == 87 { 200.0 } else { 0.0 },
            ],
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    let b = w.finish().unwrap();
    let mut src = b.clone();
    let p = crate::upload::prepare(&mut src, b.len() as u64, "r.mzML").unwrap();
    crate::mzidx::build_mzml_to_vec(&p.table, &b, 64 << 20)
        .unwrap()
        .0
}

fn route(query: &str) -> Result<Value> {
    let idx = mzidx();
    let n = idx.len() as u64;
    let mut r = Routes::new(Index::open(idx, n).unwrap());
    let t = crate::rawinfo::handle(&mut r, "/api/rawscan", &Query::parse(query))?.unwrap();
    Ok(parse(t.as_bytes()).unwrap())
}

/// The row of a bin, by its centre.
fn row(v: &Value, centre: f64) -> Value {
    v.get("rows")
        .unwrap()
        .arr()
        .iter()
        .find(|r| r.get("mz_center").unwrap().f64() == Some(centre))
        .unwrap_or_else(|| panic!("no bin {centre} in {v:?}"))
        .clone()
}

const BURST: &str = "rt=2.69,2.79";

#[test]
fn a_burst_is_a_peak_the_steady_ion_is_nothing_and_a_lone_ion_is_nothing() {
    let v = route(BURST).unwrap();
    let s = v.get("summary").unwrap();
    assert_eq!(s.get("n_scans").unwrap().f64(), Some(5.0));
    let (l, r) = (
        s.get("n_left_scans").unwrap().f64().unwrap(),
        s.get("n_right_scans").unwrap().f64().unwrap(),
    );
    assert!(l > 0.0 && r > 0.0, "{s:?}");
    assert_eq!(s.get("n_side_scans").unwrap().f64(), Some(l + r));
    let p = row(&v, 250.0);
    assert_eq!(p.get("verdict").unwrap().str(), Some("peak"), "{p:?}");
    assert_eq!(p.get("mean").unwrap().f64(), Some(600.0));
    assert_eq!(p.get("background").unwrap().f64(), Some(0.0));
    assert_eq!(p.get("net").unwrap().f64(), Some(600.0));
    // a background of zero has no ratio, and says so
    assert_eq!(p.get("ratio"), Some(&Value::Null));
    assert_eq!(p.get("background_zero").unwrap().bool(), Some(true));
    assert_eq!(p.get("counts_net").unwrap().f64(), Some(15.0));
    assert!(p.get("z_counts").unwrap().f64().unwrap() > 3.8);
    // the steady background ion is in the window as much as beside it
    let b = row(&v, 150.0);
    assert_eq!(b.get("ratio").unwrap().f64(), Some(1.0));
    assert_eq!(b.get("net").unwrap().f64(), Some(0.0));
    assert_eq!(b.get("verdict").unwrap().str(), Some("nothing"));
    // one ion in one scan of five: net 1 ion, z 1
    let l = row(&v, 320.0);
    assert_eq!(l.get("verdict").unwrap().str(), Some("nothing"), "{l:?}");
    assert_eq!(s.get("bins_with_peak").unwrap().f64(), Some(1.0));
    assert_eq!(s.get("n_bins").unwrap().f64(), Some(3.0));
    // highest net first
    let order: Vec<f64> = v
        .get("rows")
        .unwrap()
        .arr()
        .iter()
        .map(|r| r.get("mz_center").unwrap().f64().unwrap())
        .collect();
    assert_eq!(order, [250.0, 320.0, 150.0]);
}

#[test]
fn top_cuts_the_rows_and_zero_keeps_every_bin() {
    let one = route(&format!("{BURST}&top=1")).unwrap();
    assert_eq!(one.get("rows").unwrap().arr().len(), 1);
    assert_eq!(
        one.get("summary").unwrap().get("n_bins").unwrap().f64(),
        Some(3.0)
    );
    let all = route(&format!("{BURST}&top=0")).unwrap();
    assert_eq!(all.get("rows").unwrap().arr().len(), 3);
}

#[test]
fn tenth_bins_range_and_offset_move_the_bins() {
    let v = route(&format!("{BURST}&bin=0.1")).unwrap();
    assert_eq!(row(&v, 250.3).get("verdict").unwrap().str(), Some("peak"));
    assert_eq!(v.get("bin").unwrap().str(), Some("0.1"));
    let r = route(&format!("{BURST}&mz_range=200,300")).unwrap();
    assert_eq!(r.get("rows").unwrap().arr().len(), 1);
    let o = route(&format!("{BURST}&bin=0.1&mz_offset=0.3&mz_range=249,251")).unwrap();
    assert_eq!(row(&o, 250.0).get("verdict").unwrap().str(), Some("peak"));
    assert_eq!(
        o.get("mz_offset").unwrap().get("da").unwrap().f64(),
        Some(0.3)
    );
}

#[test]
fn a_window_without_sides_has_a_zero_background_and_a_level_without_scans_has_no_rows() {
    let v = route("rt=1.0,4.98").unwrap();
    let s = v.get("summary").unwrap();
    assert_eq!(s.get("n_side_scans").unwrap().f64(), Some(0.0));
    assert_eq!(s.get("tic_side"), Some(&Value::Null));
    assert_eq!(
        row(&v, 150.0).get("background_zero").unwrap().bool(),
        Some(true)
    );
    let none = route(&format!("{BURST}&ms_level=2")).unwrap();
    assert_eq!(none.get("rows").unwrap().arr().len(), 0);
    assert_eq!(
        none.get("summary").unwrap().get("n_scans").unwrap().f64(),
        Some(0.0)
    );
}

#[test]
fn thresholds_are_parameters_and_members_name_them() {
    let strict = route(&format!("{BURST}&min_z=10")).unwrap();
    assert_eq!(
        row(&strict, 250.0).get("verdict").unwrap().str(),
        Some("trace")
    );
    let t = strict.get("thresholds").unwrap();
    assert_eq!(t.get("min_z").unwrap().f64(), Some(10.0));
    assert_eq!(t.get("min_net").unwrap().f64(), Some(1.0));
    assert_eq!(t.get("top").unwrap().f64(), Some(20.0));
    assert_eq!(
        strict.get("ion_quantum").unwrap().get("cps").unwrap().f64(),
        Some(200.0)
    );
}

#[test]
fn a_channel_selects_scans_and_bad_asks_are_refused_with_their_key() {
    let all = route(BURST).unwrap();
    let ch = route(&format!("{BURST}&channel=ms:ms1")).unwrap();
    assert_eq!(ch.get("rows"), all.get("rows"));
    assert_eq!(ch.get("channel").unwrap().str(), Some("ms:ms1"));
    assert_eq!(
        route(&format!("{BURST}&channel=ms:nope")).unwrap_err().key,
        "err.canali.unknown"
    );
    assert_eq!(
        route(&format!("{BURST}&channel=tic")).unwrap_err().key,
        "err.raw.scan.channel"
    );
    for bad in [
        "rt=2.7",
        "rt=3,2",
        "rt=a,b",
        "rt=2,3&bin=0.2",
        "rt=2,3&top=-1",
        "rt=2,3&mz_range=5",
        "rt=2,3&alpha=0.9",
        "rt=2,3&min_z=-1",
        "rt=2,3&ms_level=0",
    ] {
        assert_eq!(route(bad).unwrap_err().key, "err.raw.scan.param", "{bad}");
    }
}
