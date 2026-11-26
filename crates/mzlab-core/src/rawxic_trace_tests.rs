//! `/api/xicions` on synthetic files whose ion quantum is known (one ion = 200 cps = 5 ms a point): every scan type gives the factor of its method
//! fields, the ions are the intensities of `/api/xic` times that factor (also smoothed), and a question the route cannot answer says why.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;
use crate::idxread::Index;
use crate::json::{parse, Value};
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};

const Q: f64 = 200.0;

/// 200 MS1 scans every 0.02 min with a `filter`; at m/z 250.3 `level(i)` times `Q` cps, at 150.3 one ion every scan (so the quantum is `Q`).
fn mzidx(filter: &str, level: impl Fn(usize) -> f64) -> Vec<u8> {
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
            intensity: vec![Q as f32, (level(i) * Q) as f32],
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

type Rt = Routes<Vec<u8>>;

fn routes(idx: Vec<u8>) -> Rt {
    let n = idx.len() as u64;
    Routes::new(Index::open(idx, n).unwrap())
}

fn ask(r: &mut Rt, query: &str) -> Result<Value> {
    r.xic_ions_route(&Query::parse(query))
        .map(|t| parse(t.unwrap().as_bytes()).unwrap())
}

fn peak(i: usize) -> f64 {
    if (85..90).contains(&i) {
        3.0
    } else {
        0.0
    }
}

fn nums(v: &Value) -> Vec<f64> {
    v.arr().iter().map(|x| x.f64().unwrap()).collect()
}

/// The `y` of `/api/xic` for 250.3 (the route the page draws), smoothed with `smooth` points when given.
fn xic_y(r: &mut Rt, smooth: Option<&str>) -> Vec<f64> {
    let t = match smooth {
        None => r.xic(250.3, 0.5, 1).unwrap(),
        Some(k) => {
            let q = format!("mz=250.3&tol=0.5&level=1&smooth={k}");
            r.handle("/api/xic", &Query::parse(&q)).unwrap().unwrap()
        }
    };
    nums(parse(t.as_bytes()).unwrap().get("y").unwrap())
}

/// Every scan type with the method fields that give 1 / Q seconds a point: the factor is the method's and the ions are `/api/xic`'s y times it.
#[test]
fn each_scan_type_gives_the_factor_of_its_method_fields_and_the_ions_follow_xic() {
    let cases = [
        ("ems", "fill_ms=5"),
        ("epi", "fill_ms=5"),
        ("sim", "dwell_ms=5"),
        ("q1", "scan_s=5&step_da=0.1&mass_lo=100&mass_hi=200"),
    ];
    for (kind, facts) in cases {
        let mut r = routes(mzidx("", peak));
        let v = ask(
            &mut r,
            &format!("mz=250.3,150.3&tol=0.5,0.5&ions=1&scan_type={kind}&{facts}"),
        )
        .unwrap();
        let s = v.get("ion_scale").unwrap();
        assert_eq!(s.get("scan_type").unwrap().str(), Some(kind));
        assert_eq!(s.get("source").unwrap().str(), Some("method"), "{kind}");
        assert!((s.get("ions_per_cps").unwrap().f64().unwrap() - 0.005).abs() < 1e-12);
        let t = v.get("traces").unwrap().arr();
        assert_eq!(t[0].get("mz").unwrap().f64(), Some(250.3));
        let ions = nums(t[0].get("ions").unwrap());
        let y = xic_y(&mut r, None);
        assert_eq!(ions.len(), y.len(), "{kind}");
        for (i, c) in ions.iter().zip(&y) {
            assert_eq!(*i, round(c * 0.005, 3), "{kind}");
        }
        // three ions a scan at the peak, nothing outside it; the quantum trace is one ion a scan
        assert_eq!(ions[87], 3.0);
        assert_eq!(ions[10], 0.0);
        assert!(nums(t[1].get("ions").unwrap()).iter().all(|&x| x == 1.0));
    }
}

#[test]
fn smoothing_is_the_filter_of_xic_before_the_scale() {
    let mut r = routes(mzidx("", peak));
    for k in ["3", "5"] {
        let v = ask(
            &mut r,
            &format!("mz=250.3&tol=0.5&ions=1&scan_type=ems&fill_ms=5&smooth={k}"),
        )
        .unwrap();
        let ions = nums(v.get("traces").unwrap().at(0).unwrap().get("ions").unwrap());
        let y = xic_y(&mut r, Some(k));
        assert!(
            ions.iter().zip(&y).all(|(i, c)| *i == round(c * 0.005, 3)),
            "smooth {k}"
        );
        assert_ne!(
            ions,
            nums(
                parse(ask_plain(&mut r).as_bytes())
                    .unwrap()
                    .get("ions")
                    .unwrap()
            )
        );
    }
}

fn ask_plain(r: &mut Rt) -> String {
    let t = r
        .xic_ions_route(&Query::parse(
            "mz=250.3&tol=0.5&ions=1&scan_type=ems&fill_ms=5",
        ))
        .unwrap()
        .unwrap();
    let v = parse(t.as_bytes()).unwrap();
    let ions = v.get("traces").unwrap().at(0).unwrap().get("ions").unwrap();
    format!(
        "{{\"ions\":[{}]}}",
        nums(ions)
            .iter()
            .map(f64::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// Without the method the data estimate is used, the scan type is read from the scans (SIM from the filter) and the choice says so.
#[test]
fn without_the_method_the_data_estimate_is_used_and_logged() {
    let mut r = routes(mzidx("+ SIM ms [250.0-251.0]", peak));
    let v = ask(&mut r, "mz=250.3&tol=0.5&ions=1").unwrap();
    let s = v.get("ion_scale").unwrap();
    assert_eq!(s.get("scan_type").unwrap().str(), Some("sim"));
    assert_eq!(s.get("source").unwrap().str(), Some("data"));
    assert_eq!(s.get("missing").unwrap().str(), Some("dwell_ms"));
    assert!((s.get("ions_per_cps").unwrap().f64().unwrap() - 0.005).abs() < 1e-12);
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
        .starts_with("data estimate"));
    assert_eq!(
        nums(v.get("traces").unwrap().at(0).unwrap().get("ions").unwrap())[87],
        3.0
    );
}

/// No method value and no accepted quantum: no factor, `ions` null for every ion, and the choice says why.
#[test]
fn no_factor_leaves_the_ions_null_and_says_so() {
    // a level the file has no scan of: no quantum, no method value, nothing to scale
    let mut r = routes(mzidx("", peak));
    let v = ask(&mut r, "mz=250.3&tol=0.5&ions=1&ms_level=2").unwrap();
    assert!(v.get("ion_scale").unwrap().is_null(), "{v:?}");
    assert!(v
        .get("traces")
        .unwrap()
        .at(0)
        .unwrap()
        .get("ions")
        .unwrap()
        .is_null());
    let e = v
        .get("ion_scale_choice")
        .unwrap()
        .get("evidence")
        .unwrap()
        .str()
        .unwrap();
    assert!(
        e.starts_with("none:") && e.ends_with("the ions arrays are absent"),
        "{e}"
    );
}

#[test]
fn a_question_the_route_cannot_answer_has_its_error_key() {
    let mut r = routes(mzidx("", peak));
    for (q, key) in [
        ("mz=250.3&tol=0.5", "err.raw.xic.param"),
        ("mz=250.3&tol=0.5&ions=0", "err.raw.xic.param"),
        ("mz=250.3,150.3&tol=0.5&ions=1", "err.raw.xic.param"),
        ("mz=250.3&tol=0.5&ions=1&smooth=4", "err.raw.xic.param"),
        ("mz=250.3&tol=0.5&ions=1&ms_level=0", "err.raw.xic.param"),
        ("mz=250.3&tol=0.5&ions=1&scan_type=ms", "err.raw.xic.param"),
        ("tol=0.5&ions=1", "err.raw.xic.param"),
        ("mz=250.3&ions=1", "err.raw.xic.param"),
        ("mz=250.3&tol=0.5&ions=1&fill_ms=-1", "err.raw.xic.param"),
    ] {
        assert_eq!(ask(&mut r, q).unwrap_err().key, key, "{q}");
    }
    let many = vec!["250.3"; MAX_IONS + 1].join(",");
    let e = ask(&mut r, &format!("mz={many}&tol={many}&ions=1")).unwrap_err();
    assert_eq!(e.key, "err.raw.xic.many");
}

#[test]
fn an_offset_moves_the_windows_and_is_undone_after_the_call() {
    let mut r = routes(mzidx("", peak));
    let v = ask(
        &mut r,
        "mz=250.0&tol=0.1&ions=1&scan_type=ems&fill_ms=5&mz_offset=0.3",
    )
    .unwrap();
    assert_eq!(
        nums(v.get("traces").unwrap().at(0).unwrap().get("ions").unwrap())[87],
        3.0
    );
    assert_eq!(r.mz_off, 0.0);
    let v = ask(&mut r, "mz=250.0&tol=0.1&ions=1&scan_type=ems&fill_ms=5").unwrap();
    assert_eq!(
        nums(v.get("traces").unwrap().at(0).unwrap().get("ions").unwrap())[87],
        0.0
    );
}
