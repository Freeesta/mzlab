//! Tests of `pair` on small synthetic tables (invented neutral masses; ions made with the adduct table's own shifts).

use super::*;
use crate::json::parse;

const H: f64 = 1.007276;

// neutral masses 180.06339 (A), 250.12000 (B), 310.20000 (C); pos ions of A and B, neg ions of A and B and two decoys
fn pos() -> String {
    format!(
        "id,mz,rt\nP1,{:.5},5.00\nP2,{:.5},8.20\nP3,{:.5},3.10\n",
        180.06339 + H,
        250.12 + H,
        310.2 + H
    )
}

fn neg() -> String {
    format!(
        "id,mz,rt\nN1,{:.5},5.03\nN2,{:.5},8.18\nN3,{:.5},9.90\nN4,{:.5},3.12\n",
        180.06339 - H,
        250.12 - H,
        310.2 - H,
        310.3 - H
    )
}

fn go(pos: &str, neg: &str, params: &str) -> Result<crate::json::Value> {
    let req = format!(
        "{{\"pos\":{{\"name\":\"p.csv\",\"text\":{}}},\"neg\":{{\"name\":\"n.csv\",\"text\":{}}},\"params\":{params}}}",
        crate::jsonb::write(&crate::jsonb::text(pos)),
        crate::jsonb::write(&crate::jsonb::text(neg))
    );
    route(&req).map(|d| parse(d.as_bytes()).unwrap())
}

fn ids(d: &crate::json::Value, k: &str) -> Vec<String> {
    d.get(k)
        .unwrap()
        .arr()
        .iter()
        .map(|r| r.get("id").unwrap().str().unwrap().to_string())
        .collect()
}

#[test]
fn the_known_pairs_are_found_and_the_decoys_are_not() {
    let d = go(&pos(), &neg(), "{}").unwrap();
    let pairs = d.get("pairs").unwrap().arr();
    let got: Vec<(&str, &str)> = pairs
        .iter()
        .map(|p| {
            (
                p.get("pos").unwrap().str().unwrap(),
                p.get("neg").unwrap().str().unwrap(),
            )
        })
        .collect();
    assert_eq!(got, [("P1", "N1"), ("P2", "N2")]);
    assert_eq!(pairs[0].get("rule").unwrap().str(), Some("[M+H]+ / [M-H]-"));
    assert!(pairs
        .iter()
        .all(|p| p.get("mutual").unwrap().bool() == Some(true)));
    assert!(pairs[0].get("err_ppm").unwrap().f64().unwrap().abs() < 0.2);
    assert_eq!(pairs[0].get("drt").unwrap().f64(), Some(0.03));
    // N3 has the mass of C at the wrong RT, N4 the right RT at the wrong mass, P3 has no partner
    let un = d.get("unmatched").unwrap();
    assert_eq!(ids(un, "pos"), ["P3"]);
    assert_eq!(ids(un, "neg"), ["N3", "N4"]);
    let counts = d.get("counts").unwrap();
    assert_eq!(
        (
            counts.get("pairs").unwrap().f64(),
            counts.get("neg_unmatched").unwrap().f64()
        ),
        (Some(2.0), Some(2.0))
    );
}

#[test]
fn the_tolerances_are_parameters_and_never_widen_by_themselves() {
    let d = go(&pos(), &neg(), "{\"rt_tol\":0.01}").unwrap();
    assert_eq!(d.get("pairs").unwrap().arr().len(), 0);
    // N4 is 100 mDa from C: a Da window of 0.15 takes it, the default 5 ppm does not
    let d = go(
        &pos(),
        &neg(),
        "{\"mass_tol\":0.15,\"mass_unit\":\"da\",\"rt_tol\":0.1}",
    )
    .unwrap();
    let rules: Vec<_> = d
        .get("pairs")
        .unwrap()
        .arr()
        .iter()
        .map(|p| p.get("neg").unwrap().str().unwrap())
        .collect();
    assert_eq!(rules, ["N1", "N2", "N4"]);
    assert_eq!(
        d.get("params").unwrap().get("mass_unit").unwrap().str(),
        Some("da")
    );
}

#[test]
fn an_edited_adduct_list_pairs_other_species() {
    // a sodium adduct in positive mode against [M-H]- of the same neutral mass
    let p = format!("id,mz,rt\nP1,{:.5},4.00\n", 200.0 + 22.989218 - 0.000549);
    let n = format!("id,mz,rt\nN1,{:.5},4.02\n", 200.0 - H);
    assert_eq!(
        go(&p, &n, "{}").unwrap().get("pairs").unwrap().arr().len(),
        0
    );
    let d = go(&p, &n, "{\"pos_adducts\":\"[M+H]+,[M+Na]+\"}").unwrap();
    let pairs = d.get("pairs").unwrap().arr();
    assert_eq!(pairs.len(), 1);
    assert_eq!(
        pairs[0].get("rule").unwrap().str(),
        Some("[M+Na]+ / [M-H]-")
    );
    // the same list typed as a shift of the user's own
    let d = go(
        &p,
        &n,
        "{\"pos_adducts\":\"[M+Na]+,Na+:22.98865\",\"mass_tol\":10}",
    )
    .unwrap();
    assert_eq!(d.get("pairs").unwrap().arr().len(), 2);
}

#[test]
fn a_row_with_two_partners_keeps_both_and_marks_the_nearest() {
    let p = format!("id,mz,rt\nP1,{:.5},5.00\n", 180.06339 + H);
    let n = format!(
        "id,mz,rt\nN1,{:.5},5.05\nN2,{:.5},5.01\n",
        180.06339 - H + 0.0004,
        180.06339 - H
    );
    let d = go(&p, &n, "{}").unwrap();
    let pairs = d.get("pairs").unwrap().arr();
    assert_eq!(pairs.len(), 2);
    let mutual: Vec<_> = pairs
        .iter()
        .filter(|x| x.get("mutual").unwrap().bool() == Some(true))
        .map(|x| x.get("neg").unwrap().str().unwrap())
        .collect();
    assert_eq!(mutual, ["N2"]);
    assert_eq!(pairs[0].get("pos_partners").unwrap().f64(), Some(2.0));
}

#[test]
fn the_answer_does_not_depend_on_the_order_of_the_rows() {
    let rev = |t: &str| {
        let mut l: Vec<&str> = t.lines().collect();
        let h = l.remove(0);
        l.reverse();
        format!("{h}\n{}\n", l.join("\n"))
    };
    let a = route(&format!(
        "{{\"pos\":{{\"name\":\"p\",\"text\":{}}},\"neg\":{{\"name\":\"n\",\"text\":{}}}}}",
        crate::jsonb::write(&crate::jsonb::text(&pos())),
        crate::jsonb::write(&crate::jsonb::text(&neg()))
    ))
    .unwrap();
    let b = route(&format!(
        "{{\"pos\":{{\"name\":\"p\",\"text\":{}}},\"neg\":{{\"name\":\"n\",\"text\":{}}}}}",
        crate::jsonb::write(&crate::jsonb::text(&rev(&pos()))),
        crate::jsonb::write(&crate::jsonb::text(&rev(&neg())))
    ))
    .unwrap();
    let csv = |d: &str| {
        parse(d.as_bytes())
            .unwrap()
            .get("csv")
            .unwrap()
            .str()
            .unwrap()
            .to_string()
    };
    assert_eq!(
        csv(&a)
            .lines()
            .filter(|l| !l.contains("unmatched"))
            .collect::<Vec<_>>(),
        csv(&b)
            .lines()
            .filter(|l| !l.contains("unmatched"))
            .collect::<Vec<_>>()
    );
    assert!(csv(&a).starts_with("pos_id,neg_id,rule,neutral_mass,err_da,err_ppm,rt_pos,rt_neg,drt,mutual\nP1,N1,[M+H]+ / [M-H]-,180.06339,"));
    assert!(csv(&a).contains("\nP3,,unmatched,,,,3.1,,,\n"));
}

#[test]
fn a_bad_value_is_refused_with_its_name() {
    let key = |r: Result<crate::json::Value>| r.err().map(|e| e.key.to_string());
    assert_eq!(
        key(go(&pos(), &neg(), "{\"pos_adducts\":\"[M-H]-\"}")).as_deref(),
        Some("err.pair.param")
    );
    assert_eq!(
        key(go(&pos(), &neg(), "{\"neg_adducts\":\"[M+Foo]-\"}")).as_deref(),
        Some("err.pair.param")
    );
    assert_eq!(
        key(go(&pos(), &neg(), "{\"rt_tol\":-1}")).as_deref(),
        Some("err.pair.param")
    );
    assert_eq!(
        key(go(&pos(), &neg(), "{\"mass_unit\":\"mmu\"}")).as_deref(),
        Some("err.pair.param")
    );
    assert_eq!(
        key(go(&pos(), &neg(), "{\"nope\":1}")).as_deref(),
        Some("err.pair.param")
    );
    assert_eq!(
        key(go("id,mz,rt\nP1,abc,1\n", &neg(), "{}")).as_deref(),
        Some("err.pair.table")
    );
    assert_eq!(
        key(go("id,mz\nP1,100\n", &neg(), "{}")).as_deref(),
        Some("err.pair.table")
    );
    assert_eq!(
        key(go("id,mz,rt\n", "id,mz,rt\n", "{}")).as_deref(),
        Some("err.pair.empty")
    );
    assert_eq!(
        key(route("nope").map(|_| crate::json::Value::Null)).as_deref(),
        Some("err.pair.request")
    );
}
