//! `json::parse_members` against `json::parse` on the same texts.

use super::*;
use crate::json::parse;

/// `parse_members` keeps the members `parse` gives for the same keys and fails on the same texts.
#[test]
fn members_match_the_whole_parse() {
    let keep = ["format", "rows"];
    let ok = [
        r#" {"format":"annot/1","edges":[{"a":1,"b":[true,false,null,NaN,-1.5e3]}],"rows":[{"x":"è😀\n"}],"rows":2} "#,
        r#"{"edges":{},"other":[],"format":"x"}"#,
        "{}",
    ];
    for t in ok {
        let (whole, some) = (
            parse(t.as_bytes()).ok(),
            parse_members(t.as_bytes(), &keep).ok(),
        );
        let (Some(Value::Obj(w)), Some(Value::Obj(s))) = (whole, some) else {
            panic!("{t}");
        };
        let kept: Vec<_> = w
            .into_iter()
            .filter(|(k, _)| keep.contains(&k.as_str()))
            .collect();
        assert_eq!(format!("{kept:?}"), format!("{s:?}"), "{t}");
    }
    assert!(matches!(
        parse_members(b"[1,{\"a\":2}]", &keep),
        Ok(Value::Null)
    ));
    let deep = format!("{{\"edges\":{}1{}}}", "[".repeat(70), "]".repeat(70));
    let bad = [
        "{",
        r#"{"edges":[1,]}"#,
        r#"{"edges":tru}"#,
        r#"{"edges":"\u12"}"#,
        r#"{"a":1} x"#,
        r#"{"edges":[1}"#,
        deep.as_str(),
    ];
    for t in bad {
        assert!(
            parse(t.as_bytes()).is_err() && parse_members(t.as_bytes(), &keep).is_err(),
            "{t}"
        );
    }
    assert!(parse_members(
        &[b'{', b'"', b'e', b'"', b':', b'"', 0xff, b'"', b'}'],
        &keep
    )
    .is_err());
}
