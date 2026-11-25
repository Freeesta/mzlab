//! Unit tests of the label rule (`sample_label.rs`).

use super::*;

fn m<'a>(file: &'a str, id: u32, sample: &'a str) -> Member<'a> {
    Member { file, id, sample }
}

#[test]
fn a_sample_name_is_the_label_and_the_file_keeps_only_its_extension() {
    let l = labels(&[m("run.wiff", 1, "Sample_A"), m("run.wiff", 2, "Sample 1.5")]);
    assert_eq!(l[0], ("Sample_A.wiff".to_string(), false));
    assert_eq!(l[1], ("Sample 1.5.wiff".to_string(), false));
}

#[test]
fn names_that_collide_take_the_file_and_the_id_and_the_others_do_not() {
    let l = labels(&[
        m("a.wiff", 1, "S"),
        m("b.wiff", 1, "S"),
        m("a.wiff", 2, "T"),
        m("a.wiff", 3, "T"),
    ]);
    let names: Vec<&str> = l.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        [
            "S (a #1).wiff",
            "S (b #1).wiff",
            "T (a #2).wiff",
            "T (a #3).wiff"
        ]
    );
    assert!(l.iter().all(|(_, tie)| *tie));
    let l = labels(&[m("a.wiff", 1, "S"), m("b.wiff", 1, "U")]);
    assert_eq!(
        l,
        [("S.wiff".to_string(), false), ("U.wiff".to_string(), false)]
    );
}

#[test]
fn one_file_twice_is_numbered_so_no_name_repeats() {
    let l = labels(&[m("a.wiff", 1, "S"), m("a.wiff", 1, "S")]);
    assert_eq!(l[0].0, "S (a #1).wiff");
    assert_eq!(l[1].0, "S (a #1) [2].wiff");
}

#[test]
fn an_empty_name_falls_back_to_the_file_and_the_id() {
    let l = labels(&[m("a.wiff", 3, " ")]);
    assert_eq!(l[0].0, "a #3.wiff");
}
