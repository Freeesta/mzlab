//! `quant.kinetics@1` on a synthetic series with known rates (two conditions sharing one t0, invented injection factors): the UV reference gives the rate
//! back, the same series without it does not, and every automatic choice and warning is in the output.

use super::kinetics::kinetics_text;
use mzlab_core::json::{parse, Value};

const TIMES: [f64; 6] = [5.0, 10.0, 20.0, 30.0, 45.0, 60.0];
/// Injection factors of the 13 files in order: t0, then the six times of each condition.
const INJ: [f64; 13] = [
    1.0, 0.93, 1.08, 0.97, 1.06, 0.91, 1.04, 1.09, 0.95, 1.02, 0.92, 1.07, 0.98,
];

fn names() -> Vec<(String, f64, f64)> {
    let mut v = vec![("t0.mzML".to_string(), 0.0, 0.0)];
    for (c, k) in [("Fe", 0.04), ("Cu", 0.015)] {
        v.extend(
            TIMES
                .iter()
                .map(|t| (format!("{c}_t{}.mzML", *t as i64), *t, k)),
        );
    }
    v
}

fn params(json: &str) -> Vec<(String, Value)> {
    match parse(json.as_bytes()).unwrap() {
        Value::Obj(o) => o,
        _ => unreachable!(),
    }
}

fn samples() -> Vec<u8> {
    let files: Vec<(String, String, String)> = names()
        .iter()
        .map(|n| (n.0.clone(), String::new(), String::new()))
        .collect();
    mzlab_core::samples::to_json(&mzlab_core::samples::propose(&files)).into_bytes()
}

/// `(areas CSV, uv CSV)`; the parent decays at the rate of its condition, `uv_decay` makes the UV reference decay at 0.02 per minute too.
fn series(uv_decay: bool) -> (String, String) {
    let (mut a, mut u) = (
        "file,id,area,sn,state\n".to_string(),
        "file,kind,rt_apex_min,rt_lo_min,rt_hi_min,fwhm_s,height_mau,area_mau_s,snr\n".to_string(),
    );
    for (i, (n, t, k)) in names().iter().enumerate() {
        let kk = if n.starts_with("t0") { 0.04 } else { *k };
        a += &format!(
            "{n},parent,{},50,detected\n",
            1.0e6 * (-kk * t).exp() * INJ[i]
        );
        a += &format!("{n},ghost,,0,absent\n");
        let uv = 500.0 * INJ[i] * if uv_decay { (-0.02 * t).exp() } else { 1.0 };
        u += &format!("{n},narrow,4.2,4.1,4.3,6,10,{uv},90\n{n},narrow,7.7,7.6,7.8,6,3,9.5,20\n");
    }
    (a, u)
}

fn run(p: &str, uv_decay: bool, with_uv: bool) -> (Value, Vec<String>) {
    let (a, u) = series(uv_decay);
    let (text, w) = kinetics_text(
        &params(p),
        a.as_bytes(),
        &samples(),
        if with_uv { u.as_bytes() } else { b"" },
    )
    .unwrap();
    (parse(text.as_bytes()).unwrap(), w)
}

fn k_of(doc: &Value, series: &str) -> f64 {
    let fit = doc
        .get("fits")
        .unwrap()
        .arr()
        .iter()
        .find(|f| {
            f.get("id").unwrap().str() == Some("parent")
                && f.get("series").unwrap().str() == Some(series)
        })
        .unwrap();
    fit.get("k").unwrap().f64().unwrap()
}

#[test]
fn the_uv_reference_gives_the_rate_back_and_without_it_the_injection_error_stays() {
    let (with, w) = run("{}", false, true);
    let (fe, cu) = (k_of(&with, "Fe"), k_of(&with, "Cu"));
    assert!(
        (fe - 0.04).abs() / 0.04 < 0.01 && (cu - 0.015).abs() / 0.015 < 0.01,
        "{fe} {cu}"
    );
    assert!(!w.iter().any(|k| k == "warn.flow.kinetics.uvOff"));
    let (_, tight) = run(r#"{"uv_tol":5}"#, false, true);
    assert!(
        tight.iter().any(|k| k == "warn.flow.kinetics.uvOff"),
        "{tight:?}"
    );
    let (without, _) = run("{}", false, false);
    assert!(
        (k_of(&without, "Fe") - 0.04).abs() > (fe - 0.04).abs() * 5.0,
        "{}",
        k_of(&without, "Fe")
    );
    assert_eq!(
        without.get("uv").unwrap().get("wired").unwrap().bool(),
        Some(false)
    );
    // the check mode reports the factors and leaves the areas alone
    let (check, _) = run(r#"{"uv_mode":"check"}"#, false, true);
    assert_eq!(k_of(&check, "Fe"), k_of(&without, "Fe"));
    assert!(check.get("files").unwrap().arr()[1]
        .get("factor")
        .unwrap()
        .f64()
        .is_some());
}

#[test]
fn t0_joins_every_series_and_each_fit_has_an_interval_and_its_log() {
    let (doc, _) = run("{}", false, true);
    let fits = doc.get("fits").unwrap().arr();
    let parent: Vec<&Value> = fits
        .iter()
        .filter(|f| f.get("id").unwrap().str() == Some("parent"))
        .collect();
    assert_eq!(parent.len(), 2);
    assert!(parent
        .iter()
        .all(|f| f.get("n").unwrap().f64() == Some(7.0)));
    let ci = parent[0].get("k_ci").unwrap().arr();
    assert!(ci[0].f64().unwrap() <= parent[0].get("k").unwrap().f64().unwrap());
    let (off, _) = run(r#"{"use_t0":false}"#, false, true);
    assert_eq!(
        off.get("fits").unwrap().arr()[0].get("n").unwrap().f64(),
        Some(6.0)
    );
    let reason = |d: &Value| {
        d.get("files").unwrap().arr()[0]
            .get("reason")
            .unwrap()
            .str()
            .map(String::from)
    };
    assert_eq!(reason(&off).as_deref(), Some("t0_not_used"));
    assert!(doc
        .get("choices")
        .unwrap()
        .arr()
        .iter()
        .any(|c| c.get("key").unwrap().str() == Some("uv_rt")));
    assert_eq!(
        doc.get("uv").unwrap().get("rt_min").unwrap().f64(),
        Some(4.2)
    );
}

#[test]
fn an_ion_that_is_never_a_peak_is_not_fitted_and_missing_areas_follow_the_parameter() {
    let (doc, w) = run(r#"{"ids":"parent;ghost"}"#, false, true);
    let ghost = doc
        .get("fits")
        .unwrap()
        .arr()
        .iter()
        .find(|f| f.get("id").unwrap().str() == Some("ghost"))
        .unwrap();
    assert_eq!(ghost.get("error").unwrap().str(), Some("too_few_times"));
    assert!(
        w.iter().any(|k| k == "warn.flow.kinetics.notFitted")
            && w.iter().any(|k| k == "warn.flow.kinetics.missing")
    );
    let (z, _) = run(r#"{"ids":"ghost","missing":"zero"}"#, false, true);
    assert_eq!(
        z.get("fits").unwrap().arr()[0].get("error").unwrap().str(),
        Some("no_signal")
    );
}

#[test]
fn a_reference_that_decays_is_not_an_injection_check_and_the_warning_says_so() {
    let (doc, w) = run("{}", true, true);
    assert!(w.iter().any(|k| k == "warn.flow.kinetics.uvTrend"), "{w:?}");
    assert!(
        doc.get("uv")
            .unwrap()
            .get("time_spearman")
            .unwrap()
            .f64()
            .unwrap()
            < -0.8
    );
}

#[test]
fn bad_parameters_and_inputs_are_errors_with_their_keys() {
    let (a, _) = series(false);
    let err = |p: &str, a: &[u8], s: &[u8]| {
        kinetics_text(&params(p), a, s, b"")
            .unwrap_err()
            .key
            .to_string()
    };
    assert_eq!(
        err(r#"{"uv_mode":"both"}"#, a.as_bytes(), &samples()),
        "err.flow.kinetics.param"
    );
    assert_eq!(
        err(r#"{"boot":2}"#, a.as_bytes(), &samples()),
        "err.flow.kinetics.param"
    );
    assert_eq!(
        err("{}", b"file,id\nx,y\n", &samples()),
        "err.flow.kinetics.areas"
    );
    assert_eq!(
        err("{}", b"file,id,area\nZ,parent,1\n", &samples()),
        "err.flow.kinetics.samples"
    );
    let twice = format!("{a}t0.mzML,parent,3,50,detected\n");
    assert_eq!(
        err("{}", twice.as_bytes(), &samples()),
        "err.flow.kinetics.row"
    );
}

#[test]
fn the_same_inputs_give_the_same_bytes() {
    let (a, u) = series(false);
    let go = || {
        kinetics_text(&params("{}"), a.as_bytes(), &samples(), u.as_bytes())
            .unwrap()
            .0
    };
    assert_eq!(go(), go());
    assert!(go().starts_with(r#"{"format":"kinetics-series/1""#));
}
