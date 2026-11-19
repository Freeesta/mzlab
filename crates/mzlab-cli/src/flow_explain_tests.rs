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

/// `--json` is the CSV as one document: the same cells, and an id with spaces and quotes kept whole.
#[test]
fn json_is_the_csv_as_one_document() {
    let e = read_expected("id,mz,rt\n\"p 1\",300.0005,5.1\nb,400.0,5.0\n");
    let csv = report(&table(), &e, 5.0, 0.3);
    let text = json_of(&csv);
    assert!(
        text.starts_with("{\"schema\":\"explain-missing/1\",\"columns\":[\"id\",\"mz\""),
        "{text}"
    );
    assert!(text.ends_with("}\n") && !text.ends_with("\n\n"));
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let want: Vec<Vec<&str>> = csv
        .lines()
        .skip(1)
        .map(|l| l.split(',').collect())
        .collect();
    let rows = v["rows"].as_array().unwrap();
    assert_eq!(rows.len(), want.len());
    for (got, want) in rows.iter().zip(&want) {
        let got: Vec<&str> = got
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_str().unwrap())
            .collect();
        assert_eq!(&got, want);
    }
    assert_eq!(rows[0][0], "\"p 1\"");
    assert_eq!(
        v["columns"].as_array().unwrap().len(),
        rows[0].as_array().unwrap().len()
    );
}
