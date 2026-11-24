//! The file names of `file convert --name sample`: a sample name as a safe file name, clashes settled in order (case aside) with a note each, and the route
//! `mzml_names` that the site and the app call giving the same names as the function the CLI calls. Synthetic names only.

use mzlab_core::samples::{clean_sample_name, mzml_names, mzml_names_route, NameItem};

fn item<'a>(
    who: &'a str,
    plain: &'a str,
    sample: Option<&'a str>,
    id: Option<u32>,
) -> NameItem<'a> {
    NameItem {
        who,
        plain,
        sample,
        id,
    }
}

#[test]
fn a_sample_name_becomes_a_safe_file_name() {
    for (raw, want) in [
        ("MQ_H2O2_T30", Some("MQ_H2O2_T30")),
        ("TW-PMS-D-T05", Some("TW-PMS-D-T05")),
        ("MQ H2O2 / T30", Some("MQ_H2O2_T30")),
        ("a:b*c?d\"e", Some("a_b_c_d_e")),
        ("..\\..\\evil", Some("evil")),
        ("  __x__  ", Some("x")),
        ("sample (2)", Some("sample_(2)")),
        ("///", None),
        ("", None),
    ] {
        assert_eq!(clean_sample_name(raw).as_deref(), want, "{raw:?}");
    }
}

#[test]
fn a_device_name_gets_its_underscore_right_after_the_reserved_part() {
    for (raw, want) in [
        ("CON", "CON_"),
        ("con", "con_"),
        ("CON.txt", "CON_.txt"),
        ("aux.d", "aux_.d"),
        ("COM1", "COM1_"),
        ("lpt9.x", "lpt9_.x"),
        // not devices: COM0, COM10, a longer word
        ("COM0", "COM0"),
        ("COM10", "COM10"),
        ("CONSOLE", "CONSOLE"),
    ] {
        assert_eq!(clean_sample_name(raw).as_deref(), Some(want), "{raw:?}");
    }
}

#[test]
fn the_length_is_capped_in_bytes_on_a_character() {
    assert_eq!(clean_sample_name(&"x".repeat(300)).unwrap().len(), 200);
    // 150 two-byte letters: 100 fit in 200 bytes, none is cut in two
    let long = "é".repeat(150);
    let n = clean_sample_name(&long).unwrap();
    assert_eq!(n, "é".repeat(100));
    assert!(n.len() <= 200);
}

#[test]
fn clashes_yield_to_the_earlier_file_case_aside_and_each_says_why() {
    let items = [
        item("a@1", "a-1.mzML", Some("Blank"), Some(1)),
        item("a@2", "a-2.mzML", Some("blank"), Some(2)),
        item("b@1", "b-1.mzML", Some("BLANK"), Some(1)),
        item("b@2", "b-2.mzML", Some("MQ_T30"), Some(2)),
        item("c", "c.mzML", None, None),
        item("d", "d.mzML", Some("///"), None),
        item("e", "e.mzML", Some("MQ T30"), None),
    ];
    let n = mzml_names(&items);
    let outs: Vec<&str> = n.iter().map(|x| x.out.as_str()).collect();
    // the second Blank gets its id; the third has the id of the first again, so a number follows; a dataset with one sample and a clash gets a number
    assert_eq!(
        outs,
        [
            "Blank.mzML",
            "blank-2.mzML",
            "BLANK-1.mzML",
            "MQ_T30.mzML",
            "c.mzML",
            "d.mzML",
            "MQ_T30-2.mzML"
        ]
    );
    let lower: Vec<String> = outs.iter().map(|o| o.to_lowercase()).collect();
    let unique: std::collections::HashSet<&String> = lower.iter().collect();
    assert_eq!(unique.len(), lower.len(), "no two names equal case aside");
    assert!(n[0].note.is_none() && n[3].note.is_none() && n[4].note.is_none());
    assert!(n[1].note.as_deref().unwrap().contains("a@2"));
    assert!(n[5]
        .note
        .as_deref()
        .unwrap()
        .contains("no usable characters"));
    assert!(n[6].note.as_deref().unwrap().contains("already taken"));
}

#[test]
fn the_route_gives_the_names_of_the_function() {
    let items = [
        item("a@1", "a-1.mzML", Some("Blank"), Some(1)),
        item("a@2", "a-2.mzML", Some("blank"), Some(2)),
        item("c", "c.mzML", Some("x y"), None),
    ];
    let want = mzml_names(&items);
    let input = r#"{"files":[{"who":"a@1","plain":"a-1.mzML","sample":"Blank","id":1},{"who":"a@2","plain":"a-2.mzML","sample":"blank","id":2},{"who":"c","plain":"c.mzML","sample":"x y"}]}"#;
    let got = mzml_names_route(input).unwrap();
    let v = mzlab_core::json::parse(got.as_bytes()).unwrap();
    let rows = v.get("files").unwrap().arr();
    assert_eq!(rows.len(), want.len());
    for (r, w) in rows.iter().zip(&want) {
        assert_eq!(r.get("out").unwrap().str(), Some(w.out.as_str()));
        assert_eq!(r.get("note").and_then(|x| x.str()), w.note.as_deref());
    }
    // the same bytes twice
    assert_eq!(got, mzml_names_route(input).unwrap());
}
