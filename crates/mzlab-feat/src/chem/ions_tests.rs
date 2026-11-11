//! Goldens of `ions/1`: m/z at 4 decimals computed apart from the engine (AME nuclide masses, electron 0.000548579909) for 17 molecules with Cl,
//! Br, S, P, F, isotopic labels (D, 13C), a cation and an anion; and the rules for which rows appear.

use super::ions_json;
use mzlab_core::json::{self, Value};

/// The atoms of a formula for the request: one atom per heavy atom (`13C` labelled, `D` as its own atom), the H spread three per atom (the engine
/// takes at most 16 on one atom), charge `q` on the first atom.
fn request(formula: &str, q: i64) -> String {
    let mut atoms = Vec::new();
    let mut h = 0;
    let mut rest = formula;
    while !rest.is_empty() {
        let (iso, r) = rest.strip_prefix("13").map_or((0, rest), |r| (13, r));
        let len = if r.len() > 1 && r.as_bytes()[1].is_ascii_lowercase() {
            2
        } else {
            1
        };
        let el = &r[..len];
        let digits: String = r[len..].chars().take_while(char::is_ascii_digit).collect();
        let n: u64 = digits.parse().unwrap_or(1);
        rest = &r[len + digits.len()..];
        if el == "H" {
            h += n;
            continue;
        }
        for _ in 0..n {
            atoms.push(format!("{{\"el\":\"{el}\",\"iso\":{iso}"));
        }
    }
    let last = atoms.len() - 1;
    let atoms: Vec<String> = atoms
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let hi = if i == last { h } else { h.min(3) };
            h -= hi;
            let q = if i == 0 { q } else { 0 };
            format!("{a},\"h\":{hi},\"q\":{q}}}")
        })
        .collect();
    format!("{{\"structures\":[[{}]],\"decimals\":4}}", atoms.join(","))
}

fn rows(formula: &str, q: i64) -> Vec<(String, String)> {
    let v = json::parse(ions_json(&request(formula, q)).unwrap().as_bytes()).unwrap();
    let s = v.get("structures").unwrap().at(0).unwrap();
    s.get("rows")
        .unwrap()
        .arr()
        .iter()
        .map(|r| {
            let t = |k: &str| r.get(k).and_then(Value::str).unwrap().to_string();
            (t("name"), t("mzText"))
        })
        .collect()
}

fn mz(rs: &[(String, String)], name: &str) -> Option<String> {
    rs.iter().find(|(n, _)| n == name).map(|(_, m)| m.clone())
}

/// formula, [M+H]+, [M+Na]+, [M-H]- (a D lost when the acid is deuterated), [M+Cl]-, [2M+H]+
const GOLD: [(&str, &str, &str, &str, &str, &str); 17] = [
    (
        "C8H10N4O2",
        "195.0877",
        "217.0696",
        "193.0731",
        "229.0498",
        "389.1680",
    ),
    (
        "C14H11Cl2NO2",
        "296.0240",
        "318.0059",
        "294.0094",
        "329.9861",
        "591.0406",
    ),
    (
        "C7H3Br2NO",
        "275.8654",
        "297.8474",
        "273.8509",
        "309.8275",
        "550.7236",
    ),
    (
        "C10H11N3O3S",
        "254.0594",
        "276.0413",
        "252.0448",
        "288.0215",
        "507.1115",
    ),
    (
        "C3H8NO5P", "170.0213", "192.0032", "168.0067", "203.9834", "339.0353",
    ),
    (
        "C8H14ClN5",
        "216.1010",
        "238.0830",
        "214.0865",
        "250.0632",
        "431.1948",
    ),
    (
        "C15H12N2O",
        "237.1022",
        "259.0842",
        "235.0877",
        "271.0644",
        "473.1972",
    ),
    (
        "C13H18O2", "207.1380", "229.1199", "205.1234", "241.1001", "413.2686",
    ),
    (
        "C8HF15O2", "414.9810", "436.9629", "412.9664", "448.9431", "828.9547",
    ),
    (
        "C10H12N2O3S",
        "241.0641",
        "263.0461",
        "239.0496",
        "275.0263",
        "481.1210",
    ),
    (
        "C4H11N5", "130.1087", "152.0907", "128.0942", "164.0708", "259.2102",
    ),
    (
        "C6H8O7", "193.0343", "215.0162", "191.0197", "226.9964", "385.0613",
    ),
    (
        "C6H12Cl3O4P",
        "284.9612",
        "306.9431",
        "282.9466",
        "318.9233",
        "568.9150",
    ),
    (
        "C10H7N3S", "202.0433", "224.0253", "200.0288", "236.0055", "403.0794",
    ),
    (
        "C8H9D5ClN5",
        "221.1324",
        "243.1144",
        "219.1179",
        "255.0946",
        "441.2576",
    ),
    (
        "13C6H12O6",
        "187.0908",
        "209.0727",
        "185.0762",
        "221.0529",
        "373.1743",
    ),
    (
        "C2D4O2", "65.0535", "87.0355", "62.0327", "99.0156", "129.0997",
    ),
];

#[test]
fn goldens_of_seventeen_molecules() {
    for (f, mh, mna, mmh, mcl, m2h) in GOLD {
        let rs = rows(f, 0);
        for (name, want) in [
            ("[M+H]+", mh),
            ("[M+Na]+", mna),
            ("[M-H]-", mmh),
            ("[M+Cl]-", mcl),
            ("[2M+H]+", m2h),
        ] {
            assert_eq!(mz(&rs, name).as_deref(), Some(want), "{f} {name}");
        }
    }
}

#[test]
fn doubly_charged_only_where_a_second_charge_sits() {
    // caffeine: 4 N, 194 Da → [M+2H]2+; 2 O → no [M-2H]2-
    let caf = rows("C8H10N4O2", 0);
    assert_eq!(mz(&caf, "[M+2H]2+").as_deref(), Some("98.0475"));
    assert_eq!(mz(&caf, "[M-2H]2-"), None);
    // citric acid: 7 O, 192 Da → [M-2H]2-; metformin: 5 N but 129 Da → none
    assert_eq!(
        mz(&rows("C6H8O7", 0), "[M-2H]2-").as_deref(),
        Some("95.0062")
    );
    assert_eq!(mz(&rows("C4H11N5", 0), "[M+2H]2+"), None);
}

#[test]
fn a_charged_structure_is_its_own_ion() {
    assert_eq!(
        rows("C4H12N", 1),
        vec![("[M]+".to_string(), "74.0964".to_string())]
    );
    assert_eq!(
        rows("C2H3O2", -1),
        vec![("[M]-".to_string(), "59.0139".to_string())]
    );
}

#[test]
fn a_row_needing_what_the_molecule_lacks_is_left_out() {
    // CCl4 has no H to lose; a fully deuterated acid loses a D
    let ccl4 = rows("CCl4", 0);
    assert_eq!(mz(&ccl4, "[M-H]-"), None);
    assert!(mz(&ccl4, "[M+Cl]-").is_some());
    let r = ions_json(&request("C2D4O2", 0)).unwrap();
    assert!(r.contains("\"name\":\"[M-H]-\",\"id\":\"mminush\",\"pol\":-1,\"k\":1,\"z\":-1,\"common\":true,\"formula\":\"C2D3O2\""), "{r}");
}

#[test]
fn decimals_ppm_window_and_errors() {
    let r =
        ions_json(r#"{"structures":[[{"el":"C","h":4}],[{"el":"R#"}],[]],"decimals":2,"ppm":10}"#)
            .unwrap();
    let v = json::parse(r.as_bytes()).unwrap();
    let s = v.get("structures").unwrap();
    let row = s.at(0).unwrap().get("rows").unwrap().at(0).unwrap();
    assert_eq!(row.get("mzText").and_then(Value::str), Some("17.04"));
    let (mzv, lo) = (
        row.get("mz").and_then(Value::f64).unwrap(),
        row.get("lo").and_then(Value::f64).unwrap(),
    );
    assert!(((mzv - lo) / mzv * 1e6 - 10.0).abs() < 0.1);
    assert_eq!(
        s.at(1).unwrap().get("error").and_then(Value::str),
        Some("error.etich.element")
    );
    assert_eq!(
        s.at(2).unwrap().get("error").and_then(Value::str),
        Some("error.ions.empty")
    );
    assert_eq!(ions_json("nope").unwrap_err().key, "error.ions.request");
}
