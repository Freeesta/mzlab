//! Tests of `open/1` (`structure_file.rs`): which empty attributes are dropped from CDXML, how bytes are decoded, the JSON, and base64.

use super::*;

const RDKIT_FILE: &str = include_str!("../../../../tests/dati/cdxml/aromatics.cdxml");

fn tidy(s: &str) -> (String, Vec<String>) {
    tidy_cdxml(s)
}

fn open_ok(b: &[u8]) -> Opened {
    open_bytes(b).unwrap()
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
    // `a < b` is text and the tag after it is still cleaned; a comment that never closes is malformed: the rest is copied as it is
    assert_eq!(
        tidy(r#"<CDXML><s>a < b</s><n x=""/><!-- open <m y=""/></CDXML>"#),
        (
            r#"<CDXML><s>a < b</s><n/><!-- open <m y=""/></CDXML>"#.to_string(),
            names(&["x"])
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
        open_ok(&utf8),
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
            open_ok(&b),
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
        open_ok(&latin1),
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
    let o = open_ok(&b);
    assert_eq!(o.text, "<CDXML><s>\u{20ac} \u{201c}x\u{201d}</s></CDXML>");
    assert_eq!(o.changes, vec![Change::Charset("windows-1252".into())]);
    let b = b"<?xml version=\"1.0\" encoding=\"Shift_JIS\"?><CDXML><s>\x83</s></CDXML>".to_vec();
    assert_eq!(
        open_ok(&b).changes,
        vec![Change::Charset("windows-1252".into())],
        "no table for Shift_JIS: the notice names what was used"
    );
}

#[test]
fn binary_cdx_goes_on_as_base64_unchanged_and_a_file_that_needs_nothing_says_nothing() {
    assert_eq!(
        open_ok(&[0x56, 0x6a, 0x43, 0x44, 1, 2, 3]),
        Opened {
            text: "VmpDRAECAw==".into(),
            changes: vec![]
        }
    );
    let clean = "<CDXML><page/></CDXML>";
    assert_eq!(
        open_ok(clean.as_bytes()),
        Opened {
            text: clean.into(),
            changes: vec![]
        }
    );
    assert_eq!(
        open_ok(b""),
        Opened {
            text: String::new(),
            changes: vec![]
        }
    );
}

#[test]
fn the_rdkit_fixture_carries_the_empty_attribute_the_editor_cannot_read() {
    assert!(RDKIT_FILE.contains("BondLength=\"\""));
    let o = open_ok(RDKIT_FILE.as_bytes());
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
    let j = open_json_bytes(b"<CDXML a=\"\"/>").unwrap();
    assert_eq!(
        j,
        r#"{"schema":"open/1","text":"<CDXML/>","changes":[{"fix":"empty","names":["a"]}]}"#
    );
    assert_eq!(
        open_json_bytes(b"<CDXML/>").unwrap(),
        r#"{"schema":"open/1","text":"<CDXML/>","changes":[]}"#
    );
    let req = format!("{{\"data\":\"{}\"}}", base64(RDKIT_FILE.as_bytes()));
    assert_eq!(
        open_json(&req).unwrap(),
        open_json_bytes(RDKIT_FILE.as_bytes()).unwrap()
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

#[test]
fn text_outside_the_start_tags_is_never_edited() {
    // a stray quote in a tag opens a string that ends with the next quote: `y=''` is inside it, not an attribute
    assert_eq!(
        tidy(r#"<CDXML><a b"x y=''"/><c d=""/></CDXML>"#),
        (
            r#"<CDXML><a b"x y=''"/><c/></CDXML>"#.to_string(),
            names(&["d"])
        )
    );
    // the DOCTYPE with its internal subset, a processing instruction and the text before the root are copied
    let t = r#"<?xml version="1.0"?><!DOCTYPE CDXML [ <!ENTITY e "<b x=''>"> <!-- > <q z=''> --> ]><?pi <p w=''>?><CDXML a=""/>"#;
    assert_eq!(tidy(t), (t.replace(r#" a="""#, ""), names(&["a"])));
    // an `<?xml encoding=` that is not the declaration at the start of the file is text
    let t = "<CDXML><s><?xml encoding=\"UTF-16\"?></s></CDXML>";
    assert_eq!(declare_utf8(t), t);
    assert_eq!(
        declare_utf8("\n <?xml version=\"1.0\" encoding='UTF-16'?><CDXML/>"),
        "\n <?xml version=\"1.0\" encoding='UTF-8'?><CDXML/>"
    );
    // the root must be CDXML: a CDXML tag inside a comment of another document does not make it one
    let t = r#"<!-- <CDXML> --><molecule a=""/>"#;
    assert_eq!(tidy(t), (t.to_string(), vec![]));
}

#[test]
fn a_utf8_file_with_bad_bytes_says_characters_were_replaced() {
    let o = open_ok(b"<?xml version=\"1.0\" encoding=\"utf-8\"?><CDXML><s>a\xffb\xfe</s></CDXML>");
    assert_eq!(
        o.changes,
        vec![Change::Charset("utf-8".into()), Change::Replaced(2)]
    );
    assert!(o.text.contains("a\u{fffd}b\u{fffd}"));
    // an unpaired surrogate and an odd last byte in UTF-16 are told too
    let mut b = vec![0xff, 0xfe];
    for u in [0x3c_u16, 0xd800, 0x3e] {
        b.extend(u.to_le_bytes());
    }
    b.push(0x41);
    let o = open_ok(&b);
    assert_eq!(
        o.changes,
        vec![Change::Charset("UTF-16".into()), Change::Replaced(2)]
    );
}

#[test]
fn utf16_without_a_byte_order_mark_is_told_by_its_nul_bytes() {
    let xml =
        "<?xml version=\"1.0\" encoding=\"UTF-16\"?><CDXML BondLength=\"\"><s>\u{fc}</s></CDXML>";
    for le in [true, false] {
        let mut b = Vec::new();
        for u in xml.encode_utf16() {
            b.extend(if le { u.to_le_bytes() } else { u.to_be_bytes() });
        }
        let o = open_ok(&b);
        assert_eq!(
            o.changes,
            vec![
                Change::Charset(if le { "UTF-16LE" } else { "UTF-16BE" }.into()),
                Change::Empty(names(&["BondLength"]))
            ]
        );
        assert_eq!(
            o.text,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><CDXML><s>\u{fc}</s></CDXML>"
        );
    }
    // plain text with a stray NUL is not UTF-16
    assert!(open_ok(b"<CDXML a=\"b\"><s>x\0y</s></CDXML>")
        .changes
        .is_empty());
}

#[test]
fn the_names_are_an_array_and_each_fix_carries_its_own_fields() {
    let mut b = vec![0xff, 0xfe];
    for u in "<CDXML a=\"\" b=''/>".encode_utf16() {
        b.extend(u.to_le_bytes());
    }
    let j = open_json_bytes(&b).unwrap();
    assert_eq!(
        j,
        r#"{"schema":"open/1","text":"<CDXML/>","changes":[{"fix":"charset","charset":"UTF-16"},{"fix":"empty","names":["a","b"]}]}"#
    );
    assert!(
        open_json_bytes(b"<CDXML>\xff</CDXML>")
            .unwrap()
            .contains(r#"{"fix":"replaced","count":1}"#)
            == false
    ); // no declaration: windows-1252, nothing replaced
    assert!(
        open_json_bytes(b"<?xml encoding=\"utf-8\"?><CDXML>\xff</CDXML>")
            .unwrap()
            .contains(r#"{"fix":"replaced","count":1}"#)
    );
}

/// The hostile inputs of the review: each took seconds or minutes before the scanner read a run of white space or a missing closer once.
#[test]
fn hostile_input_is_read_in_linear_time() {
    let n = 200_000;
    let cases: Vec<(&str, String)> = vec![
        ("blank run in a tag", format!("<CDXML{}", " ".repeat(n))),
        (
            "blank run then an attribute",
            format!("<CDXML{}a=\"\">", " ".repeat(n)),
        ),
        ("unclosed comments", format!("<CDXML>{}", "<!--".repeat(n))),
        (
            "unclosed CDATA",
            format!("<CDXML>{}", "<![CDATA[".repeat(n)),
        ),
        ("unclosed quotes", format!("<CDXML>{}", "<a \"".repeat(n))),
        (
            "unclosed single quotes",
            format!("<CDXML>{}", "<a '".repeat(n)),
        ),
        (
            "mixed quotes",
            format!("<CDXML>{}", "<a '<b \"".repeat(n / 2)),
        ),
        ("open tags", format!("<CDXML>{}", "<a ".repeat(n))),
        ("brackets", format!("<CDXML>{}", "<".repeat(n))),
        (
            "stray quotes in a tag",
            format!("<CDXML a{}>", "\"x y='' ".repeat(n / 4)),
        ),
        (
            "many empty attributes",
            format!("<CDXML{}>", " a=''".repeat(n / 4)),
        ),
        (
            "unclosed declaration",
            format!("<CDXML><!DOCTYPE x [{}", "<!ENTITY a \"b\">".repeat(n / 8)),
        ),
        (
            "unclosed instruction",
            format!("<CDXML>{}", "<?x ".repeat(n)),
        ),
    ];
    for (name, t) in &cases {
        let start = std::time::Instant::now();
        let (out, _) = tidy(t);
        let took = start.elapsed();
        assert!(
            took < std::time::Duration::from_secs(2),
            "{name}: {} bytes took {took:?}",
            t.len()
        );
        assert!(out.len() <= t.len(), "{name}");
    }
}

#[test]
fn a_file_over_the_cap_is_refused_with_its_size() {
    let big = vec![b' '; MAX_BYTES + 1];
    let e = open_bytes(&big).unwrap_err();
    assert_eq!(e.key, "error.open.tooBig");
    assert_eq!(
        e.params,
        vec![
            ("size", (MAX_BYTES + 1).to_string()),
            ("limit", MAX_BYTES.to_string())
        ]
    );
    assert_eq!(open_json_bytes(&big).unwrap_err().key, "error.open.tooBig");
    assert!(open_bytes(&big[..MAX_BYTES]).is_ok());
}
