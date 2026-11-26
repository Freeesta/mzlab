//! Tests of `raw xic` with `ions=1` (`ionscale`): synthetic files whose ion quantum is known for each scan type (a dwell, a fill time or a time
//! per step of 1 / q seconds), the method factor against the data estimate, the Poisson S/N on the ion scale, and the bytes without the opt-in.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;
use crate::idxread::Index;
use crate::json::{parse, Value};
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};

/// Cps of one ion in the synthetic files: one ion in 5 ms.
const Q: f64 = 200.0;

/// 200 MS1 scans every 0.02 min with a `filter`; at m/z 250.3 `ions(i)` ions of `Q` cps, at 150.3 one ion every scan (the quantum).
fn mzidx(filter: &str, ions: impl Fn(usize) -> f64) -> Vec<u8> {
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
        let s = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 1.0 + i as f64 * 0.02,
            polarity: 1,
            filter: filter.into(),
            mz: vec![150.3, 250.3],
            intensity: vec![Q as f32, (ions(i) * Q) as f32],
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

fn ask(idx: Vec<u8>, query: &str) -> Value {
    let n = idx.len() as u64;
    let mut r = Routes::new(Index::open(idx, n).unwrap());
    let t = crate::rawinfo::handle(&mut r, "/api/rawxic", &Query::parse(query))
        .unwrap()
        .unwrap();
    parse(t.as_bytes()).unwrap()
}

/// Three ions per scan over five scans at about 2.7 min, nothing else.
fn peak(i: usize) -> f64 {
    if (85..90).contains(&i) {
        3.0
    } else {
        0.0
    }
}

const Q_RT: &str = "mz=250.3&rt=2.5,3.0";
fn f(v: &Value, k: &str) -> f64 {
    v.get(k).unwrap().f64().unwrap()
}

/// Every scan type with the method fields that give 1 / Q seconds a point: the factor is the method's, agrees with the data and the ion scale is the quantum's.
#[test]
fn each_scan_type_gives_the_factor_of_its_method_fields() {
    let cases = [
        ("ems", "fill_ms=5"),
        ("epi", "fill_ms=5"),
        ("sim", "dwell_ms=5"),
        // 100 Da in 0.5 s with 0.1 Da steps: 1000 steps of 0.5 ms; 10 Da in 0.05 s... 5 ms a step needs 5 s for the range
        ("q1", "scan_s=5&step_da=0.1&mass_lo=100&mass_hi=200"),
    ];
    for (kind, facts) in cases {
        let v = ask(
            mzidx("", peak),
            &format!("{Q_RT}&ions=1&scan_type={kind}&{facts}"),
        );
        let s = v.get("ion_scale").unwrap();
        assert_eq!(s.get("scan_type").unwrap().str(), Some(kind));
        assert_eq!(s.get("source").unwrap().str(), Some("method"), "{kind}");
        assert!(
            (f(s, "ions_per_cps") - 0.005).abs() < 1e-12,
            "{kind}: {s:?}"
        );
        assert_eq!(s.get("agree"), Some(&Value::Bool(true)), "{kind}");
        let r = &v.get("rows").unwrap().arr()[0];
        assert_eq!(f(r, "ions_net"), f(r, "counts_net"), "{kind}");
        assert_eq!(f(r, "sn_ions"), f(r, "sn_floor"), "{kind}");
        assert_eq!(f(r, "ions_net"), 15.0);
        assert_eq!(f(r, "cps_net"), 15.0 / 0.005);
    }
}

/// Without the method the data estimate is used and the choice says so; SIM is read from the filter of the scans, not asked.
#[test]
fn without_the_method_the_data_estimate_is_used_and_logged() {
    let v = ask(
        mzidx("+ SIM ms [250.0-251.0]", peak),
        &format!("{Q_RT}&ions=1"),
    );
    let s = v.get("ion_scale").unwrap();
    assert_eq!(s.get("scan_type").unwrap().str(), Some("sim"));
    assert_eq!(s.get("source").unwrap().str(), Some("data"));
    assert_eq!(s.get("missing").unwrap().str(), Some("dwell_ms"));
    assert!((f(s, "ions_per_cps") - 0.005).abs() < 1e-12);
    let c = v.get("ion_scale_choice").unwrap();
    assert_eq!(
        c.get("scan_type_from").unwrap().str(),
        Some("the scans of the level")
    );
    assert!(c
        .get("evidence")
        .unwrap()
        .str()
        .unwrap()
        .contains("no dwell_ms"));
    // a file that does not name its scan type and no method: the data estimate under `other`
    let o = ask(mzidx("", peak), &format!("{Q_RT}&ions=1"));
    assert_eq!(
        o.get("ion_scale").unwrap().get("scan_type").unwrap().str(),
        Some("other")
    );
    assert_eq!(
        o.get("ion_scale").unwrap().get("source").unwrap().str(),
        Some("data")
    );
}

/// A method time that is not the data's is kept and shown against it: the ions scale with the method, the verdict can differ.
#[test]
fn a_method_time_twice_the_data_one_doubles_the_ions_and_says_it_disagrees() {
    let v = ask(
        mzidx("", peak),
        &format!("{Q_RT}&ions=1&scan_type=ems&fill_ms=10"),
    );
    let s = v.get("ion_scale").unwrap();
    assert_eq!(
        (f(s, "ratio"), s.get("agree")),
        (2.0, Some(&Value::Bool(false)))
    );
    let r = &v.get("rows").unwrap().arr()[0];
    assert_eq!((f(r, "counts_net"), f(r, "ions_net")), (15.0, 30.0));
    // the existing counts still use the quantum of the data: nothing above changed
    assert_eq!(f(r, "counts_gross"), 15.0);
}

/// The Poisson S/N on the ion scale: the height over one ion with no background, over sqrt(background in ions) above one.
#[test]
fn the_poisson_sn_is_the_height_over_the_ion_noise() {
    let v = ask(
        mzidx("", peak),
        &format!("{Q_RT}&ions=1&scan_type=sim&dwell_ms=5"),
    );
    let r = &v.get("rows").unwrap().arr()[0];
    assert_eq!(f(r, "sn_ions"), 3.0);
    assert_eq!(r.get("verdict_ions").unwrap().str(), Some("peak"));
    // four ions per scan of background: the height above it is the same three ions over sigma per scan = sqrt(4) ions
    let bg = |i: usize| if peak(i) > 0.0 { 7.0 } else { 4.0 };
    let v = ask(
        mzidx("", bg),
        &format!("{Q_RT}&ions=1&scan_type=sim&dwell_ms=5"),
    );
    let r = &v.get("rows").unwrap().arr()[0];
    assert_eq!(f(r, "ions_background_per_scan"), 4.0);
    assert_eq!(f(r, "sn_ions"), 3.0 / 2.0);
    // a time of 20 ms a point turns the same cps into four times the ions: the S/N is the other one
    let v = ask(
        mzidx("", peak),
        &format!("{Q_RT}&ions=1&scan_type=sim&dwell_ms=20"),
    );
    assert_eq!(f(&v.get("rows").unwrap().arr()[0], "ions_net"), 60.0);
}

/// Without the opt-in the answer has none of the new keys; a bad value is refused with the route's key.
#[test]
fn the_ion_scale_is_opt_in_and_its_inputs_are_checked() {
    let idx = mzidx("", peak);
    let off = ask(idx.clone(), Q_RT);
    assert!(
        off.get("ion_scale").is_none()
            && off.get("rows").unwrap().arr()[0].get("ions_net").is_none()
    );
    let zero = ask(idx.clone(), &format!("{Q_RT}&ions=0"));
    assert!(zero.get("ion_scale").is_none());
    for bad in [
        "ions=2",
        "ions=1&scan_type=ms",
        "ions=1&fill_ms=-1",
        "ions=1&dwell_ms=x",
    ] {
        let n = idx.len() as u64;
        let mut r = Routes::new(Index::open(idx.clone(), n).unwrap());
        let e = crate::rawinfo::handle(
            &mut r,
            "/api/rawxic",
            &Query::parse(&format!("{Q_RT}&{bad}")),
        )
        .unwrap_err();
        assert_eq!(e.key, "err.raw.xic.param", "{bad}");
    }
}
