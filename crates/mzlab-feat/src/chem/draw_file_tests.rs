//! `draw_file`: KET and CDXML written for a drawn structure. The readers below are test-only and small: they turn the file back into a V2000
//! molfile for `mzlab-mol`, and the round trip must give the same molecule (core hash, stereo included) on the 200 synthetic molecules of
//! `tests/dati/draw_set_a.tsv`.

use super::super::draw::{draw, draw_json};
use mzlab_core::json::Out;
use mzlab_mol::Mol;

const SET_A: &str = include_str!("../../../../tests/dati/draw_set_a.tsv");
const SYMBOLS: &str = "H He Li Be B C N O F Ne Na Mg Al Si P S Cl Ar K Ca Sc Ti V Cr Mn Fe Co Ni Cu Zn Ga Ge As Se Br Kr Rb Sr Y Zr Nb Mo Tc Ru Rh Pd Ag Cd In Sn Sb Te I Xe";

/// One atom read from a file: symbol, x, y (angstrom, y up), charge, isotope, radical code, stated hydrogens.
struct A(String, f64, f64, i64, i64, i64, Option<u32>);
/// One bond read from a file: atoms (0-based), order, molfile stereo code.
struct B(usize, usize, i64, i64);

/// The V2000 molfile of atoms and bonds read from a KET or CDXML file.
fn molfile(atoms: &[A], bonds: &[B]) -> String {
    let mut sum = vec![0i64; atoms.len()];
    for b in bonds {
        sum[b.0] += b.2;
        sum[b.1] += b.2;
    }
    let mut s = format!(
        "\n\n\n{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
        atoms.len(),
        bonds.len()
    );
    for (i, a) in atoms.iter().enumerate() {
        let v = a.6.map_or(0, |h| {
            if sum[i] + i64::from(h) == 0 {
                15
            } else {
                sum[i] + i64::from(h)
            }
        });
        s += &format!(
            "{:>10.4}{:>10.4}{:>10.4} {:<3} 0  0  0  0  0{v:>3}  0  0  0  0  0  0\n",
            a.1, a.2, 0.0, a.0
        );
    }
    for b in bonds {
        s += &format!("{:>3}{:>3}{:>3}{:>3}\n", b.0 + 1, b.1 + 1, b.2, b.3);
    }
    for (tag, k) in [("CHG", 3), ("ISO", 4), ("RAD", 5)] {
        for (i, a) in atoms.iter().enumerate() {
            let v = [0, 0, 0, a.3, a.4, a.5][k];
            if v != 0 {
                s += &format!("M  {tag}  1 {:>3} {v:>3}\n", i + 1);
            }
        }
    }
    s + "M  END\n"
}

/// The molecule a KET text holds (first molecule node; the text nodes are passed over).
fn read_ket(text: &str) -> Mol {
    let v: serde_json::Value = serde_json::from_str(text).expect("KET is JSON");
    let mol = &v["mol0"];
    let atoms: Vec<A> = mol["atoms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            let f = |k: &str| a[k].as_i64().unwrap_or(0);
            let loc = |k: usize| a["location"][k].as_f64().unwrap() * 1.5;
            A(
                a["label"].as_str().unwrap().into(),
                loc(0),
                loc(1),
                f("charge"),
                f("isotope"),
                f("radical"),
                a["implicitHCount"].as_u64().map(|h| h as u32),
            )
        })
        .collect();
    let bonds: Vec<B> = mol["bonds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            let at = |k: usize| b["atoms"][k].as_u64().unwrap() as usize;
            B(
                at(0),
                at(1),
                b["type"].as_i64().unwrap(),
                b["stereo"].as_i64().unwrap_or(0),
            )
        })
        .collect();
    Mol::from_molfile(&molfile(&atoms, &bonds)).expect("the KET molecule reads")
}

/// The value of `name="..."` in a start tag.
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let from = tag.find(&key)? + key.len();
    Some(&tag[from..from + tag[from..].find('"')?])
}

/// The molecule a CDXML text holds: its `n` and `b` start tags, points back to angstrom with y turned up.
fn read_cdxml(text: &str) -> Mol {
    let tags = |name: &str| -> Vec<&str> {
        let open = format!("<{name} ");
        text.match_indices(&open)
            .map(|(i, _)| &text[i..i + text[i..].find('>').unwrap()])
            .collect()
    };
    let symbols: Vec<&str> = SYMBOLS.split(' ').collect();
    let (mut ids, mut atoms) = (Vec::new(), Vec::new());
    for n in tags("n") {
        let p: Vec<f64> = attr(n, "p")
            .unwrap()
            .split(' ')
            .map(|v| v.parse().unwrap())
            .collect();
        let z: usize = attr(n, "Element").map_or(6, |z| z.parse().unwrap());
        let int = |k: &str| attr(n, k).map_or(0, |v| v.parse::<i64>().unwrap());
        ids.push(attr(n, "id").unwrap());
        atoms.push(A(
            symbols[z - 1].into(),
            p[0] / 20.0,
            -p[1] / 20.0,
            int("Charge"),
            int("Isotope"),
            match attr(n, "Radical") {
                None => 0,
                Some("Doublet") => 2,
                Some("Triplet") => 3,
                Some(other) => panic!("Radical {other}"),
            },
            attr(n, "NumHydrogens").map(|h| h.parse().unwrap()),
        ));
    }
    let bonds: Vec<B> = tags("b")
        .into_iter()
        .map(|b| {
            let at = |k: &str| ids.iter().position(|i| Some(*i) == attr(b, k)).unwrap();
            let order = attr(b, "Order").map_or(1, |o| o.parse().unwrap());
            let stereo = match attr(b, "Display") {
                Some("WedgeBegin") => 1,
                Some("WedgedHashBegin") => 6,
                Some("Wavy") if order == 1 => 4,
                Some("Wavy") => 3,
                _ => 0,
            };
            B(at("B"), at("E"), order, stereo)
        })
        .collect();
    Mol::from_molfile(&molfile(&atoms, &bonds)).expect("the CDXML molecule reads")
}

fn request(spec: &str, extra: &str) -> String {
    let mut o = Out::default();
    o.raw("{\"spec\":").str(spec).raw(extra).raw("}");
    o.0
}

fn file(spec: &str, format: &str) -> String {
    draw_json(&request(spec, &format!(",\"format\":\"{format}\""))).unwrap()
}

#[test]
fn the_200_synthetic_molecules_read_back_as_the_same_molecule() {
    let mut n = 0;
    for row in SET_A.lines().skip(1) {
        let smiles = row.split('\t').nth(1).unwrap();
        let want = draw(&request(smiles, "")).unwrap().0.mol.can();
        let (ket, cdxml) = (file(smiles, "ket"), file(smiles, "cdxml"));
        assert_eq!(read_ket(&ket).can(), want, "KET of {smiles}");
        assert_eq!(read_cdxml(&cdxml).can(), want, "CDXML of {smiles}");
        n += 1;
    }
    assert_eq!(n, 200);
}

#[test]
fn charges_isotopes_radicals_hydrogens_and_stereo_survive() {
    for smiles in [
        "C[C@H](N)C(=O)O",
        "C[C@@H](N)C(=O)O",
        "C/C=C/C(=O)O",
        "C/C=C\\C(=O)O",
        "[13CH4]",
        "[NH4+].[Cl-]",
        "[CH3]",
        "[O-][N+](=O)c1ccccc1",
        "[2H]C([2H])([2H])O",
        "C[S@](=O)c1ccccc1",
        "F[C@](Cl)(Br)I",
        "[NH3+]CC(=O)[O-]",
    ] {
        let want = draw(&request(smiles, "")).unwrap().0.mol.can();
        assert_eq!(
            read_ket(&file(smiles, "ket")).can(),
            want,
            "KET of {smiles}"
        );
        assert_eq!(
            read_cdxml(&file(smiles, "cdxml")).can(),
            want,
            "CDXML of {smiles}"
        );
    }
}

#[test]
fn a_wedge_starts_at_its_narrow_end_in_both_formats() {
    let ket = file("C[C@H](N)C(=O)O", "ket");
    assert!(ket.starts_with("{\"root\":{\"nodes\":[{\"$ref\":\"mol0\"}]},\"mol0\":{\"type\":\"molecule\",\"atoms\":["), "{ket}");
    assert!(
        ket.contains("\"stereo\":1") || ket.contains("\"stereo\":6"),
        "{ket}"
    );
    let cdxml = file("C[C@H](N)C(=O)O", "cdxml");
    assert!(
        cdxml.contains("Display=\"WedgeBegin\"") || cdxml.contains("Display=\"WedgedHashBegin\""),
        "{cdxml}"
    );
    assert!(cdxml.starts_with("<?xml") && cdxml.ends_with("</CDXML>\n"));
    let plain = file("CCO", "cdxml");
    assert!(!plain.contains("Display="), "{plain}");
}

#[test]
fn the_title_is_a_text_label_and_is_escaped() {
    let ask = |format: &str| {
        draw_json(&request(
            "CCO",
            &format!(",\"format\":\"{format}\",\"name\":\"A & B <x> \\\"q\\\"\""),
        ))
        .unwrap()
    };
    let cdxml = ask("cdxml");
    assert!(
        cdxml.contains("A &amp; B &lt;x&gt; &quot;q&quot;</s></t>\n</page>"),
        "{cdxml}"
    );
    let ket = ask("ket");
    let v: serde_json::Value = serde_json::from_str(&ket).unwrap();
    let node = &v["root"]["nodes"][1];
    assert_eq!(node["type"], "text");
    let rich: serde_json::Value =
        serde_json::from_str(node["data"]["content"].as_str().unwrap()).unwrap();
    assert_eq!(rich["blocks"][0]["text"], "A & B <x> \"q\"");
    assert!(
        !file("CCO", "ket").contains("\"text\""),
        "no title, no text node"
    );
    assert_eq!(read_ket(&ket).can(), read_cdxml(&cdxml).can());
}

#[test]
fn the_scene_can_carry_the_file_and_the_bytes_are_fixed() {
    let ask = |file: &str| {
        draw_json(&request(
            "C[C@H](N)C(=O)O",
            &format!(",\"format\":\"scene\",\"file\":\"{file}\""),
        ))
        .unwrap()
    };
    for (kind, plain) in [
        ("ket", file("C[C@H](N)C(=O)O", "ket")),
        ("cdxml", file("C[C@H](N)C(=O)O", "cdxml")),
    ] {
        let s = ask(kind);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert!(s.starts_with("{\"schema\":\"draw/1\""));
        assert_eq!(v["file_format"], kind);
        assert_eq!(v["file"].as_str().unwrap(), plain);
        assert!(
            s.contains(&format!("\"key\":\"file_format\",\"value\":\"{kind}\"")),
            "{s}"
        );
        assert_eq!(ask(kind), s, "deterministic");
    }
    let plain = draw_json(&request("CCO", ",\"format\":\"scene\"")).unwrap();
    assert!(
        !plain.contains("\"file\""),
        "a scene without the option is as before"
    );
}

#[test]
fn a_bad_format_or_file_setting_is_an_error_with_its_name() {
    for extra in [
        ",\"format\":\"png\"",
        ",\"format\":\"molfile\",\"file\":\"ket\"",
        ",\"format\":\"scene\",\"file\":\"molfile\"",
        ",\"format\":\"ket\",\"file\":\"ket\"",
    ] {
        let e = draw_json(&request("CCO", extra)).expect_err(extra);
        assert_eq!(e.key, "err.mol.draw.set", "{extra}");
    }
}

#[test]
fn a_molfile_with_coordinates_keeps_them_in_the_file() {
    let mf = draw_json(&request("C[C@H](N)C(=O)O", "")).unwrap();
    let (a, b) = (file(&mf, "ket"), file("C[C@H](N)C(=O)O", "ket"));
    assert_eq!(
        a, b,
        "the molfile of a drawing gives the same KET as the SMILES"
    );
}

#[test]
fn cdxml_radicals_are_the_enumeration_and_ket_keeps_the_numbers() {
    let cdxml = file("[CH3]", "cdxml");
    assert!(cdxml.contains("Radical=\"Doublet\""), "{cdxml}");
    assert!(!cdxml.contains("Radical=\"2\""), "{cdxml}");
    assert!(file("[CH3]", "ket").contains("\"radical\":2"));
    let two = file("[CH2]", "cdxml");
    assert!(two.contains("Radical=\"Triplet\""), "{two}");
    assert!(!file("CC", "cdxml").contains("Radical"));
}

#[test]
fn cdxml_object_ids_are_unique() {
    let text = file("CC(=O)Oc1ccccc1C(=O)O", "cdxml");
    let mut ids: Vec<&str> = text
        .match_indices(" id=\"")
        .map(|(i, _)| {
            let r = &text[i + 5..];
            &r[..r.find('"').unwrap()]
        })
        .collect();
    let n = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), n, "{text}");
}

#[test]
fn extra_title_lines_are_logged_as_a_choice() {
    let ask = |name: &str| {
        let (d, _) = draw(&request(
            "CCO",
            &format!(",\"format\":\"ket\",\"name\":\"{name}\""),
        ))
        .unwrap();
        d.choices.iter().any(|c| c.key == "file_title")
    };
    assert!(ask("first\\nsecond"));
    assert!(!ask("only one"));
}
