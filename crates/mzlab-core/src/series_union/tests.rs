//! Tests of `union` on small synthetic feature tables (the CSV of `mzlab-cli features`).

use crate::json::{parse, Value};

/// A series table: `shared` ions at m/z 100, 110, ... and RT 1.0, 1.5, ... plus `own` ions from m/z `own_mz`, all shifted by `shift` minutes.
fn table(shared: usize, own: usize, own_mz: f64, shift: f64) -> String {
    let mut t = String::from("id,mz,rt,rt_range:min,rt_range:max,charge,datafile:a:area\n");
    let rows = (0..shared)
        .map(|i| (100.0 + 10.0 * i as f64, 1.0 + 0.5 * i as f64))
        .chain((0..own).map(|i| (own_mz + 10.0 * i as f64, 2.0 + 0.5 * i as f64)));
    for (k, (mz, rt)) in rows.enumerate() {
        let rt = rt + shift;
        t += &format!(
            "{},{mz},{rt:.4},{:.4},{:.4},1,5\n",
            k + 1,
            rt - 0.1,
            rt + 0.1
        );
    }
    t
}

fn request(tables: &[(&str, String)], params: &str) -> String {
    let t: Vec<String> = tables
        .iter()
        .map(|(n, x)| {
            format!(
                "{{\"name\":{},\"text\":{}}}",
                crate::jsonb::write(&Value::Str(n.to_string())),
                crate::jsonb::write(&Value::Str(x.clone()))
            )
        })
        .collect();
    format!("{{\"tables\":[{}],\"params\":{params}}}", t.join(","))
}

fn run(tables: &[(&str, String)], params: &str) -> Value {
    parse(super::route(&request(tables, params)).unwrap().as_bytes()).unwrap()
}

fn count(v: &Value, key: &str) -> usize {
    v.get("counts").unwrap().get(key).unwrap().f64().unwrap() as usize
}

#[test]
fn two_series_with_ten_shared_ions_and_three_own_give_sixteen_targets() {
    let v = run(
        &[
            ("s1", table(10, 3, 300.0, 0.0)),
            ("s2", table(10, 3, 400.0, 0.0)),
        ],
        "{}",
    );
    assert_eq!(
        (
            count(&v, "rows_in"),
            count(&v, "targets"),
            count(&v, "merged")
        ),
        (26, 16, 10)
    );
    let merged: Vec<&Value> = v
        .get("targets")
        .unwrap()
        .arr()
        .iter()
        .filter(|t| t.get("series").unwrap().f64() == Some(2.0))
        .collect();
    assert_eq!(merged.len(), 10);
    for t in merged {
        assert_eq!(
            t.get("members").unwrap().arr().len(),
            2,
            "every merge lists both rows"
        );
    }
    let csv = v.get("csv").unwrap().str().unwrap();
    assert!(
        csv.starts_with("id,mz,rt_min,rt_max,polarity,role\nU0001,100,0.9,1.1,,target\n"),
        "{csv}"
    );
    assert_eq!(csv.lines().count(), 17);
}

#[test]
fn a_shift_between_series_merges_with_piecewise_and_not_with_off() {
    let tables = [
        ("s1", table(10, 3, 300.0, 0.0)),
        ("s2", table(10, 3, 400.0, 0.2)),
    ];
    let off = run(&tables, "{}");
    assert_eq!(count(&off, "targets"), 26);
    let pw = run(&tables, r#"{"rt_warp":"piecewise"}"#);
    assert_eq!(count(&pw, "targets"), 16);
    let s2 = &pw.get("series").unwrap().arr()[1];
    let w = s2.get("warp").unwrap();
    assert_eq!(w.get("applied"), Some(&Value::Bool(true)));
    assert!((w.get("median_shift").unwrap().f64().unwrap() - 0.2).abs() < 1e-3);
    let what: Vec<&str> = pw
        .get("choices")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|c| c.get("what").and_then(Value::str))
        .collect();
    assert!(what.contains(&"rt_warp"), "{what:?}");
    // the window covers the rows in the RT frame of each series
    let csv = pw.get("csv").unwrap().str().unwrap();
    assert!(csv.contains("U0001,100,0.9,1.3,,target"), "{csv}");
}

#[test]
fn too_few_anchors_leave_the_series_as_it_is_and_say_so() {
    let tables = [("s1", table(2, 0, 0.0, 0.0)), ("s2", table(2, 0, 0.0, 0.2))];
    let v = run(&tables, r#"{"rt_warp":"piecewise"}"#);
    assert_eq!(count(&v, "targets"), 4);
    let what: Vec<&str> = v
        .get("choices")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|c| c.get("what").and_then(Value::str))
        .collect();
    assert!(what.contains(&"rt_warp_skipped"), "{what:?}");
}

#[test]
fn rows_of_one_series_are_never_merged_and_a_close_pair_is_a_conflict() {
    let close = "id,mz,rt\na,200.0,3.00\nb,200.2,3.05\n".to_string();
    let v = run(&[("s1", close)], "{}");
    assert_eq!((count(&v, "targets"), count(&v, "conflicts")), (2, 1));
    let c = &v.get("conflicts").unwrap().arr()[0];
    assert_eq!(
        (c.get("a").unwrap().str(), c.get("b").unwrap().str()),
        (Some("U0001"), Some("U0002"))
    );
}

#[test]
fn min_series_lists_what_it_drops() {
    let v = run(
        &[
            ("s1", table(3, 1, 300.0, 0.0)),
            ("s2", table(3, 0, 0.0, 0.0)),
        ],
        r#"{"min_series":2}"#,
    );
    assert_eq!((count(&v, "targets"), count(&v, "dropped")), (3, 1));
    assert_eq!(
        v.get("dropped").unwrap().arr()[0]
            .get("reason")
            .unwrap()
            .str(),
        Some("min_series")
    );
}

#[test]
fn polarity_comes_from_the_table_or_the_parameter_and_rows_stay_apart() {
    let own = "id,mz,rt,polarity\na,200,3,-\nb,200,3,\n".to_string();
    let v = run(&[("s1", own)], r#"{"polarity":"+"}"#);
    assert_eq!(count(&v, "targets"), 2);
    let csv = v.get("csv").unwrap().str().unwrap();
    assert!(
        csv.contains("U0001,200,2.9,3.1,-,target") && csv.contains("U0002,200,2.9,3.1,+,target"),
        "{csv}"
    );
}

#[test]
fn polarity_any_is_every_scan_like_no_polarity_and_a_wrong_word_is_refused() {
    let t = [("s1", "id,mz,rt\na,200,3\n".to_string())];
    let none = run(&t, "{}");
    for any in [r#"{"polarity":"any"}"#, r#"{"polarity":""}"#] {
        assert_eq!(run(&t, any).get("csv"), none.get("csv"), "{any}");
    }
    let bad = super::route(&format!(
        r#"{{"tables":[{{"name":"s1","text":"id,mz,rt\na,200,3\n"}}],"params":{{"polarity":"both"}}}}"#
    ));
    assert_eq!(bad.unwrap_err().key, "err.union.param");
}

#[test]
fn the_same_input_gives_the_same_bytes() {
    let t = [
        ("s1", table(10, 3, 300.0, 0.0)),
        ("s2", table(10, 3, 400.0, 0.2)),
    ];
    let p = r#"{"rt_warp":"piecewise"}"#;
    assert_eq!(
        super::route(&request(&t, p)).unwrap(),
        super::route(&request(&t, p)).unwrap()
    );
}

#[test]
fn bad_input_has_a_key_and_the_place() {
    let key = |r: String| {
        super::route(&r)
            .err()
            .map(|e| (e.key.to_string(), e.params.clone()))
    };
    let bad_mz = key(request(
        &[("s1", "id,mz,rt\na,100,1\nb,x,2\n".into())],
        "{}",
    ))
    .unwrap();
    assert_eq!(bad_mz.0, "err.union.table");
    assert!(
        format!("{:?}", bad_mz.1).contains("bad_mz") && format!("{:?}", bad_mz.1).contains('3')
    );
    let no_col = key(request(&[("s1", "id,rt\na,1\n".into())], "{}")).unwrap();
    assert!(format!("{:?}", no_col.1).contains("no_column_mz"));
    assert_eq!(
        key(request(&[("s1", table(1, 0, 0.0, 0.0))], r#"{"da":9}"#))
            .unwrap()
            .0,
        "err.union.param"
    );
    assert_eq!(
        key(request(&[("s1", table(1, 0, 0.0, 0.0))], r#"{"nope":1}"#))
            .unwrap()
            .0,
        "err.union.param"
    );
    assert_eq!(key("{\"tables\":[]}".into()).unwrap().0, "err.union.empty");
    assert_eq!(key("nope".into()).unwrap().0, "err.union.request");
}
