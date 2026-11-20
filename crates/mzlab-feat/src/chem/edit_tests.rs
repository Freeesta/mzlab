//! The edit patch of `draw`: each operation, hydrogens and stereo after an edit, untouched coordinates, all-or-none and the keyed errors.

use crate::chem::draw::draw_json;
use mzlab_core::error::Error;
use mzlab_core::json::{self, Out, Value};
use mzlab_mol::Mol;

fn request(spec: &str, format: &str, edit: &str) -> String {
    let mut o = Out::default();
    o.raw("{\"spec\":")
        .str(spec)
        .raw(",\"format\":")
        .str(format);
    o.raw(",\"edit\":").str(edit).raw("}");
    o.0
}

fn molfile(spec: &str, edit: &str) -> String {
    draw_json(&request(spec, "molfile", edit)).unwrap()
}

fn scene(spec: &str, edit: &str) -> Value {
    json::parse(draw_json(&request(spec, "scene", edit)).unwrap().as_bytes()).unwrap()
}

fn failure(spec: &str, edit: &str) -> Error {
    draw_json(&request(spec, "scene", edit)).expect_err("an error")
}

fn param<'a>(e: &'a Error, k: &str) -> &'a str {
    e.params
        .iter()
        .find(|p| p.0 == k)
        .map_or("", |p| p.1.as_str())
}

fn can(molfile: &str) -> String {
    Mol::from_molfile(molfile).unwrap().can()
}

fn choice(s: &Value, key: &str) -> Option<String> {
    s.get("choices")?
        .arr()
        .iter()
        .find(|c| c.get("key").and_then(Value::str) == Some(key))
        .and_then(|c| c.get("value").and_then(Value::str))
        .map(str::to_string)
}

#[test]
fn a_methyl_on_benzene_is_toluene_and_the_ring_stays_where_it_was() {
    let edit = r#"[{"op":"atom.add","el":"C"},{"op":"bond.add","a":0,"b":6}]"#;
    let out = molfile("c1ccccc1", edit);
    assert_eq!(can(&out), Mol::from_smiles("Cc1ccccc1").unwrap().can());
    let before = molfile("c1ccccc1", "[]");
    let rows = |t: &str| {
        t.lines()
            .skip(4)
            .take(6)
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        rows(&before),
        rows(&out),
        "untouched atoms keep their coordinates"
    );
    let s = scene("c1ccccc1", edit);
    assert_eq!(choice(&s, "edit_added").as_deref(), Some("atom ids 6"));
    assert!(choice(&s, "edit_placed").is_some());
}

#[test]
fn the_object_form_and_an_embedded_list_give_the_same_bytes_as_the_text() {
    let text = r#"[{"op":"atom.set","id":2,"el":"S"}]"#;
    let by_text = draw_json(&request("CCO", "molfile", text)).unwrap();
    let embedded = r#"{"spec":"CCO","edit":[{"op":"atom.set","id":2,"el":"S"}]}"#;
    let object = r#"{"spec":"CCO","edit":{"ops":[{"op":"atom.set","id":2,"el":"S"}]}}"#;
    assert_eq!(draw_json(embedded).unwrap(), by_text);
    assert_eq!(draw_json(object).unwrap(), by_text);
    assert_eq!(can(&by_text), Mol::from_smiles("CCS").unwrap().can());
}

#[test]
fn atoms_are_edited_and_their_hydrogens_counted_again() {
    let h = |spec: &str, edit: &str, atom: usize| {
        scene(spec, edit).get("atoms").unwrap().arr()[atom]
            .get("h")
            .and_then(Value::f64)
            .unwrap() as u8
    };
    assert_eq!(h("CCN", r#"[{"op":"atom.set","id":2,"chg":1}]"#, 2), 3);
    assert_eq!(h("CC", r#"[{"op":"atom.set","id":1,"rad":2}]"#, 1), 1);
    assert_eq!(h("CC", r#"[{"op":"bond.set","a":0,"b":1,"o":2}]"#, 0), 2);
    assert_eq!(h("CCC", r#"[{"op":"bond.add","a":0,"b":2}]"#, 0), 2);
    assert_eq!(h("CC", r#"[{"op":"atom.set","id":0,"h":0}]"#, 0), 0);
    let o = molfile("CCO", r#"[{"op":"atom.set","id":2,"iso":18}]"#);
    assert_eq!(can(&o), Mol::from_smiles("CC[18OH]").unwrap().can());
}

#[test]
fn remove_and_merge_renumber_the_atoms_in_order() {
    let s = scene("CCO", r#"[{"op":"remove","atoms":[0]}]"#);
    assert_eq!(s.get("atoms").unwrap().arr().len(), 2);
    assert_eq!(
        s.get("bonds").unwrap().arr()[0]
            .get("a")
            .and_then(Value::f64),
        Some(0.0)
    );
    let cut = scene("CCO", r#"[{"op":"remove","bonds":[[1,2]]}]"#);
    assert_eq!(cut.get("fragments").unwrap().arr().len(), 2);
    // two pieces fused at an atom: the second piece moves so that the atoms coincide
    let m = scene("C.CO", r#"[{"op":"merge","from":1,"into":0}]"#);
    assert_eq!(m.get("atoms").unwrap().arr().len(), 2);
    assert_eq!(m.get("fragments").unwrap().arr().len(), 1);
    let at = |i: usize, k: &str| {
        m.get("atoms").unwrap().arr()[i]
            .get(k)
            .and_then(Value::f64)
            .unwrap()
    };
    let d = ((at(0, "x_a") - at(1, "x_a")).powi(2) + (at(0, "y_a") - at(1, "y_a")).powi(2)).sqrt();
    assert!((d - 1.5).abs() < 1e-3, "bond length {d}");
}

#[test]
fn move_takes_bond_lengths_or_angstrom() {
    let at = |edit: &str| {
        let s = scene("CCO", edit);
        let a = &s.get("atoms").unwrap().arr()[2];
        (
            a.get("x_a").and_then(Value::f64).unwrap(),
            a.get("y_a").and_then(Value::f64).unwrap(),
        )
    };
    let (x0, y0) = at("[]");
    let (x1, y1) = at(r#"[{"op":"move","atoms":[2],"dx":1,"dy":-0.5}]"#);
    assert!((x1 - x0 - 1.5).abs() < 1e-9 && (y1 - y0 + 0.75).abs() < 1e-9);
    let (x2, y2) = at(r#"[{"op":"move","atoms":[2],"dx_a":1.5,"dy_a":-0.75}]"#);
    assert_eq!((x1, y1), (x2, y2));
}

#[test]
fn a_wedge_set_by_the_patch_decides_the_stereo() {
    let ala = "N[C@@H](C)C(=O)O";
    let flip = |w: i8| {
        molfile(
            ala,
            &format!(r#"[{{"op":"bond.set","a":1,"b":0,"wedge":{w}}}]"#),
        )
    };
    let (up, down) = (flip(1), flip(-1));
    assert_eq!(can(&up), Mol::from_smiles("N[C@H](C)C(=O)O").unwrap().can());
    assert_eq!(can(&down), Mol::from_smiles(ala).unwrap().can());
    assert_ne!(can(&up), can(&down));
}

#[test]
fn stereo_of_an_atom_whose_bonds_changed_is_dropped_and_logged() {
    let s = scene("N[C@@H](C)C(=O)O", r#"[{"op":"remove","atoms":[2]}]"#);
    assert!(choice(&s, "edit_stereo_dropped").is_some_and(|c| c.contains('1')));
    let flat = s
        .get("bonds")
        .unwrap()
        .arr()
        .iter()
        .all(|b| b.get("wedge").and_then(Value::f64) == Some(0.0));
    assert!(flat, "a molecule without stereo has no wedge");
    // untouched stereo stays
    let kept = scene("N[C@@H](C)C(=O)O", r#"[{"op":"atom.set","id":5,"chg":0}]"#);
    assert!(choice(&kept, "edit_stereo_dropped").is_none());
    assert!(kept
        .get("bonds")
        .unwrap()
        .arr()
        .iter()
        .any(|b| b.get("wedge").and_then(Value::f64) != Some(0.0)));
}

#[test]
fn a_label_is_in_the_scene_and_a_v2000_molfile_alias_that_reads_back() {
    let edit = r#"[{"op":"atom.set","id":5,"label":"Ph"}]"#;
    let s = scene("c1ccccc1", edit);
    assert_eq!(
        s.get("atoms").unwrap().arr()[5]
            .get("label")
            .and_then(Value::str),
        Some("Ph")
    );
    let m = molfile("c1ccccc1", edit);
    assert!(m.contains("A    6\nPh\nM  END\n"), "{m}");
    assert_eq!(can(&m), Mol::from_smiles("c1ccccc1").unwrap().can());
    let cleared = scene(
        "c1ccccc1",
        r#"[{"op":"atom.set","id":5,"label":"Ph"},{"op":"atom.set","id":5,"label":""}]"#,
    );
    assert!(cleared.get("atoms").unwrap().arr()[5]
        .get("label")
        .is_none());
}

#[test]
fn a_failed_operation_names_its_index_and_reason_and_changes_nothing() {
    let e = failure(
        "CCO",
        r#"[{"op":"atom.set","id":0,"el":"N"},{"op":"bond.add","a":1,"b":9}]"#,
    );
    assert_eq!(
        (
            e.key,
            param(&e, "index"),
            param(&e, "op"),
            param(&e, "reason")
        ),
        ("err.mol.draw.edit", "1", "bond.add", "unknown_atom")
    );
    let reasons = [
        (r#"[{"op":"atom.zap"}]"#, "unknown_op"),
        (r#"[{"op":"atom.set","id":0,"elem":"N"}]"#, "unknown_field"),
        (r#"[{"op":"atom.add"}]"#, "missing_field"),
        (r#"[{"op":"atom.add","el":"Qq"}]"#, "bad_field"),
        (r#"[{"op":"atom.add","id":1,"el":"C"}]"#, "atom_exists"),
        (
            r#"[{"op":"atom.add","el":"C","x":1,"y":1,"x_a":1}]"#,
            "units_mixed",
        ),
        (r#"[{"op":"atom.add","el":"C","x":1}]"#, "missing_field"),
        (r#"[{"op":"bond.add","a":0,"b":1}]"#, "bond_exists"),
        (r#"[{"op":"bond.add","a":0,"b":0}]"#, "self_bond"),
        (r#"[{"op":"bond.set","a":0,"b":2,"o":2}]"#, "unknown_bond"),
        (r#"[{"op":"bond.set","a":0,"b":1,"o":4}]"#, "bad_field"),
        (r#"[{"op":"bond.set","a":0,"b":1}]"#, "missing_field"),
        (r#"[{"op":"remove"}]"#, "missing_field"),
        (r#"[{"op":"move","atoms":[0]}]"#, "missing_field"),
        (r#"[{"op":"merge","from":0,"into":0}]"#, "self_bond"),
        (
            r#"[{"op":"atom.add","el":"C"},{"op":"move","atoms":[3],"dx":1,"dy":1}]"#,
            "unplaced",
        ),
        (r#"[5]"#, "bad_op"),
    ];
    for (edit, want) in reasons {
        let e = failure("CCO", edit);
        assert_eq!(
            (e.key, param(&e, "reason")),
            ("err.mol.draw.edit", want),
            "{edit}"
        );
    }
    for edit in ["nope", "5", r#"{"ops":[],"x":1}"#] {
        assert_eq!(failure("CCO", edit).key, "err.mol.draw.editJson", "{edit}");
    }
}

#[test]
fn a_molfile_keeps_its_coordinates_and_an_empty_patch_changes_nothing() {
    let base = molfile("CC(=O)Nc1ccc(O)cc1", "[]");
    let plain = draw_json(&request("CC(=O)Nc1ccc(O)cc1", "molfile", "[]")).unwrap();
    assert_eq!(base, plain);
    let more = molfile(
        &base,
        r#"[{"op":"atom.add","el":"Cl"},{"op":"bond.add","a":0,"b":11}]"#,
    );
    let rows = |t: &str| {
        t.lines()
            .skip(4)
            .take(11)
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    assert_eq!(rows(&base), rows(&more));
}
