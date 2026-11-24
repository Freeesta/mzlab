//! The targets table and the parameters: both forms of the table, the columns told by name, every refusal with its line, the blank globs.

use super::params::{OffsetMode, Params};
use super::targets::{parse, Role};
use mzlab_core::json::Value;

#[test]
fn csv_and_json_tables_give_the_same_targets() {
    let csv = "\u{feff}# list\nName;m/z;rt_max;rt_min;Polarity;Role\na;200.5;6;4;neg;\nb;300;2;1;;standard\n";
    let json = r#"{"targets":[{"id":"a","mz":200.5,"rt_min":4,"rt_max":6,"polarity":"-"},{"name":"b","mz":300,"rt_min":1,"rt_max":2,"role":"parent"}]}"#;
    let a = parse(csv.as_bytes()).unwrap();
    let b = parse(json.as_bytes()).unwrap();
    // the same targets; each keeps where it came from: its file line (the comment is line 1, the header line 2) or its JSON item
    assert_eq!((a[0].line, a[1].line, b[0].line, b[1].line), (3, 4, 1, 2));
    let unlined = |v: Vec<super::targets::Target>| {
        v.into_iter()
            .map(|t| super::targets::Target { line: 0, ..t })
            .collect::<Vec<_>>()
    };
    assert_eq!(unlined(a.clone()), unlined(b));
    assert_eq!(
        (a[0].polarity, a[1].polarity, a[1].role),
        (Some(-1), None, Role::Reference)
    );
}

#[test]
fn wrong_rows_are_refused_with_line_and_reason() {
    let cases: [(&str, &str); 6] = [
        ("id,mz,rt_min\n", "no_column_rt_max"),
        ("id,mz,rt_min,rt_max\na,-1,1,2\n", "bad_mz"),
        ("id,mz,rt_min,rt_max\na,100,3,2\n", "rt_order"),
        (
            "id,mz,rt_min,rt_max,polarity\na,100,1,2,x\n",
            "bad_polarity",
        ),
        (
            "id,mz,rt_min,rt_max\na,100,1,2\na,101,1,2\n",
            "duplicate_id a",
        ),
        ("[1,", "bad_json"),
    ];
    for (text, detail) in cases {
        let e = parse(text.as_bytes()).unwrap_err();
        assert_eq!(e.key, "err.quant.targets.table", "{text}");
        assert!(
            e.params.iter().any(|(k, v)| *k == "detail" && v == detail),
            "{text}: {:?}",
            e.params
        );
    }
    // the row of a refusal is the file line (header = 1, a comment line counts) or the 1-based JSON item, as the lang context says
    let row_of = |text: &str| {
        let e = parse(text.as_bytes()).unwrap_err();
        let get = |k: &str| {
            e.params
                .iter()
                .find(|(n, _)| *n == k)
                .map(|x| x.1.clone())
                .unwrap()
        };
        (get("row"), get("detail"))
    };
    assert_eq!(
        row_of("# c\nid,mz,rt_min,rt_max\na,100,1,2\nb,x,1,2\n"),
        ("4".into(), "bad_mz".into())
    );
    assert_eq!(
        row_of("id,mz,rt_min,rt_max\na,100,1,2\nb,101,1,2\na,102,1,2\n"),
        ("4".into(), "duplicate_id a".into())
    );
    let json =
        r#"[{"id":"a","mz":1,"rt_min":1,"rt_max":2},{"id":"b","mz":1,"rt_min":2,"rt_max":2}]"#;
    assert_eq!(row_of(json), ("2".into(), "rt_order".into()));
    // a control character in an id (a line break would split a row of the table) is refused; a comma or a quote is kept and quoted
    let json = r#"[{"id":"a\nb","mz":100,"rt_min":1,"rt_max":2}]"#;
    assert_eq!(row_of(json), ("1".into(), "bad_id".into()));
    let ok = parse(br#"[{"id":"a, \"b\"","mz":100,"rt_min":1,"rt_max":2}]"#).unwrap();
    assert_eq!(ok[0].id, "a, \"b\"");
    assert_eq!(parse(b"  ").unwrap_err().key, "err.quant.targets.empty");
    assert_eq!(
        parse(b"id,mz,rt_min,rt_max\n").unwrap_err().key,
        "err.quant.targets.empty"
    );
}

#[test]
fn parameters_are_read_checked_and_never_clamped() {
    let p = |k: &str, v: &str| Params::of(&[(k.to_string(), Value::Str(v.to_string()))]);
    assert_eq!(p("offset", "auto").unwrap().offset, OffsetMode::Auto);
    assert_eq!(p("offset", "0.24").unwrap().offset, OffsetMode::Fixed(0.24));
    assert!(matches!(
        p("offset", "auto:400.1@2").unwrap().offset,
        OffsetMode::Ion(_)
    ));
    assert_eq!(p("offset", "0.9").unwrap_err().key, "err.mz.offset.range");
    assert_eq!(
        p("min_shape", "1.5").unwrap_err().key,
        "err.quant.targets.param"
    );
    assert_eq!(
        p("min_points", "2.5").unwrap_err().key,
        "err.quant.targets.param"
    );
    assert_eq!(p("nope", "1").unwrap_err().key, "err.quant.targets.param");
    let b = p("blank_pattern", "Blank*, *_PB?").unwrap();
    assert!(b.is_blank("blank_01.mzML") && b.is_blank("x_pb1") && !b.is_blank("sample.mzML"));
}
