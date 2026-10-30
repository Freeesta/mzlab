//! `screen.suspects@1` on an invented list and table (no lab data): 3 suspects that are in the table (one with fragments that match its MS2), 2 isomers with one formula,
//! 2 isobaric decoys more than 5 ppm away and 1 suspect that is not in the table. The core is called directly for the rules and through the plan runner for the step.

use mzlab_core::elements::{mass, parse_formula};
use mzlab_core::json::{self, Value};
use mzlab_feat::facts::FileFacts;
use mzlab_feat::flow_screen_suspects::{screen_text, Params};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const PROTON: f64 = 1.007276;
const NA: f64 = 22.989218;
const CSV_HEAD: &str = "name,inchikey,formula,rt,fragments\n";

/// `(name, first block, formula, RT, listed fragments)`.
const ROWS: [(&str, &str, &str, &str, &str); 8] = [
    (
        "Alpha",
        "AAAAAAAAAAAAAA",
        "C10H12N2O3",
        "5.0",
        "120.0808;150.0550",
    ),
    (
        "Beta",
        "BBBBBBBBBBBBBB",
        "C12H15NO4",
        "6.0",
        "111.1000;222.2000",
    ),
    ("Gamma", "CCCCCCCCCCCCCC", "C9H11NO3S", "", ""),
    ("Delta a", "DDDDDDDDDDDDDD", "C14H20N2O4", "", ""),
    ("Delta b", "EEEEEEEEEEEEEE", "C14H20N2O4", "", ""),
    ("Decoy 1", "FFFFFFFFFFFFFF", "C11H12O4", "", ""),
    ("Decoy 2", "GGGGGGGGGGGGGG", "C9H12N4O2", "", ""),
    ("Absent", "HHHHHHHHHHHHHH", "C20H30O5", "", ""),
];

fn neutral(formula: &str) -> f64 {
    mass(&parse_formula(formula).unwrap())
}

pub(super) fn csv() -> String {
    ROWS.iter().fold(CSV_HEAD.to_string(), |s, r| {
        s + &format!("{},{},{},{},{}\n", r.0, r.1, r.2, r.3, r.4)
    })
}

/// A CSV with the rows of `ROWS` and nothing else (the same rows as `mzsus`; `names` false leaves the names out).
fn csv_keys(names: bool) -> String {
    ROWS.iter()
        .fold("name,inchikey,formula\n".to_string(), |s, r| {
            s + &format!("{},{},{}\n", if names { r.0 } else { "" }, r.1, r.2)
        })
}

/// A `.mzsus` v1 with the rows of `ROWS` (a file of the format of `docs/agenti/sospetti.md`).
fn mzsus() -> Vec<u8> {
    let mut rows: Vec<(f64, &str, &str)> = ROWS.iter().map(|r| (neutral(r.2), r.1, r.2)).collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(b.1)));
    let mut formulas: Vec<&str> = rows.iter().map(|r| r.2).collect();
    formulas.sort();
    formulas.dedup();
    let pad = |v: &mut Vec<u8>| v.resize(v.len().next_multiple_of(8), 0);
    let manifest = br#"{"formato":"sospetti/1","righe":8,"attribuzione":"Invented test list","fonti":[{"id":"test","nome":"Invented","versione":"1","licenza":"CC0","doi":""}]}"#;
    let mut o = b"MZSU".to_vec();
    o.extend([1, 0, 1, 0]);
    for n in [rows.len(), formulas.len(), 0, manifest.len(), 0, 0] {
        o.extend((n as u32).to_le_bytes());
    }
    let sections: Vec<Vec<u8>> = {
        let masses = rows.iter().flat_map(|r| r.0.to_le_bytes()).collect();
        let keys = rows.iter().flat_map(|r| r.1.bytes()).collect();
        let fidx = rows
            .iter()
            .flat_map(|r| (formulas.iter().position(|f| *f == r.2).unwrap() as u32).to_le_bytes())
            .collect();
        let mut offs = vec![0u32];
        for f in &formulas {
            offs.push(offs.last().unwrap() + f.len() as u32);
        }
        vec![
            manifest.to_vec(),
            masses,
            keys,
            fidx,
            vec![1; rows.len()],
            vec![0; 2 * rows.len()],
            vec![0; 2 * rows.len()],
            Vec::new(),
            offs.iter().flat_map(|x| x.to_le_bytes()).collect(),
            formulas.concat().into_bytes(),
        ]
    };
    for s in sections {
        o.extend(&s);
        pad(&mut o);
    }
    o
}

/// Features: Alpha [M+H]+, Beta [M+Na]+, Gamma [M+H]+, the Delta isomers [M+H]+ and one ion that is nobody's.
pub(super) fn table_bytes(polarity: i8) -> Vec<u8> {
    let sign = f64::from(polarity);
    let ions = [
        (neutral("C10H12N2O3") + sign * PROTON, 5.0, 1.0),
        (neutral("C12H15NO4") + NA, 6.0, 2.0),
        (neutral("C9H11NO3S") + sign * PROTON, 7.0, 0.0),
        (neutral("C14H20N2O4") + sign * PROTON, 8.0, -3.0),
        (500.0, 9.0, 0.0),
    ];
    let mut t = Aligned::with_capacity(1, ions.len());
    for (mz, rt, ppm) in ions {
        t.mz.push(mz * (1.0 + ppm * 1e-6));
        t.rt.push(rt);
        t.area.push(1e6);
        t.height.push(1e5);
        t.apex_rt.push(rt as f32);
        t.width.push(0.2);
        t.state.push(CELL_DETECTED);
    }
    let facts = FileFacts {
        polarity,
        resolution: 60000.0,
        ..FileFacts::default()
    };
    write_table(
        &t,
        &Recipe::default(),
        &Origin {
            inputs: vec![("a.mzML".into(), "aa".repeat(32))],
            facts: vec![facts],
            ..Origin::default()
        },
    )
}

pub(super) fn mgf() -> String {
    let mz = neutral("C10H12N2O3") + PROTON;
    format!("BEGIN IONS\nTITLE=a.1\nPEPMASS={mz:.5}\nRTINSECONDS=300.0\nSCANS=1\nMSLEVEL=2\n90.0 100\n120.0809 800\n150.0551 500\nEND IONS\n")
}

fn formulas_json() -> &'static str {
    r#"{"format":"formulas/1","features":[{"id":"1","result":{"candidates":[{"neutral":"C10H12N2O3"}]}}]}"#
}

fn params(json: &str) -> Params {
    match json::parse(json.as_bytes()).unwrap() {
        Value::Obj(o) => Params::from_params(&o),
        _ => unreachable!(),
    }
}

fn run(p: &str, list: &[(&str, &[u8])], mgf: Option<&str>, formulas: bool) -> Value {
    let f = [formulas_json().as_bytes()];
    let text = screen_text(
        &params(p),
        &table_bytes(1),
        list,
        mgf.map(str::as_bytes),
        if formulas { &f } else { &[] },
    )
    .unwrap();
    json::parse(text.as_bytes()).unwrap()
}

fn hits(v: &Value) -> &[Value] {
    v.get("hits").unwrap().arr()
}

fn names(h: &Value) -> Vec<String> {
    h.get("candidates")
        .unwrap()
        .arr()
        .iter()
        .map(|c| c.get("name").and_then(Value::str).unwrap().to_string())
        .collect()
}

#[test]
fn the_true_suspects_and_the_isomer_group_are_found_and_the_decoys_are_not() {
    let list = csv_keys(true);
    let out = run("{}", &[("list.csv", list.as_bytes())], None, false);
    let hs = hits(&out);
    let found: Vec<(i64, Vec<String>)> = hs
        .iter()
        .map(|h| (h.get("feature").unwrap().f64().unwrap() as i64, names(h)))
        .collect();
    assert_eq!(
        found,
        vec![
            (1, vec!["Alpha".into()]),
            (2, vec!["Beta".into()]),
            (3, vec!["Gamma".into()]),
            (4, vec!["Delta a".into(), "Delta b".into()])
        ],
        "{out:?}"
    );
    // the isomers are one row with one formula, never a single name
    assert_eq!(
        hs[3].get("formula").and_then(Value::str),
        Some("C14H20N2O4")
    );
    assert_eq!(hs[3].get("adduct").and_then(Value::str), Some("[M+H]+"));
    assert_eq!(hs[1].get("adduct").and_then(Value::str), Some("[M+Na]+"));
    let delta = hs[0].get("delta_ppm").and_then(Value::f64).unwrap();
    assert!((delta - 1.0).abs() < 0.01, "{delta}");
    assert!(hs
        .iter()
        .all(|h| h.get("level").and_then(Value::f64) == Some(5.0)));
    // the decoys and the absent suspect are not listed
    let all = names(&Value::Obj(vec![(
        "candidates".into(),
        Value::Arr(
            hs.iter()
                .flat_map(|h| h.get("candidates").unwrap().arr().to_vec())
                .collect(),
        ),
    )]));
    assert!(
        all.iter().all(|n| !n.starts_with("Decoy") && n != "Absent"),
        "{all:?}"
    );
}

#[test]
fn fragments_decide_with_min_fragments() {
    let list = csv();
    let list = list.as_bytes();
    let with = run("{}", &[("list.csv", list)], Some(&mgf()), false);
    let count = |v: &Value, i: usize| {
        hits(v)[i]
            .get("candidates")
            .unwrap()
            .at(0)
            .unwrap()
            .get("fragments_matched")
            .and_then(Value::f64)
            .unwrap()
    };
    assert_eq!(count(&with, 0), 2.0);
    assert_eq!(count(&with, 1), 0.0);
    let only = run(
        r#"{"min_fragments":2}"#,
        &[("list.csv", list)],
        Some(&mgf()),
        false,
    );
    assert_eq!(hits(&only).len(), 1);
    assert_eq!(names(&hits(&only)[0]), vec!["Alpha"]);
    assert_eq!(
        hits(&only)[0]
            .get("candidates")
            .unwrap()
            .at(0)
            .unwrap()
            .get("fragment_mz")
            .unwrap()
            .arr()
            .len(),
        2
    );
}

#[test]
fn rt_window_drops_the_row_that_lists_a_far_rt() {
    let list = csv();
    let off = run("{}", &[("list.csv", list.as_bytes())], None, false);
    let on = run(
        r#"{"rt_window_min":0.5}"#,
        &[("list.csv", list.as_bytes())],
        None,
        false,
    );
    assert_eq!(hits(&off).len(), 4);
    assert_eq!(
        hits(&on).len(),
        4,
        "5.0 and 6.0 are the listed RTs of the features 1 and 2"
    );
    let far = list.replace(
        "Alpha,AAAAAAAAAAAAAA,C10H12N2O3,5.0",
        "Alpha,AAAAAAAAAAAAAA,C10H12N2O3,9.0",
    );
    let dropped = run(
        r#"{"rt_window_min":0.5}"#,
        &[("list.csv", far.as_bytes())],
        None,
        false,
    );
    assert_eq!(hits(&dropped).len(), 3);
    let kept = run("{}", &[("list.csv", far.as_bytes())], None, false);
    assert_eq!(
        hits(&kept)[0]
            .get("candidates")
            .unwrap()
            .at(0)
            .unwrap()
            .get("rt_diff_min")
            .and_then(Value::f64),
        Some(-4.0)
    );
}

#[test]
fn level_4_when_the_formula_is_supported() {
    let list = csv_keys(true);
    let out = run("{}", &[("list.csv", list.as_bytes())], None, true);
    let levels: Vec<f64> = hits(&out)
        .iter()
        .map(|h| h.get("level").and_then(Value::f64).unwrap())
        .collect();
    assert_eq!(levels, vec![4.0, 5.0, 5.0, 5.0]);
}

#[test]
fn a_mzsus_and_a_csv_with_the_same_rows_give_the_same_candidates() {
    let csv = csv_keys(false);
    let a = run("{}", &[("list.csv", csv.as_bytes())], None, false);
    let b = run("{}", &[("list.mzsus", &mzsus())], None, false);
    let cands = |v: &Value| {
        hits(v)
            .iter()
            .map(|h| format!("{:?}", h.get("candidates")))
            .collect::<Vec<_>>()
    };
    assert_eq!(hits(&a).len(), 4);
    assert_eq!(cands(&a), cands(&b));
    // the memberships exist and are empty until a list carries the listings extension
    assert!(hits(&b)[0]
        .get("candidates")
        .unwrap()
        .at(0)
        .unwrap()
        .get("memberships")
        .unwrap()
        .arr()
        .is_empty());
    // the provenance of the file is in the header: name, SHA-256, kind and the manifest of the .mzsus
    let f = &b.get("suspect_files").unwrap().arr()[0];
    assert_eq!(f.get("kind").and_then(Value::str), Some("mzsus"));
    assert_eq!(f.get("sha256").and_then(Value::str).map(str::len), Some(64));
    assert_eq!(
        f.get("provenance")
            .unwrap()
            .get("attribution")
            .and_then(Value::str),
        Some("Invented test list")
    );
    assert_eq!(
        f.get("provenance")
            .unwrap()
            .get("sources")
            .unwrap()
            .at(0)
            .unwrap()
            .get("licence")
            .and_then(Value::str),
        Some("CC0")
    );
    assert!(a
        .get("suspect_files")
        .unwrap()
        .at(0)
        .unwrap()
        .get("provenance")
        .unwrap()
        .is_null());
}

#[test]
fn the_provenance_hash_follows_the_file() {
    let (a, b) = (
        csv_keys(true),
        csv_keys(true) + "Extra,IIIIIIIIIIIIII,C30H50O,,\n",
    );
    let sha = |t: &str| {
        run("{}", &[("list.csv", t.as_bytes())], None, false)
            .get("suspect_files")
            .unwrap()
            .at(0)
            .unwrap()
            .get("sha256")
            .and_then(Value::str)
            .unwrap()
            .to_string()
    };
    assert_ne!(sha(&a), sha(&b));
    assert_eq!(sha(&a), sha(&a));
}

#[test]
fn a_negative_table_searches_the_negative_adducts_only() {
    let list = csv_keys(true);
    let text = screen_text(
        &params("{}"),
        &table_bytes(-1),
        &[("list.csv", list.as_bytes())],
        None,
        &[],
    )
    .unwrap();
    let v = json::parse(text.as_bytes()).unwrap();
    assert_eq!(v.get("polarity").and_then(Value::f64), Some(-1.0));
    assert!(hits(&v).iter().all(|h| h
        .get("adduct")
        .and_then(Value::str)
        .is_some_and(|a| a.ends_with('-'))));
    assert!(screen_text(
        &params(r#"{"adducts":"[M+H]+"}"#),
        &table_bytes(-1),
        &[("list.csv", list.as_bytes())],
        None,
        &[]
    )
    .is_err());
}

#[test]
fn the_formula_groups_per_feature_are_capped_closest_first() {
    // at 100 ppm the two decoys (54 ppm away) are formula groups of the feature 1 beside Alpha
    let list = csv_keys(true);
    let open = run(
        r#"{"ppm":100}"#,
        &[("list.csv", list.as_bytes())],
        None,
        false,
    );
    let of_first = |v: &Value| {
        hits(v)
            .iter()
            .filter(|h| h.get("feature").and_then(Value::f64) == Some(1.0))
            .map(names)
            .collect::<Vec<_>>()
    };
    assert_eq!(of_first(&open).len(), 3);
    let capped = run(
        r#"{"ppm":100,"max_hits_per_feature":1}"#,
        &[("list.csv", list.as_bytes())],
        None,
        false,
    );
    assert_eq!(of_first(&capped), vec![vec!["Alpha".to_string()]]);
    assert_eq!(capped.get("dropped_groups").and_then(Value::f64), Some(2.0));
}
