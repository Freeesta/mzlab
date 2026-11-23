//! The targets table and the parameters: both forms of the table, the columns told by name, every refusal with its line, the blank globs.

use super::params::{OffsetMode, Params};
use super::targets::{parse, Role};
use mzlab_core::json::Value;

#[test]
fn csv_and_json_tables_give_the_same_targets() {
    let csv = "\u{feff}# list\nName;m/z;rt_max;rt_min;Polarity;Role\na;200.5;6;4;neg;\nb;300;2;1;;standard\n";
    let json = r#"{"targets":[{"id":"a","mz":200.5,"rt_min":4,"rt_max":6,"polarity":"-"},{"name":"b","mz":300,"rt_min":1,"rt_max":2,"role":"parent"}]}"#;
    let a = parse(csv.as_bytes()).unwrap();
    assert_eq!(a, parse(json.as_bytes()).unwrap());
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
