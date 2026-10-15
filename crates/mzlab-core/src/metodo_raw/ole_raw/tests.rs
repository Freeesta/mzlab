use super::*;

#[test]
fn utf16_and_utf8_texts_are_told_apart() {
    let s = "Mass [m/z] | Formula [M] | Species | CS [z]\n100.0 | C2H6 | [M+H]+ | 1";
    let u16: Vec<u8> = s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    assert_eq!(decode(&u16).as_deref(), Some(s));
    let mut bom = vec![0xFF, 0xFE];
    bom.extend(&u16);
    assert_eq!(decode(&bom).as_deref(), Some(s));
    assert_eq!(decode(s.as_bytes()).as_deref(), Some(s));
    assert!(decode(&[0xC3, 0x28, 0xA0, 0xA1].repeat(20)).is_none()); // neither
    assert!(decode(b"short").is_none());
}

#[test]
fn the_scan_adds_only_the_texts_the_container_lacks() {
    let c = vec![Stream {
        path: "A/Text".into(),
        text: "x".repeat(300),
    }];
    let m = merge(&c, vec!["x".repeat(300), "y".repeat(300)]);
    assert_eq!(m.len(), 2);
    assert!(m[1].starts_with('y'));
}
