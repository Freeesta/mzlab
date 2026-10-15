//! The similarity of the MS2 of a row with the parent's (`lr::ms2sim`): in `detail.sim` and `rows[].sim` when both have matched experiments.

use super::*;

fn parent_rt(d: &lr::LrDig) -> f64 {
    row(d, 0).ref_rt.unwrap()
}

fn json_of(d: &lr::LrDig, f: &[u8]) -> (Value, Value) {
    let cols = &base().0;
    let mut op = |k: usize| {
        Ok(if k < cols.len() {
            open(&cols[k])
        } else {
            open(f)
        })
    };
    let det: Value = serde_json::from_str(&detail(d, 7, &mut op).unwrap()).unwrap();
    let sum: Value = serde_json::from_str(&lr::summary::summary(d, "{}")).unwrap();
    (det, sum)
}

#[test]
fn a_row_and_the_parent_with_ms2_have_a_similarity() {
    let d = base().1.clone();
    let (pp, p7) = (p_of(&d, 0), p_of(&d, 7));
    let f = ms2_file(
        "both.mzML",
        &[
            x(pp, &[(parent_rt(&d), 0.07, 5.0e4)]),
            x(p7, &[(7.4, 0.07, 5.0e4)]),
        ],
    );
    let d = matched(d, &[("both.mzML", None, f.clone())], &[]);
    assert!(links(&d).parent_has_ms2);
    let (det, sum) = json_of(&d, &f);
    let sim = &det["sim"];
    assert!(sim["cos"]["total"].as_f64().unwrap() > 0.5, "{sim}");
    assert_eq!(
        sim["n"]["equal"].as_u64().unwrap()
            + sim["n"]["shifted"].as_u64().unwrap()
            + sim["n"]["loss"].as_u64().unwrap(),
        2
    );
    let r = sum["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == 7)
        .unwrap();
    let total = sim["cos"]["total"].as_f64().unwrap();
    assert!((r["sim"].as_f64().unwrap() - total).abs() < 6e-4, "{r}");
    // the parent itself and a row without MS2 have none
    let other = sum["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == 10007)
        .unwrap();
    assert!(other.get("sim").is_none());
}

#[test]
fn without_the_ms2_of_the_parent_there_is_no_similarity() {
    let d = base().1.clone();
    let p7 = p_of(&d, 7);
    let f = ms2_file("only.mzML", &[x(p7, &[(7.4, 0.07, 5.0e4)])]);
    let d = matched(d, &[("only.mzML", None, f.clone())], &[]);
    assert!(!links(&d).parent_has_ms2);
    let (det, sum) = json_of(&d, &f);
    assert!(det.get("sim").is_none());
    assert!(sum["rows"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r.get("sim").is_none()));
}
