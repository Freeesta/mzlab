//! Unit tests of the label rule (`sample_label.rs`).

use super::*;

fn m<'a>(file: &'a str, id: u32, sample: &'a str) -> Member<'a> {
    Member { file, id, sample }
}

#[test]
fn a_sample_name_is_the_label_and_the_file_keeps_only_its_extension() {
    let l = labels(&[m("run.wiff", 1, "Sample_A"), m("run.wiff", 2, "Sample 1.5")]);
    assert_eq!(l[0], ("Sample_A.wiff".to_string(), Kind::Own));
    assert_eq!(l[1], ("Sample 1.5.wiff".to_string(), Kind::Own));
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
    assert!(l.iter().all(|(_, k)| *k == Kind::Tie));
    let l = labels(&[m("a.wiff", 1, "S"), m("b.wiff", 1, "U")]);
    assert_eq!(
        l,
        [
            ("S.wiff".to_string(), Kind::Own),
            ("U.wiff".to_string(), Kind::Own)
        ]
    );
}

#[test]
fn one_file_twice_is_numbered_so_no_name_repeats() {
    let l = labels(&[m("a.wiff", 1, "S"), m("a.wiff", 1, "S")]);
    assert_eq!(l[0].0, "S (a #1).wiff");
    assert_eq!(l[1].0, "S (a #1) [2].wiff");
    assert_eq!((l[0].1, l[1].1), (Kind::Tie, Kind::Numbered));
}

#[test]
fn an_empty_name_falls_back_to_the_file_and_the_id() {
    let l = labels(&[m("a.wiff", 3, " ")]);
    assert_eq!(l[0].0, "a #3.wiff");
}

#[test]
fn separators_become_underscores_and_an_at_sign_survives_the_member_label() {
    let l = labels(&[
        m("a.wiff", 1, "x/y"),
        m("a.wiff", 2, "p\\q r"),
        m("a.wiff", 3, "c@5"),
    ]);
    let names: Vec<&str> = l.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["x_y.wiff", "p_q r.wiff", "c@5.wiff"]);
    // the label of a member of the plan is `COLUMN@ID`: the last `@` followed by the id splits it, whatever the name holds
    for (k, (column, _)) in l.iter().enumerate() {
        let member = format!("{column}@{}", k + 1);
        assert_eq!(
            crate::flow_set::split_sample(&member),
            (column.as_str(), Some(k as u32 + 1))
        );
    }
}
