//! `samples/1`: a plain list as a table, canonical bytes under permutations, `validate` (columns, keys, pairs), CSV both ways, the name model
//! (regex and template, spans, the generated names) and `from_file`; schema check.

mod csv;
mod import;
mod import_edges;
mod model;
mod series;

use mzlab_core::samples::{
    self, from_json, per_tp, table_from_json, table_to_json, to_json, validate, Cell, Column,
    ColumnType, Table,
};
use mzlab_core::samples::{propose, Sample};
use serde_json::Value;

pub fn many(names: &[&str]) -> Vec<Sample> {
    let v: Vec<(String, String, String)> = names
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    propose(&v)
}

fn codes(t: &Table) -> Vec<String> {
    let j = samples::route("validate", &table_to_json(t)).unwrap();
    let v: Value = serde_json::from_str(&j).unwrap();
    v["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| match a["column"].as_str() {
            Some(c) => format!("{}:{c}", a["code"].as_str().unwrap()),
            None => a["code"].as_str().unwrap().to_string(),
        })
        .collect()
}

fn table(names: &[&str]) -> Table {
    Table {
        samples: many(names),
        columns: Vec::new(),
    }
}

#[test]
fn a_plain_list_is_a_table_without_columns() {
    let cs = many(&[
        "X_t000min",
        "X_t005min",
        "X_t010min",
        "bianco_solv",
        "X_dark",
        "std_5ppm",
    ]);
    let doc = to_json(&cs);
    assert!(
        doc.starts_with("{\"schema\":\"samples/1\",\"columns\":[],"),
        "{doc}"
    );
    let t = table_from_json(doc.as_bytes()).unwrap();
    assert_eq!(t.samples, cs);
    assert_eq!(
        per_tp(&t.samples),
        per_tp(&from_json(doc.as_bytes()).unwrap())
    );
    // the canonical form of a document is itself, and there is one schema: another one is refused
    assert_eq!(samples::route("canonical", &doc).unwrap(), doc);
    assert_eq!(samples::route("digest", &doc).unwrap().len(), 64);
    let other = doc.replace("samples/1", "samples/9");
    assert_eq!(
        samples::route("canonical", &other).unwrap_err().key,
        "error.badSamples"
    );
    assert!(samples::route("migrate", &doc).is_err());
}

fn rich() -> Table {
    let mut t = table(&["A1_pos_t005", "A1_neg_t005", "B2_pos_t005", "B2_neg_t005"]);
    for (i, c) in t.samples.iter_mut().enumerate() {
        c.sample_id = ["A1", "A1", "B2", "B2"][i].into();
        c.polarity = Some(if i % 2 == 0 { 1 } else { -1 });
        c.matrix = "water".into();
        c.batch = "2".into();
        c.group = "g".into();
        c.series = "S".into();
        c.acquired = Some(format!("2025-03-0{}T10:00:00", i + 1));
        c.set_extra("zeta", Cell::Number(i as f64 + 0.5));
        c.set_extra("alfa", Cell::Text(format!("v{i}")));
    }
    t.columns = vec![
        Column::new("zeta", ColumnType::Number),
        Column {
            name: "alfa".into(),
            kind: ColumnType::Category,
            values: Some(vec!["v3".into(), "v0".into(), "v1".into(), "v2".into()]),
            required: true,
        },
    ];
    t
}

fn permutations(n: usize) -> Vec<Vec<usize>> {
    if n == 1 {
        return vec![vec![0]];
    }
    let mut out = Vec::new();
    for p in permutations(n - 1) {
        for k in 0..=p.len() {
            let mut q = p.clone();
            q.insert(k, n - 1);
            out.push(q);
        }
    }
    out
}

#[test]
fn canonical_bytes_do_not_depend_on_the_order_written() {
    let t = rich();
    let want = table_to_json(&t);
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/schemi/samples-1.json"
        ))
        .unwrap(),
    )
    .unwrap();
    assert!(super::fits(&serde_json::from_str(&want).unwrap(), &schema));
    assert_eq!(table_from_json(want.as_bytes()).unwrap().samples.len(), 4);
    for p in permutations(4) {
        for q in [vec![0, 1], vec![1, 0]] {
            let mut u = t.clone();
            u.samples = p.iter().map(|&i| t.samples[i].clone()).collect();
            u.columns = q.iter().map(|&i| t.columns[i].clone()).collect();
            u.samples.iter_mut().for_each(|c| c.extra.reverse());
            assert_eq!(table_to_json(&u), want);
        }
    }
    assert_eq!(
        table_from_json(want.as_bytes()).unwrap().samples[3].extra[0].0,
        "alfa"
    );
    assert_eq!(
        samples::table_digest(&t),
        samples::table_digest(&table_from_json(want.as_bytes()).unwrap())
    );
}

#[test]
fn valida_checks_columns_keys_and_pairs() {
    let mut t = rich();
    assert!(codes(&t)
        .iter()
        .all(|c| !c.contains(':') && c != "duplicateKey" && !c.starts_with("pair")));
    // a required column without a value, a number in a text column, a value outside the category, a text in a number column
    t.samples[0].extra.retain(|(k, _)| k != "alfa");
    t.samples[1].set_extra("alfa", Cell::Number(1.0));
    t.samples[2].set_extra("alfa", Cell::Text("nope".into()));
    t.samples[3].set_extra("zeta", Cell::Text("x".into()));
    let c = codes(&t);
    assert_eq!(c.iter().filter(|x| *x == "missingValue:alfa").count(), 1);
    assert_eq!(c.iter().filter(|x| *x == "typeMismatch:alfa").count(), 2);
    assert_eq!(c.iter().filter(|x| *x == "typeMismatch:zeta").count(), 1);
    // keys
    let mut d = rich();
    d.samples[1].file = d.samples[0].file.clone();
    assert!(codes(&d).contains(&"duplicateKey".into()));
    let mut s = rich();
    s.samples[2].sha256 = "ab".into();
    s.samples[3].sha256 = "ab".into();
    assert!(codes(&s).contains(&"duplicateKey".into()));
    // pairs
    let mut m = rich();
    m.samples[3].polarity = Some(1);
    assert!(codes(&m).contains(&"pairMissing".into()));
    let mut x = rich();
    x.samples[1].series = "other".into();
    assert!(codes(&x).contains(&"pairMismatch".into()));
    let mut y = rich();
    y.samples[3].set_time(Some(30.0), "min");
    assert!(codes(&y).contains(&"pairMismatch".into()));
    let mut z = rich();
    z.samples[3].kind = "qc".into();
    assert!(codes(&z).contains(&"pairMismatch".into()));
    // a plain list is checked like a table without columns
    let doc = to_json(&many(&["X_t000min", "X_t005min"]));
    assert_eq!(
        samples::route("validate", &doc).unwrap(),
        samples::route("validate", &samples::route("canonical", &doc).unwrap()).unwrap()
    );
    assert!(!validate(&many(&["X_t000min", "X_t005min"])).is_empty());
}

#[test]
fn from_file_fills_what_the_file_says() {
    let mut t = rich();
    t.samples[0].sha256 = "aa".into();
    t.samples[0].polarity = Some(-1); // the name was wrong
    t.samples[1].polarity = None;
    let input = format!(
        r#"{{"samples":{},"file":[
        {{"sha256":"aa","polarity":[1,1],"rt_min":0.1,"rt_max":9.9,"mz_min":50,"mz_max":600,"scans":420,"acquired":"2025-04-01 08:30:15.123","instrument":"QTRAP 3200","method":"m1.dam"}},
        {{"name":"{}","polarity":[-1],"method_json":{{"origin":{{"file_created":"2025-04-01T09:00:00"}},"instrument":{{"model":"3200 QTRAP"}},"sample":{{"method_file":"neg.dam"}},"ms":{{"periods":[{{"experiments":[{{"mz":[100,900]}},{{"mz":[80,400]}}]}}]}}}}}},
        {{"name":"unknown.mzML","polarity":[1]}}]}}"#,
        table_to_json(&t),
        t.samples[1].file
    );
    let out: Value =
        serde_json::from_str(&samples::route_all("from_file", &input).unwrap()).unwrap();
    let c = &out["samples"]["samples"];
    let by = |f: &str| {
        c.as_array()
            .unwrap()
            .iter()
            .find(|x| x["file"] == f)
            .unwrap()
    };
    let a = by(&t.samples[0].file);
    assert_eq!(
        (a["polarity"].clone(), a["acquired"].as_str()),
        (1.into(), Some("2025-04-01T08:30:15"))
    );
    assert_eq!(a["file_data"]["scans"], 420);
    assert_eq!(a["file_data"]["mz_max"], 600);
    assert_eq!(a["file_data"]["instrument"], "QTRAP 3200");
    let b = by(&t.samples[1].file);
    assert_eq!(
        (b["polarity"].clone(), b["acquired"].as_str()),
        ((-1).into(), Some("2025-04-01T09:00:00"))
    );
    assert_eq!(
        (
            b["file_data"]["mz_min"].clone(),
            b["file_data"]["mz_max"].clone()
        ),
        (80.into(), 900.into())
    );
    assert_eq!(b["file_data"]["method"], "neg.dam");
    assert_eq!(out["unmatched"], serde_json::json!([2]));
    // only the replaced values are reported
    let d = out["mismatches"].as_array().unwrap();
    assert!(d
        .iter()
        .any(|x| x["field"] == "polarity" && x["before"] == "-1" && x["after"] == "1"));
    assert!(d
        .iter()
        .all(|x| x["index"] != 1 || x["field"] == "acquired"));
    // the file_data survive the canonical form
    let again = samples::route(
        "canonical",
        &serde_json::to_string(&out["samples"]).unwrap(),
    )
    .unwrap();
    assert!(again.contains("\"scans\":420"));
}

#[test]
fn errors_are_json_keys_not_panics() {
    for (op, input) in [
        ("csv_read", "{}"),
        ("csv_read", r#"{"text":"a,b\n\"x"}"#),
        ("csv_write", "{}"),
        ("model", "{}"),
        ("model", r#"{"model":"(?P<a>","files":[]}"#),
        ("from_file", "{}"),
        ("canonical", r#"{"schema":"samples/9"}"#),
    ] {
        assert!(samples::route_all(op, input).is_err(), "{op} {input}");
    }
}
