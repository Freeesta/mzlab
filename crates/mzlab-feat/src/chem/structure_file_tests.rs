//! Tests of `open/1` (`structure_file.rs`): which empty attributes are dropped from CDXML, how bytes are decoded, the JSON, and base64.

use super::*;

const RDKIT_FILE: &str = include_str!("../../../../tests/dati/cdxml/aromatics.cdxml");

fn tidy(s: &str) -> (String, Vec<String>) {
    tidy_cdxml(s)
}

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn an_empty_attribute_of_a_cdxml_start_tag_is_dropped_and_named_once() {
    assert_eq!(
        tidy(r#"<CDXML a="1" BondLength="" b=""><n x=""/></CDXML>"#),
        (
            r#"<CDXML a="1"><n/></CDXML>"#.to_string(),
            names(&["BondLength", "b", "x"])
        )
    );
    // text between tags is never touched
    let t = r#"<s>Q=""</s><CDXML/>"#;
    assert_eq!(tidy(t), (t.to_string(), vec![]));
}

#[test]
fn text_that_is_not_cdxml_is_left_alone() {
    let t = r#"<molecule a="">"#;
    assert_eq!(tidy(t), (t.to_string(), vec![]));
    let t = r#"<CDXMLish a=""/>"#;
    assert_eq!(tidy(t), (t.to_string(), vec![]));
}

#[test]
fn a_quoted_value_may_hold_a_bracket_or_the_other_quote_and_is_never_edited() {
    assert_eq!(
        tidy(r#"<CDXML a="x > y" b=""><n/></CDXML>"#),
        (
            r#"<CDXML a="x > y"><n/></CDXML>"#.to_string(),
            names(&["b"])
        )
    );
    assert_eq!(
        tidy(r#"<CDXML a='' b="" c='it"s' d="it's" e=''/>"#),
        (
            r#"<CDXML c='it"s' d="it's"/>"#.to_string(),
            names(&["a", "b", "e"])
        )
    );
    assert_eq!(
        tidy(r#"<CDXML><t v="a b=&quot;&quot;"/><u w = "" /></CDXML>"#),
        (
            r#"<CDXML><t v="a b=&quot;&quot;"/><u /></CDXML>"#.to_string(),
            names(&["w"])
        )
    );
}

#[test]
fn markup_inside_a_comment_or_a_cdata_section_is_not_a_tag() {
    assert_eq!(
        tidy(r#"<CDXML a=""><!-- <x b=""> --><s><![CDATA[<y c="">]]></s></CDXML>"#),
        (
            r#"<CDXML><!-- <x b=""> --><s><![CDATA[<y c="">]]></s></CDXML>"#.to_string(),
            names(&["a"])
        )
    );
}

#[test]
fn empty_attributes_go_from_every_start_tag_not_only_the_root() {
    assert_eq!(
        tidy(r#"<CDXML><page><fragment id="1" Name=""><n z=""/></fragment></page></CDXML>"#),
        (
            r#"<CDXML><page><fragment id="1"><n/></fragment></page></CDXML>"#.to_string(),
            names(&["Name", "z"])
        )
    );
}

#[test]
fn a_bracket_that_opens_no_tag_and_an_unterminated_quote_or_comment_do_not_stop_the_scan() {
    // `a < b` is text; the tag after it is still cleaned; the comment that never closes is text too
    assert_eq!(
        tidy(r#"<CDXML><s>a < b</s><n x=""/><!-- open <m y=""/></CDXML>"#),
        (
            r#"<CDXML><s>a < b</s><n/><!-- open <m/></CDXML>"#.to_string(),
            names(&["x", "y"])
        )
    );
    let t = r#"<CDXML><n x="1 y=""/></CDXML>"#;
    assert_eq!(tidy(t).1.len(), 0, "{t}");
}

#[test]
fn the_bytes_are_decoded_as_their_mark_or_declaration_says() {
    let xml = |enc: &str| {
        format!("<?xml version=\"1.0\" encoding=\"{enc}\" ?><CDXML BondLength=\"\"><s>\u{fc}</s></CDXML>")
    };
    let clean = |enc: &str| {
        format!("<?xml version=\"1.0\" encoding=\"{enc}\" ?><CDXML><s>\u{fc}</s></CDXML>")
    };
    let empty = Change::Empty(names(&["BondLength"]));

    let mut utf8 = vec![0xef, 0xbb, 0xbf];
    utf8.extend(xml("UTF-8").bytes());
    assert_eq!(
        open_bytes(&utf8),
        Opened {
            text: clean("UTF-8"),
            changes: vec![empty]
        }
    );

    for le in [true, false] {
        let mut b = if le {
            vec![0xff, 0xfe]
        } else {
            vec![0xfe, 0xff]
        };
        for u in xml("UTF-16").encode_utf16() {
            b.extend(if le { u.to_le_bytes() } else { u.to_be_bytes() });
        }
        // the declaration no longer names an encoding the text is not in
        assert_eq!(
            open_bytes(&b),
            Opened {
                text: clean("UTF-8"),
                changes: vec![
                    Change::Charset("UTF-16".into()),
                    Change::Empty(names(&["BondLength"]))
                ]
            }
        );
    }

    let latin1: Vec<u8> = xml("ISO-8859-1").chars().map(|c| c as u8).collect();
    assert_eq!(
        open_bytes(&latin1),
        Opened {
            text: clean("ISO-8859-1"),
            changes: vec![
                Change::Charset("ISO-8859-1".into()),
                Change::Empty(names(&["BondLength"]))
            ]
        }
    );
}

#[test]
fn eight_bit_text_without_a_known_declaration_is_windows_1252_and_says_so() {
    let b = b"<CDXML><s>\x80 \x93x\x94</s></CDXML>".to_vec();
    let o = open_bytes(&b);
    assert_eq!(o.text, "<CDXML><s>\u{20ac} \u{201c}x\u{201d}</s></CDXML>");
    assert_eq!(o.changes, vec![Change::Charset("windows-1252".into())]);
    let b = b"<?xml version=\"1.0\" encoding=\"Shift_JIS\"?><CDXML><s>\x83</s></CDXML>".to_vec();
    assert_eq!(
        open_bytes(&b).changes,
        vec![Change::Charset("windows-1252".into())],
        "no table for Shift_JIS: the notice names what was used"
    );
}

#[test]
fn binary_cdx_goes_on_as_base64_unchanged_and_a_file_that_needs_nothing_says_nothing() {
    assert_eq!(
        open_bytes(&[0x56, 0x6a, 0x43, 0x44, 1, 2, 3]),
        Opened {
            text: "VmpDRAECAw==".into(),
            changes: vec![]
        }
    );
    let clean = "<CDXML><page/></CDXML>";
    assert_eq!(
        open_bytes(clean.as_bytes()),
        Opened {
            text: clean.into(),
            changes: vec![]
        }
    );
    assert_eq!(
        open_bytes(b""),
        Opened {
            text: String::new(),
            changes: vec![]
        }
    );
}

#[test]
fn the_rdkit_fixture_carries_the_empty_attribute_the_editor_cannot_read() {
    assert!(RDKIT_FILE.contains("BondLength=\"\""));
    let o = open_bytes(RDKIT_FILE.as_bytes());
    assert_eq!(o.changes, vec![Change::Empty(names(&["BondLength"]))]);
    assert!(!o.text.contains("BondLength"));
    // only the attribute and the white space before it (RDKit writes one attribute per line) went
    let at = RDKIT_FILE.find("BondLength=\"\"").unwrap();
    let start = RDKIT_FILE[..at].trim_end().len();
    assert_eq!(
        o.text,
        format!(
            "{}{}",
            &RDKIT_FILE[..start],
            &RDKIT_FILE[at + "BondLength=\"\"".len()..]
        )
    );
}

#[test]
fn the_json_has_stable_keys_and_the_request_form_gives_the_same_bytes() {
    let j = open_json_bytes(b"<CDXML a=\"\"/>");
    assert_eq!(
        j,
        r#"{"schema":"open/1","text":"<CDXML/>","changes":[{"fix":"empty","names":"a"}]}"#
    );
    assert_eq!(
        open_json_bytes(b"<CDXML/>"),
        r#"{"schema":"open/1","text":"<CDXML/>","changes":[]}"#
    );
    let req = format!("{{\"data\":\"{}\"}}", base64(RDKIT_FILE.as_bytes()));
    assert_eq!(
        open_json(&req).unwrap(),
        open_json_bytes(RDKIT_FILE.as_bytes())
    );
    for bad in [
        "",
        "[]",
        "{}",
        r#"{"data":5}"#,
        r#"{"data":"a"}"#,
        r#"{"data":"=AAA"}"#,
        r#"{"data":"AA==AAAA"}"#,
    ] {
        assert_eq!(
            open_json(bad).unwrap_err().key,
            "error.open.request",
            "{bad}"
        );
    }
}

#[test]
fn base64_round_trips_every_length() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    for n in 0..40usize {
        let b: Vec<u8> = (0..n).map(|i| (i * 37 + 11) as u8).collect();
        assert_eq!(unbase64(&base64(&b)).unwrap(), b, "{n}");
    }
}
