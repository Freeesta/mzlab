use super::*;

/// A side as the callers send it: the `rebin=` answer of the route with the bins and intensities given (m/z = the bin plus `shift`).
fn side(name: &str, precursor: f64, ces: &str, bins: &[(f64, f64)], shift: f64) -> String {
    let col = |f: &dyn Fn(&(f64, f64)) -> f64| bins.iter().map(|b| f(b).to_string()).collect::<Vec<_>>().join(",");
    format!(
        "{{\"name\":\"{name}\",\"precursor\":{precursor},\"ce_values\":[{ces}],\"spectrum\":{{\"mode\":\"bins\",\"rule\":\"nominal\",\"bin\":[{}],\"mz\":[{}],\"y\":[{}]}}}}",
        col(&|b| b.0),
        col(&|b| b.0 + shift),
        col(&|b| b.1)
    )
}

fn run(params: &str, a: String, b: String) -> Value {
    let t = compare_text(params, &format!("{{\"a\":{a},\"b\":{b}}}")).unwrap();
    parse(t.as_bytes()).unwrap()
}

fn f(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::f64).unwrap()
}

#[test]
fn a_known_cosine_of_square_root_intensities() {
    // A = (100: 4, 101: 9, 102: 16), B = (100: 4, 101: 9, 103: 25): the dot product is sqrt(16) + sqrt(81) = 13 over sqrt(29 * 38)
    let a = side("a", 253.1, "35", &[(100.0, 4.0), (101.0, 9.0), (102.0, 16.0)], 0.0);
    let b = side("b", 253.1, "35", &[(100.0, 4.0), (101.0, 9.0), (103.0, 25.0)], 0.0);
    let v = run("{}", a, b);
    assert_eq!(f(&v, "cosine"), round(13.0 / (29.0f64 * 38.0).sqrt(), 6));
    assert_eq!(f(&v, "n_matched"), 2.0);
    assert_eq!(v.get("matched").unwrap().arr().len(), 2);
    assert_eq!(f(&v.get("only_a").unwrap().arr()[0], "bin"), 102.0);
    assert_eq!(f(&v.get("only_b").unwrap().arr()[0], "bin"), 103.0);
    assert_eq!(f(v.get("a").unwrap(), "explained"), round(13.0 / 29.0, 6));
    assert_eq!(f(v.get("b").unwrap(), "explained"), round(13.0 / 38.0, 6));
    assert!(v.get("warnings").unwrap().arr().is_empty());
}

#[test]
fn the_same_spectrum_scores_one_and_disjoint_ones_zero() {
    let bins = [(100.0, 3.0), (110.0, 50.0)];
    let s = || side("s", 200.0, "20", &bins, 0.0);
    assert_eq!(f(&run("{}", s(), s()), "cosine"), 1.0);
    let other = side("o", 200.0, "20", &[(105.0, 3.0)], 0.0);
    let v = run("{}", s(), other);
    assert_eq!((f(&v, "cosine"), f(&v, "n_matched")), (0.0, 0.0));
}

#[test]
fn the_tolerance_is_a_parameter_and_the_default_is_logged() {
    let a = side("a", 200.0, "20", &[(100.0, 4.0)], 0.0);
    let b = side("b", 200.0, "20", &[(101.0, 4.0)], 0.0);
    let v = run("{}", a.clone(), b.clone());
    assert_eq!(f(&v, "cosine"), 0.0);
    assert_eq!(v.get("params").unwrap().get("default").unwrap().arr().len(), 2);
    let v = run("{\"tol\":1}", a, b);
    assert_eq!(f(&v, "cosine"), 1.0);
    assert_eq!(f(v.get("params").unwrap(), "tol"), 1.0);
    assert_eq!(v.get("params").unwrap().get("default").unwrap().arr().len(), 1);
}

#[test]
fn different_collision_energies_and_precursors_are_warned() {
    let a = side("a", 200.0, "20", &[(100.0, 4.0)], 0.0);
    let b = side("b", 205.0, "35", &[(100.0, 4.0)], 0.0);
    let v = run("{}", a.clone(), b.clone());
    let keys: Vec<&str> = v.get("warnings").unwrap().arr().iter().map(|w| w.get("key").and_then(Value::str).unwrap()).collect();
    assert_eq!(keys, ["warn.spectrum.ceDiffer", "warn.spectrum.precursorDiffer"]);
    // within the tolerance, and a mixed or missing CE
    assert!(run("{\"ce_tol\":20}", a.clone(), b.clone()).get("warnings").unwrap().arr().len() == 1);
    let mixed = side("m", 200.0, "20,35", &[(100.0, 4.0)], 0.0);
    let none = side("n", 200.0, "", &[(100.0, 4.0)], 0.0);
    let v = run("{}", mixed, none);
    let keys: Vec<&str> = v.get("warnings").unwrap().arr().iter().map(|w| w.get("key").and_then(Value::str).unwrap()).collect();
    assert_eq!(keys, ["warn.spectrum.ceMixed", "warn.spectrum.ceUnknown"]);
}

#[test]
fn both_intensities_and_masses_of_a_match_are_listed() {
    let a = side("a", 200.0, "20", &[(100.0, 4.0)], 0.1);
    let b = side("b", 200.0, "20", &[(100.0, 9.0)], -0.2);
    let v = run("{}", a, b);
    let m = &v.get("matched").unwrap().arr()[0];
    assert_eq!((f(m, "a"), f(m, "b")), (4.0, 9.0));
    assert_eq!((f(m, "mz_a"), f(m, "mz_b")), (100.1, 99.8));
}

#[test]
fn bad_input_names_the_key() {
    let ok = side("a", 200.0, "20", &[(100.0, 4.0)], 0.0);
    let err = |p: &str, s: &str| compare_text(p, s).unwrap_err().key;
    assert_eq!(err("{\"nope\":1}", "{}"), "err.spectrum.compare.param");
    assert_eq!(err("{\"tol\":-1}", "{}"), "err.spectrum.compare.value");
    assert_eq!(err("{}", &format!("{{\"a\":{ok}}}")), "err.spectrum.compare.input");
    let empty = side("e", 200.0, "20", &[(100.0, 0.0)], 0.0);
    assert_eq!(err("{}", &format!("{{\"a\":{ok},\"b\":{empty}}}")), "err.spectrum.compare.empty");
    let raw = "{\"name\":\"r\",\"spectrum\":{\"mode\":\"profile\"}}";
    assert_eq!(err("{}", &format!("{{\"a\":{ok},\"b\":{raw}}}")), "err.spectrum.compare.input");
}
