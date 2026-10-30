//! `export.inclusion` on the products of `tp.link`: the default adduct follows the polarity of the series (bentazone, synthetic, both polarities and resolutions).

use crate::flow_bentazone_tests::{fixture, plan, run};

const PROTON: f64 = 1.007_276;
/// The neutral +O product of the fixture (C10H12N2O4S) and the RT of its peak.
const OXIDISED: (f64, f64) = (256.0517, 4.3);

/// The plan of the bentazone series with an inclusion list after `tp.link` (`adducts` written only when given).
fn with_inclusion(lr: bool, adducts: Option<&str>) -> String {
    let base = plan(&|_| String::new(), lr);
    let params = adducts.map_or(String::new(), |a| {
        format!(r#","params":{{"adducts":"{a}"}}"#)
    });
    let at = base.rfind("]}").unwrap();
    format!(
        r#"{},{{"id":"inc","step":"export.inclusion@1","in":{{"tp":"lnk.tp"}}{params}}}]}}"#,
        &base[..at]
    )
}

/// The m/z column of the rows of a list.
fn mzs(csv: &str) -> Vec<f64> {
    csv.lines()
        .skip(1)
        .map(|l| l.split(',').next().unwrap().parse().unwrap())
        .collect()
}

fn case(tag: &str, pol: i8, hr: bool) {
    let d = fixture(tag, pol, hr);
    run(&d, &with_inclusion(!hr, None)).unwrap();
    let csv = std::fs::read_to_string(d.join("work/inc.csv")).unwrap();
    let want = OXIDISED.0 + f64::from(pol) * PROTON;
    let tol = if hr { 0.01 } else { 0.6 };
    assert!(
        mzs(&csv).iter().any(|m| (m - want).abs() < tol),
        "no row at {want} in {csv}"
    );
    // no row is the ion of the other polarity of the same product
    let other = OXIDISED.0 - f64::from(pol) * PROTON;
    assert!(mzs(&csv).iter().all(|m| (m - other).abs() > tol), "{csv}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_negative_series_gives_a_negative_list_with_no_adduct_written_hr() {
    case("inc_neg_hr", -1, true);
}

#[test]
fn a_negative_series_gives_a_negative_list_with_no_adduct_written_lr() {
    case("inc_neg_lr", -1, false);
}

#[test]
fn a_positive_series_keeps_the_proton_adduct_hr() {
    case("inc_pos_hr", 1, true);
}

#[test]
fn a_positive_series_keeps_the_proton_adduct_lr() {
    case("inc_pos_lr", 1, false);
}

#[test]
fn an_adduct_written_wins_over_the_polarity() {
    let d = fixture("inc_written", -1, true);
    run(&d, &with_inclusion(false, Some("[M+H]+"))).unwrap();
    let csv = std::fs::read_to_string(d.join("work/inc.csv")).unwrap();
    assert!(
        mzs(&csv)
            .iter()
            .any(|m| (m - (OXIDISED.0 + PROTON)).abs() < 0.01),
        "{csv}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
