//! The pictures of `draw_json`/`draw_bytes`: SVG text and PNG bytes of a spec, labels, CIP marks, highlights, scale, the `draw-image/1` wrapper.

use super::super::draw::{draw_bytes, draw_json};
use mzlab_core::json::{self, Value};

fn image(spec: &str, extra: &str) -> String {
    let mut r = String::from("{\"spec\":\"smiles:");
    r += spec;
    r += "\",\"format\":\"svg\"";
    r += extra;
    r += "}";
    draw_json(&r).unwrap()
}

fn wrapped(spec: &str, format: &str, extra: &str) -> Value {
    let r = format!("{{\"spec\":\"smiles:{spec}\",\"format\":\"{format}\",\"json\":true{extra}}}");
    json::parse(draw_json(&r).unwrap().as_bytes()).unwrap()
}

fn texts(v: &Value, kind: &str) -> Vec<String> {
    v.get("labels")
        .unwrap()
        .arr()
        .iter()
        .filter(|l| l.get("kind").and_then(Value::str) == Some(kind))
        .map(|l| l.get("text").and_then(Value::str).unwrap().to_string())
        .collect()
}

#[test]
fn svg_is_one_deterministic_document() {
    let a = image("CC(=O)Oc1ccccc1C(=O)O", "");
    assert!(
        a.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\""),
        "{a}"
    );
    assert!(a.ends_with("</svg>\n"));
    assert_eq!(a, image("CC(=O)Oc1ccccc1C(=O)O", ""));
    assert!(a.contains("<line ") && a.contains("<text "));
    assert!(!a.contains("NaN") && !a.contains("inf"));
}

#[test]
fn only_other_atoms_than_carbon_are_labelled_with_their_hydrogens_on_the_free_side() {
    assert_eq!(texts(&wrapped("CCO", "svg", ""), "atom"), ["OH"]);
    // the bond comes from the right: the hydrogens go to the left
    assert_eq!(texts(&wrapped("OCC", "svg", ""), "atom"), ["HO"]);
    assert_eq!(texts(&wrapped("C[NH3+]", "svg", ""), "atom"), ["NH3+"]);
    assert_eq!(texts(&wrapped("[13CH4]", "svg", ""), "atom"), ["13CH4"]);
    assert_eq!(
        texts(&wrapped("[Na+].[Cl-]", "svg", ""), "atom"),
        ["Na+", "Cl-"]
    );
}

#[test]
fn cip_marks_are_on_by_default_and_off_on_request() {
    let on = wrapped("N[C@@H](C)C(=O)O", "svg", "");
    assert_eq!(texts(&on, "cip"), ["(S)"]);
    assert!(texts(
        &wrapped("N[C@@H](C)C(=O)O", "svg", ",\"cip\":\"off\""),
        "cip"
    )
    .is_empty());
    assert_eq!(texts(&wrapped("C/C=C/C", "svg", ""), "cip"), ["(E)"]);
    assert_eq!(texts(&wrapped("C/C=C\\\\C", "svg", ""), "cip"), ["(Z)"]);
    assert!(texts(&wrapped("CCO", "svg", ""), "cip").is_empty());
}

#[test]
fn highlights_are_halos_by_colour_name_in_the_list_or_the_compact_form() {
    let list = image(
        "CCO",
        ",\"highlight\":[{\"atoms\":[1,2],\"bonds\":[[1,2]],\"color\":\"red\"}]",
    );
    assert!(
        list.contains("#f4a6a6") && list.contains("<circle "),
        "{list}"
    );
    assert_eq!(list, image("CCO", ",\"highlight\":\"1,2,1-2@red\""));
    assert!(!image("CCO", "").contains("<circle "));
    for bad in [
        "[{\"atoms\":[7]}]",
        "[{\"atoms\":[0],\"color\":\"mauve\"}]",
        "\"0-0\"",
        "\"0-2\"",
        "\"0,1,2,0,1,2\"",
        "[{\"bonds\":[[0]]}]",
    ] {
        let r = format!("{{\"spec\":\"smiles:CCO\",\"format\":\"svg\",\"highlight\":{bad}}}");
        assert_eq!(draw_json(&r).unwrap_err().key, "err.mol.draw.set", "{bad}");
    }
}

#[test]
fn everything_of_a_structure_can_be_highlighted_but_not_more_than_its_atoms_and_bonds_in_items() {
    // benzene: 6 atoms and 6 bonds are 12 items, the whole budget
    let all = "0,1,2,3,4,5,0-1,1-2,2-3,3-4,4-5,5-0@red";
    let svg = |spec: &str, h: &str| {
        draw_json(&format!(
            "{{\"spec\":\"smiles:{spec}\",\"format\":\"svg\",\"highlight\":\"{h}\"}}"
        ))
    };
    let ok = svg("c1ccccc1", all).unwrap();
    assert_eq!(ok.matches("<circle ").count(), 6);
    // the same bonds written the other way round
    assert!(svg("c1ccccc1", "1-0,2-1,3-2,4-3,5-4,0-5,0,1,2,3,4,5@blue").is_ok());
    // ethanol: 3 atoms and 2 bonds fill 5 items
    assert!(svg("CCO", "0,1,2,0-1,1-2").is_ok());
    let e = svg("CCO", "0,1,2,0-1,1-2,0").unwrap_err();
    assert_eq!(e.key, "err.mol.draw.set");
    assert!(
        e.params.iter().any(|(_, v)| v == "too many items"),
        "{:?}",
        e.params
    );
    assert!(svg("c1ccccc1", &format!("{all};0@red")).is_err());
    // valid atoms that are not bonded (0 and 3 of benzene) are refused as a bond
    let e = svg("c1ccccc1", "0-3").unwrap_err();
    assert_eq!(e.key, "err.mol.draw.set");
    assert!(e.params.iter().any(|(_, v)| v == "bond"), "{:?}", e.params);
}

#[test]
fn scale_sets_the_size_and_is_checked() {
    let w = |extra: &str| {
        wrapped("CCO", "svg", extra)
            .get("width_px")
            .and_then(Value::f64)
            .unwrap()
    };
    assert!(w(",\"scale\":48") > 1.8 * w(",\"scale\":24"));
    assert!(w(",\"scale\":\"40\"") > w(""));
    for bad in ["7", "81", "\"big\""] {
        let r = format!("{{\"spec\":\"smiles:CCO\",\"format\":\"svg\",\"scale\":{bad}}}");
        assert_eq!(draw_json(&r).unwrap_err().key, "err.mol.draw.set", "{bad}");
    }
}

#[test]
fn a_very_wide_drawing_is_scaled_down_and_says_so() {
    let v = wrapped(&"C".repeat(600), "svg", "");
    let (w, h) = (
        v.get("width_px").and_then(Value::f64).unwrap(),
        v.get("height_px").and_then(Value::f64).unwrap(),
    );
    assert!(w.max(h) <= 4096.0, "{w} x {h}");
    let choices = v.get("choices").unwrap().arr();
    assert!(choices
        .iter()
        .any(|c| c.get("key").and_then(Value::str) == Some("scale_reduced")));
}

#[test]
fn the_wrapper_has_the_schema_first_boxes_in_pixels_and_the_picture() {
    let text =
        draw_json("{\"spec\":\"smiles:CC(=O)Oc1ccccc1C(=O)O\",\"format\":\"svg\",\"json\":true}")
            .unwrap();
    assert!(text.starts_with("{\"schema\":\"draw-image/1\""));
    let v = json::parse(text.as_bytes()).unwrap();
    let svg = v.get("svg").and_then(Value::str).unwrap();
    assert_eq!(svg, image("CC(=O)Oc1ccccc1C(=O)O", ""));
    let (w, h) = (
        v.get("width_px").and_then(Value::f64).unwrap(),
        v.get("height_px").and_then(Value::f64).unwrap(),
    );
    for l in v.get("labels").unwrap().arr() {
        let n = |k: &str| l.get(k).and_then(Value::f64).unwrap();
        assert!(
            n("left_px") >= 0.0 && n("top_px") >= 0.0 && n("right_px") <= w && n("bottom_px") <= h
        );
        assert!(n("left_px") < n("right_px") && n("top_px") < n("bottom_px"));
    }
}

#[test]
fn the_labels_of_a_crowded_drawing_do_not_overlap() {
    for smi in [
        "N[C@@H](CO)C(=O)O",
        "OC[C@H]1OC(O)[C@H](O)[C@@H](O)[C@@H]1O",
        "C[C@]12CC[C@H]3[C@@H](CCC4=CC(=O)CC[C@]34C)[C@@H]1CC[C@@H]2O",
        "NC(N)=[NH2+].[O-]S(=O)(=O)O",
    ] {
        let v = wrapped(smi, "svg", "");
        let b: Vec<[f64; 4]> = v
            .get("labels")
            .unwrap()
            .arr()
            .iter()
            .map(|l| {
                ["left_px", "top_px", "right_px", "bottom_px"]
                    .map(|k| l.get(k).and_then(Value::f64).unwrap())
            })
            .collect();
        for i in 0..b.len() {
            for j in i + 1..b.len() {
                let (p, q) = (b[i], b[j]);
                assert!(
                    !(p[0] < q[2] && q[0] < p[2] && p[1] < q[3] && q[1] < p[3]),
                    "{smi}: {p:?} {q:?}"
                );
            }
        }
    }
}

fn chunks(png: &[u8]) -> Vec<(String, Vec<u8>)> {
    assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 13, 10, 26, 10]);
    let (mut i, mut out) = (8, Vec::new());
    while i < png.len() {
        let n = u32::from_be_bytes(png[i..i + 4].try_into().unwrap()) as usize;
        let body = &png[i + 4..i + 8 + n];
        let crc = u32::from_be_bytes(png[i + 8 + n..i + 12 + n].try_into().unwrap());
        assert_eq!(crc, mzlab_core::zip::crc32(body), "crc of a chunk");
        out.push((
            String::from_utf8(body[..4].to_vec()).unwrap(),
            body[4..].to_vec(),
        ));
        i += 12 + n;
    }
    out
}

/// The bytes of a zlib stream made of stored blocks, its Adler-32 checked.
fn unstore(z: &[u8]) -> Vec<u8> {
    assert_eq!(&z[..2], &[0x78, 0x01]);
    let (mut i, mut raw) = (2, Vec::new());
    loop {
        let last = z[i] == 1;
        let n = usize::from(u16::from_le_bytes([z[i + 1], z[i + 2]]));
        assert_eq!(u16::from_le_bytes([z[i + 3], z[i + 4]]), !(n as u16));
        raw.extend_from_slice(&z[i + 5..i + 5 + n]);
        i += 5 + n;
        if last {
            break;
        }
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &v in &raw {
        a = (a + u32::from(v)) % 65521;
        b = (b + a) % 65521;
    }
    assert_eq!(&z[i..], &((b << 16) | a).to_be_bytes());
    raw
}

#[test]
fn png_is_a_valid_rgb_file_of_the_size_of_the_svg_with_ink_on_white() {
    let spec = "N[C@@H](C)C(=O)O";
    let png = draw_bytes(&format!(
        "{{\"spec\":\"smiles:{spec}\",\"format\":\"png\",\"highlight\":\"0@red\"}}"
    ))
    .unwrap();
    assert_eq!(
        png,
        draw_bytes(&format!(
            "{{\"spec\":\"smiles:{spec}\",\"format\":\"png\",\"highlight\":\"0@red\"}}"
        ))
        .unwrap()
    );
    let c = chunks(&png);
    assert_eq!(
        c.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(),
        ["IHDR", "IDAT", "IEND"]
    );
    let v = wrapped(spec, "svg", "");
    let (w, h) = (
        v.get("width_px").and_then(Value::f64).unwrap() as u32,
        v.get("height_px").and_then(Value::f64).unwrap() as u32,
    );
    assert_eq!(u32::from_be_bytes(c[0].1[0..4].try_into().unwrap()), w);
    assert_eq!(u32::from_be_bytes(c[0].1[4..8].try_into().unwrap()), h);
    assert_eq!(&c[0].1[8..], &[8, 2, 0, 0, 0]);
    let raw = unstore(&c[1].1);
    assert_eq!(raw.len(), (h * (3 * w + 1)) as usize);
    let px = |x: u32, y: u32| &raw[(y * (3 * w + 1) + 1 + 3 * x) as usize..][..3];
    assert_eq!(px(0, 0), &[255, 255, 255]);
    assert!(
        raw.chunks(3 * w as usize + 1)
            .any(|r| r[1..].chunks(3).any(|p| p == [0, 0, 0])),
        "ink"
    );
    assert!(
        raw.chunks(3 * w as usize + 1)
            .any(|r| r[1..].chunks(3).any(|p| p == [244, 166, 166])),
        "halo"
    );
    // wrapped: the same file as base64
    let b = wrapped(spec, "png", ",\"highlight\":\"0@red\"");
    assert_eq!(
        b.get("png_base64").and_then(Value::str).unwrap(),
        super::base64(&png)
    );
    // text answers refuse a bare PNG
    assert_eq!(
        draw_json(&format!(
            "{{\"spec\":\"smiles:{spec}\",\"format\":\"png\"}}"
        ))
        .unwrap_err()
        .key,
        "err.mol.draw.binary"
    );
}

#[test]
fn an_empty_or_bonded_nothing_still_gives_a_picture() {
    let svg = image("[He]", "");
    assert!(svg.contains(">He</text>"), "{svg}");
    assert_eq!(
        draw_json("{\"spec\":\"smiles:CCO\",\"format\":\"gif\"}")
            .unwrap_err()
            .key,
        "err.mol.draw.set"
    );
}

/// A V3000 molfile of two bonded carbons at `a` and `b`, as the text of a JSON string.
fn two_atoms(a: &str, b: &str) -> String {
    let lines = [
        "title",
        "  mzlab",
        "",
        "  0  0  0     0  0            999 V3000",
        "M  V30 BEGIN CTAB",
        "M  V30 COUNTS 2 1 0 0 0",
        "M  V30 BEGIN ATOM",
        &format!("M  V30 1 C {a} 0 0"),
        &format!("M  V30 2 C {b} 0 0"),
        "M  V30 END ATOM",
        "M  V30 BEGIN BOND",
        "M  V30 1 1 1 2",
        "M  V30 END BOND",
        "M  V30 END CTAB",
        "M  END",
    ];
    lines.join("\\n")
}

#[test]
fn coordinates_that_are_huge_or_not_finite_are_laid_out_again_and_the_picture_stays_small() {
    let mut read = 0;
    for (a, b) in [
        ("0 0", "1e9 1e9"),
        ("0 0", "1e300 -1e300"),
        ("nan 0", "1 inf"),
    ] {
        let r = format!(
            "{{\"spec\":\"{}\",\"format\":\"svg\",\"json\":true}}",
            two_atoms(a, b)
        );
        // the file may be refused as unreadable; when it is read the picture is bounded and has no NaN
        let Ok(text) = draw_json(&r) else { continue };
        read += 1;
        let v = json::parse(text.as_bytes()).unwrap();
        let (w, h) = (
            v.get("width_px").unwrap().f64().unwrap(),
            v.get("height_px").unwrap().f64().unwrap(),
        );
        assert!(w <= 4096.0 && h <= 4096.0 && w >= 1.0, "{a} {b}: {w} x {h}");
        let svg = v.get("svg").and_then(Value::str).unwrap();
        assert!(!svg.contains("NaN") && !svg.contains("inf"), "{a} {b}");
        assert!(text.contains("beyond 1000 angstrom"), "{a} {b}: {text}");
    }
    assert!(read >= 1, "no file was read");
}

#[test]
fn the_title_of_a_picture_loses_the_characters_xml_forbids() {
    let r = "{\"spec\":\"smiles:CCO\",\"format\":\"svg\",\"name\":\"a\\u0001b\\u000bc<d\\ud7ff\\uffff\\te\"}";
    let svg = draw_json(r).unwrap();
    assert!(svg.contains("<title>abc&lt;d\u{d7ff}\te</title>"), "{svg}");
    assert!(svg
        .chars()
        .all(|c| c >= ' ' || matches!(c, '\t' | '\n' | '\r')));
}

#[test]
fn a_refused_setting_shows_the_value_as_given_and_a_png_as_text_has_its_own_error() {
    for (extra, value) in [
        (",\"scale\":true", "true"),
        (",\"cip\":5", "5"),
        (",\"cip\":\"maybe\"", "maybe"),
        (",\"scale\":\"200\"", "200"),
    ] {
        let r = format!("{{\"spec\":\"smiles:CCO\",\"format\":\"svg\"{extra}}}");
        let e = draw_json(&r).unwrap_err();
        assert_eq!(e.key, "err.mol.draw.set");
        assert!(
            e.params.iter().any(|(k, v)| *k == "value" && v == value),
            "{extra}: {:?}",
            e.params
        );
    }
    let e = draw_json("{\"spec\":\"smiles:CCO\",\"format\":\"png\"}").unwrap_err();
    assert_eq!(e.key, "err.mol.draw.binary");
}
