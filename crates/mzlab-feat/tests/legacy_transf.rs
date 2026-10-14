//! The list of transformations of mzFinder in Rust (`legacy::transf`) against the frozen Python answers (`tools/finder_transf_riferimento.py --golden`):
//! the rows, the rules that the list adds to `TP_Mine/data/regole_tp.json`, the `lit:` references and the error texts.

use mzlab_feat::hypo::Rules;
use mzlab_feat::legacy::transf;
use serde_json::Value;

fn counts(v: &Value) -> Vec<(String, i64)> {
    v.as_object()
        .unwrap()
        .iter()
        .map(|(k, n)| (k.clone(), n.as_i64().unwrap()))
        .collect()
}

fn flat(c: &mzlab_feat::hypo::Counts) -> Vec<(String, i64)> {
    c.iter().map(|(k, n)| (k.clone(), *n)).collect()
}

#[test]
fn transformations_match_python() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let golden: Value = serde_json::from_slice(
        &std::fs::read(root.join("tests/golden/finder_transf.json")).unwrap(),
    )
    .unwrap();
    let base = std::fs::read(root.join("TP_Mine/data/regole_tp.json")).unwrap();
    let n_base = Rules::parse(&base).unwrap().rules.len();
    for case in golden["cases"].as_array().unwrap() {
        let (name, text) = (
            case["name"].as_str().unwrap(),
            case["text"].as_str().unwrap(),
        );
        if let Some(e) = case.get("error") {
            assert_eq!(
                transf::merge(&base, text).unwrap_err(),
                e.as_str().unwrap(),
                "{name}"
            );
            continue;
        }
        let rows = transf::parse(text).unwrap();
        let want = case["rows"].as_array().unwrap();
        assert_eq!(rows.len(), want.len(), "{name}: rows");
        for (r, w) in rows.iter().zip(want) {
            assert_eq!(r.name, w["name"].as_str().unwrap(), "{name}");
            assert_eq!(r.delta, counts(&w["delta"]), "{name}: {}", r.name);
            assert_eq!(r.comment, w["comment"].as_str().unwrap(), "{name}");
        }
        let merged = Rules::parse(&transf::merge(&base, text).unwrap())
            .unwrap()
            .rules;
        let new = &merged[n_base..];
        let want = case["new"].as_array().unwrap();
        assert_eq!(new.len(), want.len(), "{name}: new rules");
        for (r, w) in new.iter().zip(want) {
            assert_eq!(r.id, w["name"].as_str().unwrap(), "{name}");
            assert_eq!(flat(&r.delta), counts(&w["delta"]), "{name}: {}", r.id);
        }
        let lit: Vec<_> = merged.iter().filter(|r| r.level == 2).collect();
        let want = case["literature"].as_array().unwrap();
        assert_eq!(lit.len(), want.len(), "{name}: literature");
        for w in want {
            let d = counts(&w["delta"]);
            let r = merged.iter().find(|r| flat(&r.delta) == d).unwrap();
            assert_eq!(
                r.source,
                format!("lit:{}", w["source"].as_str().unwrap()),
                "{name}"
            );
        }
    }
}

#[test]
fn describe_rows_for_the_table() {
    let t = "# header\nhydroxylation;+O;one oxygen\n#loss of CO;-CO\n# just a note\nbad;+Zz9\nx;-Cl+H;lit: Doe 2020\n\n#nonsense;";
    let v: Value = serde_json::from_str(&transf::describe(t)).unwrap();
    let rows = v["rows"].as_array().unwrap();
    let key = |r: &Value| (r["line"].as_i64().unwrap(), r["on"].as_bool().unwrap());
    assert_eq!(
        rows.iter().map(key).collect::<Vec<_>>(),
        [(1, true), (2, false), (4, true), (5, true)]
    );
    assert_eq!(rows[0]["change"], "+O");
    assert_eq!(rows[0]["class"], "oxidation");
    assert!((rows[0]["dm"].as_f64().unwrap() - 15.9949).abs() < 1e-4);
    assert_eq!(rows[1]["class"], "loss");
    assert!(rows[2]["error"].is_string());
    assert_eq!(rows[3]["class"], "halogen");
    assert_eq!(rows[3]["lit"], true);
}
