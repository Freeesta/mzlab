//! Tests of `explain-missing` (`flow_explain.rs`).

use super::*;

fn table() -> Aligned {
    let mut t = Aligned::with_capacity(2, 3);
    t.n_files = 2;
    // row 0: in view; row 1: same mass, wrong RT; row 2: a control row
    t.mz = vec![300.0, 400.0, 500.0];
    t.rt = vec![5.0, 9.0, 7.0];
    t.area = vec![10.0, 0.0, 5.0, 6.0, 7.0, 8.0];
    t.height = vec![1.0, 0.0, 2.0, 3.0, 4.0, 5.0];
    t.state = vec![
        CELL_DETECTED,
        3,
        CELL_DETECTED,
        CELL_FILLED,
        CELL_DETECTED,
        CELL_DETECTED,
    ];
    t.tp.status = vec![0, 0, 1];
    t.tp.kinship = vec![2, 3, 0];
    t
}

#[test]
fn stages_are_named_by_the_first_filter() {
    let e = read_expected("id,mz,rt\na,300.0005,5.1\nb,400.0,5.0\nc,500.0,7.0\nd,600.0,5.0\n");
    let r = report(&table(), &e, 5.0, 0.3);
    let col = |l: usize| {
        r.lines()
            .nth(l)
            .unwrap()
            .split(',')
            .nth(12)
            .unwrap()
            .to_string()
    };
    assert_eq!(
        [col(1), col(2), col(3), col(4)],
        ["view", "rt_off", "F3", "no_row_mz"]
    );
    let n = r.lines().next().unwrap().split(',').count();
    assert!(r.lines().all(|l| l.split(',').count() == n));
    // rank_now of the first expected product: kinship 3 outranks kinship 2
    assert_eq!(r.lines().nth(1).unwrap().split(',').nth(18), Some("2"));
    // the first product is alone in its kinship group, with one single-file row above it; the whole view was seen in one file
    let l1: Vec<&str> = r.lines().nth(1).unwrap().split(',').collect();
    assert_eq!(l1[24..27], ["1", "1", "1"]);
    assert_eq!(l1[27], "100.0");
}

#[test]
fn a_family_member_is_absorbed_not_missing() {
    let mut t = table();
    // row 2 (500.0 @ 7.0) holds an ion at 450.0 @ 6.0 as a member
    t.member_start = vec![0, 1, 2, 4];
    t.member_mz = vec![300.0, 400.0, 500.0, 450.0];
    t.member_rt = vec![5.0, 9.0, 7.0, 6.0];
    t.member_role = vec![0; 4];
    let e = read_expected("id,mz,rt\na,450.0,6.1\nb,600.0,5.0\n");
    let r = report(&t, &e, 5.0, 0.3);
    let l: Vec<Vec<&str>> = r.lines().skip(1).map(|l| l.split(',').collect()).collect();
    assert_eq!(l[0][12], "absorbed");
    assert_eq!(l[0][28..], ["2", "500.0000", "7.00"]);
    assert_eq!(l[1][12], "no_row_mz");
    assert_eq!(l[1][28..], ["", "", ""]);
}

/// `--json` is the report as one document: the same cells as the CSV, and an id with a comma, a quote or a space keeps its one cell.
#[test]
fn json_is_the_report_as_one_document() {
    let ids = ["p 1", "a,b", "say \"x\""];
    let e: Vec<(String, f64, f64)> = ids.iter().map(|i| (i.to_string(), 300.0005, 5.1)).collect();
    let rows = rows(&table(), &e, 5.0, 0.3);
    let text = json_of(&rows);
    assert!(
        text.starts_with("{\"schema\":\"explain-missing/1\",\"columns\":[\"id\",\"mz\""),
        "{text}"
    );
    assert!(text.ends_with("}\n") && !text.ends_with("\n\n"));
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let n = v["columns"].as_array().unwrap().len();
    let got = v["rows"].as_array().unwrap();
    assert_eq!(got.len(), ids.len());
    for (row, id) in got.iter().zip(ids) {
        assert_eq!(row.as_array().unwrap().len(), n, "{row}");
        assert_eq!(row[0], id);
    }
    // the CSV quotes what needs it and has the same cell count once parsed back
    let csv = report(&table(), &e, 5.0, 0.3);
    assert!(csv.contains("\"a,b\",300.0005,"), "{csv}");
    assert!(csv.contains("\"say \"\"x\"\"\",300.0005,"), "{csv}");
    assert_eq!(csv.lines().count(), 1 + ids.len());
}

/// `execute(["explain-missing", T, E, "--json"])` end to end on a real `.mzfeat`: stdout and `--out` carry the same document, one trailing newline.
#[test]
fn explain_missing_json_runs_end_to_end() {
    let dir = std::env::temp_dir().join(format!("mzlab-explainjson-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let table_path = dir.join("t.mzfeat");
    struct Texts;
    impl mzlab_feat::store::RecipeTexts for Texts {
        fn file_text(&self) -> String {
            "{}".into()
        }
        fn table_text(&self) -> String {
            "{}".into()
        }
    }
    let bytes =
        mzlab_feat::store::write_table(&table(), &Texts, &mzlab_feat::store::Origin::default());
    std::fs::write(&table_path, bytes).unwrap();
    let expected = dir.join("e.csv");
    std::fs::write(&expected, "id,mz,rt\np1,300.0005,5.1\nb,400.0,5.0\n").unwrap();
    let run = |extra: &[&str]| {
        let mut a: Vec<String> = [
            "explain-missing",
            table_path.to_str().unwrap(),
            expected.to_str().unwrap(),
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        a.extend(extra.iter().map(|s| s.to_string()));
        crate::execute(&a)
    };
    let o = run(&["--json"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(
        text.starts_with("{\"schema\":\"explain-missing/1\""),
        "{text}"
    );
    assert!(text.ends_with("}\n") && !text.ends_with("\n\n"));
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["rows"].as_array().unwrap().len(), 2);
    let out = dir.join("r.json");
    let o = run(&["--json", "--out", out.to_str().unwrap()]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stdout.is_empty());
    assert_eq!(std::fs::read_to_string(&out).unwrap(), text);
    // without --json: the CSV, one line more than the products
    let csv = String::from_utf8(run(&[]).stdout).unwrap();
    assert_eq!(csv.lines().count(), 3);
    std::fs::remove_dir_all(dir).ok();
}
