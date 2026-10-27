//! The adduct table of the m/z calculator: shifts against known ion masses, ppm and implied neutral mass of an observed m/z, differences.
use mzlab_core::adduct_table::table;
use serde_json::Value;

fn get(f: &str, obs: &str, tol: &str, delta: &str) -> Value {
    serde_json::from_str(&table(f, obs, tol, delta).unwrap()).unwrap()
}

fn row<'a>(t: &'a Value, n: &str) -> &'a Value {
    t["rows"].as_array().unwrap().iter().find(|r| r["n"] == n).unwrap()
}

fn near(a: &Value, b: f64, eps: f64) {
    let a = a.as_f64().unwrap();
    assert!((a - b).abs() < eps, "{a} vs {b}");
}

#[test]
fn shifts_match_the_known_ion_masses() {
    let t = get("", "", "", "");
    for (n, s) in [
        ("[M+H]+", 1.007276),
        ("[M+NH4]+", 18.033826),
        ("[M+Na]+", 22.989221),
        ("[M+K]+", 38.963158),
        ("[M+H-H2O]+", -17.003289),
        ("[M+H+ACN]+", 42.033826),
        ("[M-H]-", -1.007276),
        ("[M+Cl]-", 34.969402),
        ("[M+HCOO]-", 44.998203),
        ("[M+CH3COO]-", 59.013853),
        ("[M-H-H2O]-", -19.017841),
    ] {
        near(&row(&t, n)["shift"], s, 2e-6);
    }
    assert!(row(&t, "[M+H]+")["mz4"].is_null());
}

#[test]
fn the_ion_mz_follows_k_and_z() {
    let t = get("C13H26N4", "", "", "");
    assert_eq!(t["formula"], "C13H26N4");
    near(&t["neutral"], 238.2157, 1e-4);
    near(&row(&t, "[M+H]+")["mz4"], 239.2230, 1e-4);
    near(&row(&t, "[M+Na]+")["mz4"], 261.2050, 1e-4);
    near(&row(&t, "[2M+H]+")["mz4"], 477.4388, 1e-4);
    near(&row(&t, "[M+2H]2+")["mz4"], 120.1151, 1e-4);
    near(&row(&t, "[M-2H]2-")["mz4"], 118.1006, 1e-4);
    near(&row(&t, "[M-H]-")["mz4"], 237.2084, 1e-4);
    assert_eq!(row(&t, "[M+H]+")["nominal"], 239.0);
}

#[test]
fn an_observed_mz_marks_the_adduct_and_the_neutral_mass() {
    let t = get("C13H26N4", "261.2051", "5", "");
    assert_eq!(t["rows"][t["best"].as_u64().unwrap() as usize]["n"], "[M+Na]+");
    assert!(row(&t, "[M+Na]+")["ppm"].as_f64().unwrap().abs() < 1.0);
    assert!(row(&t, "[M+H]+")["ppm"].as_f64().unwrap().abs() > 5.0);
    let t = get("C13H26N4", "261.2051", "0.01", "");
    assert!(t["best"].is_null());
}

#[test]
fn an_observed_mz_without_formula_gives_the_neutral_mass_of_each_adduct() {
    let t = get("", "261,2051", "", "");
    assert!(t["neutral"].is_null() && t["best"].is_null());
    near(&row(&t, "[M+Na]+")["m_if"], 238.2157, 1e-3);
    near(&row(&t, "[M+H]+")["m_if"], 260.1978, 1e-3);
    near(&row(&t, "[2M+H]+")["m_if"], (261.2051 - 1.007276) / 2.0, 1e-3);
    near(&row(&t, "[M+2H]2+")["m_if"], 261.2051 * 2.0 - 2.014553, 1e-3);
    assert!(row(&t, "[M+Na]+")["ppm"].is_null());
}

#[test]
fn the_characteristic_differences_and_the_delta_search() {
    let t = get("", "", "", "21.9819");
    let d: Vec<(String, f64)> = t["diffs"].as_array().unwrap().iter().map(|x| (x["pair"].as_str().unwrap().to_string(), x["d"].as_f64().unwrap())).collect();
    assert!(d.contains(&("Na − H".to_string(), 21.981945)) || (d[0].1 - 21.981945).abs() < 1e-5);
    assert!((d.iter().find(|x| x.0 == "Cl").unwrap().1 - 35.976678).abs() < 1e-5);
    let h = t["delta"]["hits"].as_array().unwrap();
    assert_eq!((h[0]["a"].as_str(), h[0]["b"].as_str()), (Some("[M+Na]+"), Some("[M+H]+")));
    let none = get("", "", "5", "100");
    assert!(none["delta"]["hits"].as_array().unwrap().is_empty());
    let nh4 = get("", "", "", "17.0265");
    assert_eq!(nh4["delta"]["hits"][0]["a"], "[M+NH4]+");
}

#[test]
fn a_bad_formula_is_an_error_and_a_bad_number_is_ignored() {
    assert!(table("zz", "", "", "").is_err());
    assert!(get("CH4", "abc", "x", "")["best"].is_null());
}
