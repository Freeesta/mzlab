//! Tests of `rawinfo` on synthetic indexes: sparse ion counts labelled profile (a unit-resolution full scan), a continuous profile on a 0.1 Da grid.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::idxread::Index;
use crate::json::parse;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};
use crate::routes::{Query, Routes};

/// The `.mzidx` of an mzML of `n` MS1 scans of positive polarity, the points of scan `i` from `points(i)`.
fn mzidx(n: usize, profile: bool, points: impl Fn(usize) -> (Vec<f64>, Vec<f32>)) -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "r".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let (mz, intensity) = points(i);
        let s = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 1.0 + i as f64 * 0.02,
            polarity: 1,
            profile,
            mz,
            intensity,
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

fn info(idx: Vec<u8>, query: &str) -> crate::json::Value {
    let n = idx.len() as u64;
    let mut r = Routes::new(Index::open(idx, n).unwrap());
    // the app's routes (`handle`): the engine's dispatch plus `/api/rawinfo`, which the engine itself does not answer
    assert_eq!(
        r.handle("/api/rawinfo", &Query::parse(query)).unwrap(),
        None
    );
    let t = super::handle(&mut r, "/api/rawinfo", &Query::parse(query))
        .unwrap()
        .unwrap();
    parse(t.as_bytes()).unwrap()
}

/// Isolated counts of 200 cps per ion at nominal masses + 0.3, a few per scan.
fn counts(i: usize) -> (Vec<f64>, Vec<f32>) {
    let mz: Vec<f64> = (0..6)
        .map(|k| 100.3 + ((i * 7 + k * 13) % 200) as f64)
        .collect();
    let mut mz = mz;
    mz.sort_by(f64::total_cmp);
    mz.dedup();
    let y = mz
        .iter()
        .enumerate()
        .map(|(k, _)| 200.0 * (1 + (i + k) % 3) as f32)
        .collect();
    (mz, y)
}

#[test]
fn sparse_counts_labelled_profile_are_centroid_data_with_a_quantum() {
    let v = info(mzidx(120, true, counts), "");
    assert_eq!(v.get("polarity").unwrap().arr()[0].str(), Some("+"));
    assert_eq!(v.get("n_scans").unwrap().f64(), Some(120.0));
    let g = v.get("grid").unwrap();
    assert_eq!(g.get("kind").unwrap().str(), Some("nominal"));
    assert_eq!(g.get("step_da").unwrap().f64(), Some(1.0));
    assert_eq!(g.get("phase_da").unwrap().f64(), Some(0.3));
    let q = v.get("ion_quantum").unwrap();
    assert_eq!(q.get("cps").unwrap().f64(), Some(200.0));
    assert_eq!(q.get("status").unwrap().str(), Some("ok"));
    let t = v.get("spectrum_type").unwrap();
    assert_eq!(t.get("label").unwrap().str(), Some("profile"));
    assert_eq!(t.get("data").unwrap().str(), Some("centroid"));
    assert_eq!(t.get("agree").unwrap().bool(), Some(false));
    let p = v.get("probe").unwrap();
    assert_eq!(p.get("scans").unwrap().f64(), Some(64.0));
    assert_eq!(
        v.get("mass_offset").unwrap().get("status").unwrap().str(),
        Some("not_measured")
    );
}

#[test]
fn a_profile_on_a_tenth_of_a_dalton_is_stepped_and_continuous() {
    let prof = |i: usize| {
        let mz: Vec<f64> = (0..400).map(|k| 200.0 + k as f64 * 0.1).collect();
        let y = mz
            .iter()
            .map(|m| (1000.0 * (-(m - 220.0 - i as f64 * 0.01).powi(2) / 0.08).exp() + 13.7) as f32)
            .collect();
        (mz, y)
    };
    let v = info(mzidx(30, true, prof), "probe=8");
    let g = v.get("grid").unwrap();
    assert_eq!(g.get("kind").unwrap().str(), Some("stepped"));
    assert_eq!(g.get("step_da").unwrap().f64(), Some(0.1));
    assert!(v.get("ion_quantum").unwrap().get("cps").unwrap().is_null());
    let t = v.get("spectrum_type").unwrap();
    assert_eq!(t.get("data").unwrap().str(), Some("profile"));
    assert_eq!(t.get("agree").unwrap().bool(), Some(true));
    assert_eq!(
        v.get("probe").unwrap().get("scans").unwrap().f64(),
        Some(8.0)
    );
    let range = v.get("mass_range").unwrap().arr();
    assert_eq!((range[0].f64(), range[1].f64()), (Some(200.0), Some(239.9)));
}

#[test]
fn a_bad_probe_is_refused_and_the_offset_is_measured_on_request() {
    let idx = mzidx(20, false, counts);
    let n = idx.len() as u64;
    let mut r = Routes::new(Index::open(idx.clone(), n).unwrap());
    for bad in ["probe=0", "probe=1.5", "probe=99999"] {
        let e = r.raw_info_route(&Query::parse(bad)).unwrap_err();
        assert_eq!(e.key, "err.raw.probe", "{bad}");
    }
    let v = info(idx, "offset_mz=150.3");
    let m = v.get("mass_offset").unwrap();
    assert_eq!(m.get("status").unwrap().str(), Some("measured"));
    assert!(m.get("evidence").unwrap().get("status").is_some());
}
