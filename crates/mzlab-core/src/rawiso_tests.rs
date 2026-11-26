//! Tests of `rawiso`: the isotope ratios of a synthetic peak whose M, M+1 and M+2 counts are known, the verdicts against a formula, the carbon range, the
//! interference flag and the refusals with their keys.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;
use crate::idxread::Index;
use crate::json::{parse, Value};
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};

/// 200 MS1 scans every 0.02 min from 1.0 min. A peak of 21 scans (2.0 to 2.4 min, 200 cps per ion) at m/z 181.07 (M) with `m1` ions per scan at 182.07
/// and `m2` at 183.07; `m0` ions per scan at M. `steady` adds one ion per scan at 182.07 outside the peak (a background on the M+1 bin).
fn mzidx(m0: u32, m1: u32, m2: u32, steady: bool) -> Vec<u8> {
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
        let on = (50..71).contains(&i);
        let f = |n: u32| if on { 200.0 * n as f32 } else { 0.0 };
        let s = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 1.0 + i as f64 * 0.02,
            polarity: 1,
            mz: vec![181.07, 182.07, 183.07, 300.0],
            intensity: vec![
                f(m0),
                f(m1) + if steady { 200.0 } else { 0.0 },
                f(m2),
                200.0,
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

fn answer(idx: Vec<u8>, query: &str) -> Result<Value> {
    let n = idx.len() as u64;
    let mut r = Routes::new(Index::open(idx, n).unwrap());
    let t = crate::rawinfo::handle(&mut r, "/api/rawiso", &Query::parse(query))?.unwrap();
    Ok(parse(t.as_bytes()).unwrap())
}

const Q: &str = "mz=181.07&rt=2.0,2.4";

fn f(v: &Value, path: &[&str]) -> Option<f64> {
    let mut x = v;
    for k in path {
        x = x.get(k).unwrap();
    }
    x.f64()
}

#[test]
fn the_counts_ratios_and_sigma_of_a_known_pattern() {
    // 21 scans of 10 / 1 / 0 ions per scan at M, M+1 and M+2: 210, 21 and 0 ions
    let v = answer(mzidx(10, 1, 0, false), Q).unwrap();
    let r = v.get("result").unwrap();
    assert_eq!(f(r, &["ions"]), None);
    let ions = r.get("ions").unwrap().arr();
    assert_eq!(ions.len(), 3);
    assert_eq!(f(&ions[0], &["net"]), Some(210.0));
    assert_eq!(f(&ions[1], &["net"]), Some(21.0));
    assert_eq!(f(&ions[2], &["net"]), Some(0.0));
    assert_eq!(f(&ions[0], &["window"]), None);
    let r1 = r.get("observed").unwrap().get("r1").unwrap();
    assert_eq!(f(r1, &["r"]), Some(0.1));
    // sqrt(21 / 210^2 + 21^2 x 210 / 210^4)
    let sigma = (21.0f64 / 210.0f64.powi(2) + 21.0f64.powi(2) * 210.0 / 210.0f64.powi(4)).sqrt();
    assert!((f(r1, &["sigma"]).unwrap() - round(sigma, 4)).abs() < 1e-9);
    assert_eq!(f(&v, &["ion_quantum", "cps"]), Some(200.0));
    assert_eq!(
        r.get("region").unwrap().get("source").unwrap().str(),
        Some("peak")
    );
}

#[test]
fn a_formula_gives_z_scores_and_a_verdict_per_ratio() {
    // glucose [M+H]+ has M+1 near 6.9 % and M+2 near 1.3 % of M
    let ok = answer(mzidx(100, 7, 1, false), &format!("{Q}&formula=C6H12O6")).unwrap();
    let t = ok.get("result").unwrap().get("tests").unwrap();
    assert_eq!(
        t.get("r1").unwrap().get("verdict").unwrap().str(),
        Some("consistent"),
        "{t:?}"
    );
    let e = ok.get("result").unwrap().get("expected").unwrap();
    assert_eq!(e.get("adduct").unwrap().str(), Some("[M+H]+"));
    assert!((f(e, &["r1"]).unwrap() - 0.069).abs() < 0.004);
    // 30 ions per 100 at M+1 is far from a glucose
    let bad = answer(mzidx(100, 30, 1, false), &format!("{Q}&formula=C6H12O6")).unwrap();
    let t = bad.get("result").unwrap().get("tests").unwrap();
    assert_eq!(
        t.get("r1").unwrap().get("verdict").unwrap().str(),
        Some("inconsistent"),
        "{t:?}"
    );
    assert!(f(t, &["r1", "z"]).unwrap() > 3.0);
}

#[test]
fn too_few_counts_are_undetermined_with_their_reason() {
    // 1 ion per scan at M, none at M+1: N0 = 21, N1 = 0 < min_counts
    let v = answer(mzidx(1, 0, 0, false), &format!("{Q}&formula=C6H12O6")).unwrap();
    let t = v
        .get("result")
        .unwrap()
        .get("tests")
        .unwrap()
        .get("r1")
        .unwrap();
    assert_eq!(t.get("verdict").unwrap().str(), Some("undetermined"));
    assert_eq!(t.get("reason").unwrap().str(), Some("too_few_counts"));
    // the limit is a parameter: with 1 ion the ratio 0 is then judged
    let lax = answer(
        mzidx(1, 0, 0, false),
        &format!("{Q}&formula=C6H12O6&min_counts=0"),
    )
    .unwrap();
    assert_ne!(
        lax.get("result")
            .unwrap()
            .get("tests")
            .unwrap()
            .get("r1")
            .unwrap()
            .get("reason")
            .unwrap()
            .str(),
        Some("too_few_counts")
    );
}

#[test]
fn without_a_formula_the_carbon_range_and_the_m2_hint_are_evidence() {
    // R1 = 0.22: 20 carbons at 1.1 %; M+2 at 0.3 of M is more than a carbon chain explains
    let v = answer(mzidx(100, 22, 30, false), Q).unwrap();
    let r = v.get("result").unwrap();
    assert_eq!(r.get("expected"), Some(&Value::Null));
    let c = r.get("carbon_range").unwrap();
    assert_eq!(f(c, &["best"]), Some(20.0));
    assert!(f(c, &["min"]).unwrap() <= 20.0 && f(c, &["max"]).unwrap() >= 20.0);
    assert_eq!(
        r.get("m2_hint").unwrap().get("hint").unwrap().str(),
        Some("cl_br_s_possible")
    );
    let none = answer(mzidx(100, 22, 1, false), Q).unwrap();
    assert_eq!(
        none.get("result")
            .unwrap()
            .get("m2_hint")
            .unwrap()
            .get("hint")
            .unwrap()
            .str(),
        Some("none")
    );
}

#[test]
fn a_flat_m_plus_1_trace_is_flagged_as_interference() {
    // M+1 moves with M: r is 1; a bin that is constant over the peak has no variance, so no r
    let v = answer(mzidx(10, 1, 0, false), Q).unwrap();
    let i = v.get("result").unwrap().get("interference").unwrap();
    assert!(f(i, &["r_m1"]).unwrap() > 0.99, "{i:?}");
    assert_eq!(i.get("possible").unwrap().bool(), Some(false));
    // a steady ion on the M+1 bin: inside the peak M+1 = M+1 ions of the peak + 1 steady ion, the sides hold the steady ion
    let s = answer(mzidx(10, 1, 0, true), Q).unwrap();
    assert!(s
        .get("result")
        .unwrap()
        .get("interference")
        .unwrap()
        .get("r_m1")
        .is_some());
}

#[test]
fn a_level_without_scans_has_no_data_and_bad_asks_are_refused() {
    let v = answer(mzidx(10, 1, 0, false), &format!("{Q}&ms_level=2")).unwrap();
    let r = v.get("result").unwrap();
    assert_eq!(
        r.get("m").unwrap().get("verdict").unwrap().str(),
        Some("no_data")
    );
    assert_eq!(r.get("observed"), Some(&Value::Null));
    for bad in [
        "mz=0&rt=2,3",
        "mz=181&rt=3,2",
        "mz=181&rt=2",
        "mz=181&rt=2,3&bin=0.1",
        "mz=181&rt=2,3&min_r=2",
        "mz=181&rt=2,3&z_ok=5&z_bad=3",
        "mz=181&rt=2,3&min_points=0",
    ] {
        assert_eq!(
            answer(mzidx(1, 0, 0, false), bad).unwrap_err().key,
            "err.raw.isotopes.param",
            "{bad}"
        );
    }
    assert_eq!(
        answer(mzidx(1, 0, 0, false), "mz=181&rt=2,3&adduct=[M%2BH]%2B")
            .unwrap_err()
            .key,
        "err.raw.isotopes.adductNeedsFormula"
    );
    assert_eq!(
        answer(
            mzidx(1, 0, 0, false),
            "mz=181&rt=2,3&formula=C6H12O6&adduct=[M%2B2H]2%2B"
        )
        .unwrap_err()
        .key,
        "err.raw.isotopes.charge"
    );
    assert_eq!(
        answer(
            mzidx(1, 0, 0, false),
            "mz=181&rt=2,3&formula=C6H12O6&adduct=[M%2BX]%2B"
        )
        .unwrap_err()
        .key,
        "err.adduct.unknown"
    );
}
