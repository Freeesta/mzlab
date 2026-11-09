use super::*;

#[test]
fn empty_sections_do_not_exist_and_unknown_is_absent() {
    let v = obj(vec![
        ("a", None),
        ("b", Some(V::Arr(vec![]))),
        ("c", num(Some(f64::NAN))),
    ]);
    assert_eq!(v, None);
    let v = obj(vec![
        ("a", Some(V::s("x"))),
        ("b", num(None)),
        ("c", num(Some(2.5))),
    ])
    .unwrap();
    assert_eq!(v.json(), "{\"a\":\"x\",\"c\":2.5}");
}

#[test]
fn userparam_round_trips_any_text() {
    let doc = "{\"schema\":\"metodo/1\",\"x\":\"a \\\"q\\\" <b> & c\\nline\"}";
    let x = format!("<run id=\"r\">{}</run>", userparam_xml(doc));
    assert!(!x.contains("<b>"));
    assert_eq!(from_userparam(&x).as_deref(), Some(doc));
    assert_eq!(from_userparam("<run/>"), None);
}

#[test]
fn sha1_of_a_buffer_and_the_source_files_of_a_header() {
    let mut data = b"abc".to_vec();
    assert_eq!(
        sha1_hex(&mut data, 3).unwrap(),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
    let head = "<fileDescription><sourceFileList count=\"2\"><sourceFile id=\"WIFF\" name=\"B_MRM.wiff\" location=\"file://x\">\n<cvParam cvRef=\"MS\" accession=\"MS:1000562\" name=\"ABI WIFF format\" value=\"\"/>\n<cvParam cvRef=\"MS\" accession=\"MS:1000569\" name=\"SHA-1\" value=\"EABDE1F5F7417181A0790BA25F8D0888530E7261\"/></sourceFile><sourceFile id=\"S\" name=\"B_MRM.wiff.scan\"><cvParam accession=\"MS:1000569\" name=\"SHA-1\" value=\"a047\"/></sourceFile></sourceFileList>";
    let s = source_sha1s(head);
    assert_eq!(
        s,
        vec![
            (
                "B_MRM.wiff".to_string(),
                "eabde1f5f7417181a0790ba25f8d0888530e7261".to_string()
            ),
            ("B_MRM.wiff.scan".to_string(), "a047".to_string())
        ]
    );
    assert_eq!(
        sample_of_id("SRM SIC Q1=1 Q3=2 sample=13 period=1"),
        Some(13)
    );
    assert_eq!(sample_of_id("TIC"), None);
}

#[test]
fn xml_unescape_undoes_the_four_entities_of_the_mzml_writer() {
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    for s in [
        "",
        "a<b>\"c\"&d",
        "&#9; &#10; &#13; &apos;",
        "&amp;lt;",
        "{\"k\":\"x&y\"}",
    ] {
        assert_eq!(xml_unescape(&esc(s)), s);
    }
}

#[test]
fn sha1_of_bytes_matches_the_known_digests_and_the_reader() {
    assert_eq!(
        sha1_hex_bytes(b""),
        "da39a3ee5e6b4b0d3255bfef95601890afd80709"
    );
    assert_eq!(
        sha1_hex_bytes(b"abc"),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
    let data: Vec<u8> = (0..3000u32).map(|i| (i * 7 % 251) as u8).collect();
    let len = data.len() as u64;
    let want = sha1_hex_bytes(&data);
    assert_eq!(want, sha1_hex(&mut data.clone(), len).unwrap());
}
