//! Tests of `order_like` (`ordine.rs`).
use super::order_like;
use crate::json::parse;

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

const DOC: &str = r#"{"schema":"campioni/2","campioni":[{"file":"T10p25.mzML","tipo":"sample"},{"file":"T2p5.mzML","tipo":"sample"},{"file":"blank.mzML","tipo":"blank"}]}"#;

#[test]
fn the_files_follow_the_names_of_the_table() {
    let got = order_like(DOC, &names(&["blank.mzidx", "T2p5", "dir/T10p25.mzML"])).unwrap();
    let files: Vec<String> = parse(got.as_bytes())
        .unwrap()
        .get("campioni")
        .unwrap()
        .arr()
        .iter()
        .map(|v| v.get("file").unwrap().str().unwrap().to_string())
        .collect();
    assert_eq!(files, ["blank.mzML", "T2p5.mzML", "T10p25.mzML"]);
    assert!(got.contains(r#"{"file":"blank.mzML","tipo":"blank"}"#));
}

#[test]
fn the_same_order_or_no_names_keep_the_bytes() {
    let same = names(&["T10p25.mzML", "T2p5.mzML", "blank.mzML"]);
    assert_eq!(order_like(DOC, &same).unwrap(), DOC);
    assert_eq!(order_like(DOC, &[]).unwrap(), DOC);
    assert_eq!(order_like(DOC, &names(&["a", "", "b"])).unwrap(), DOC);
}

#[test]
fn an_exact_name_wins_over_a_longer_one() {
    let doc = r#"{"schema":"campioni/2","campioni":[{"file":"S1.5.mzML"},{"file":"S1.mzML"}]}"#;
    let got = order_like(doc, &names(&["S1.mzML", "S1.5.mzML"])).unwrap();
    assert!(got.find("\"S1.mzML\"").unwrap() < got.find("\"S1.5.mzML\"").unwrap());
}

#[test]
fn a_missing_or_a_foreign_file_is_an_error_with_its_name() {
    let e = order_like(DOC, &names(&["T2p5", "T10p25", "T5"])).unwrap_err();
    assert_eq!(e.key, "error.badCampioni");
    assert!(format!("{:?}", e.params).contains("T5"), "{:?}", e.params);
    let e = order_like(DOC, &names(&["T2p5", "T10p25"])).unwrap_err();
    assert!(
        format!("{:?}", e.params).contains("blank.mzML"),
        "{:?}",
        e.params
    );
    assert_eq!(
        order_like("{}", &names(&["a"])).unwrap_err().key,
        "error.badCampioni"
    );
}
