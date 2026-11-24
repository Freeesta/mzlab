//! `stereo.group`: groups set and cleared, the exact inverse from the scene `stg`, refusals on atoms that are no defined centre, groups across
//! pieces, renumbering, and groups after bond changes and wedges.

use crate::chem::draw::draw_json;
use mzlab_core::error::Error;
use mzlab_core::json::{self, Out, Value};
use mzlab_mol::Mol;

/// Three centres in and1, or1 and and2 (atoms 1, 4, 7).
const THREE: &str = "C[C@H](O)C[C@@H](N)C[C@H](F)Cl |&1:1,o1:4,&2:7|";

fn request(spec: &str, format: &str, edit: &str) -> String {
    let mut o = Out::default();
    o.raw("{\"spec\":")
        .str(spec)
        .raw(",\"format\":")
        .str(format);
    o.raw(",\"edit\":").str(edit).raw("}");
    o.0
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

/// The `stg` of every scene atom ("" when in no group).
fn stg(s: &Value) -> Vec<String> {
    s.get("atoms")
        .unwrap()
        .arr()
        .iter()
        .map(|a| a.get("stg").and_then(Value::str).unwrap_or("").to_string())
        .collect()
}

fn choice(s: &Value, key: &str) -> Option<String> {
    s.get("choices")?
        .arr()
        .iter()
        .find(|c| c.get("key").and_then(Value::str) == Some(key))
        .and_then(|c| c.get("value").and_then(Value::str))
        .map(str::to_string)
}

/// The patch that undoes a group change, from the `stg` before and after: each atom that differs goes back to its own group.
fn inverse(before: &[String], after: &[String]) -> String {
    let ops: Vec<String> = before
        .iter()
        .zip(after)
        .enumerate()
        .filter(|(_, (b, a))| b != a)
        .map(|(i, (b, _))| {
            let g = if b.is_empty() { "none" } else { b };
            format!(r#"{{"op":"stereo.group","atoms":[{i}],"g":"{g}"}}"#)
        })
        .collect();
    format!("[{}]", ops.join(","))
}

/// The molfile of a drawing given as a molfile, so that the inverse runs on the record the patch made.
fn molfile(spec: &str, edit: &str) -> String {
    draw_json(&request(spec, "molfile", edit)).unwrap()
}

#[test]
fn the_scene_names_the_group_of_each_atom_as_the_writers_number_them() {
    let s = scene(&format!("smiles:{THREE}"), "[]");
    let g = stg(&s);
    assert_eq!(
        (&*g[1], &*g[4], &*g[7], &*g[0]),
        ("and1", "or1", "and2", "")
    );
    // the V3000 molfile of the same drawing reads back with the same groups
    let m = Mol::from_molfile(&molfile(&format!("smiles:{THREE}"), "[]")).unwrap();
    assert_eq!(m.groups, Mol::from_smiles(THREE).unwrap().groups);
}

#[test]
fn a_group_is_set_cleared_and_undone_exactly_by_its_inverse() {
    let start = molfile(&format!("smiles:{THREE}"), "[]");
    let spec = format!("smiles:{THREE}");
    let b = stg(&scene(&spec, "[]"));
    for patch in [
        r#"[{"op":"stereo.group","atoms":[7],"g":"and1"}]"#,
        r#"[{"op":"stereo.group","atoms":[1,4],"g":"abs"}]"#,
        r#"[{"op":"stereo.group","atoms":[1],"g":"none"}]"#,
        r#"[{"op":"stereo.group","atoms":[4],"g":"and1"},{"op":"stereo.group","atoms":[7],"g":"or5"}]"#,
        r#"[{"op":"stereo.group","atoms":[1,4,7],"g":"none"}]"#,
    ] {
        let after = scene(&spec, patch);
        let a = stg(&after);
        assert_ne!(a, b, "{patch}");
        let edited = molfile(&spec, patch);
        let back = draw_json(&request(&edited, "molfile", &inverse(&b, &a))).unwrap();
        let undone = Mol::from_molfile(&back).unwrap();
        assert_eq!(
            undone.groups,
            Mol::from_molfile(&start).unwrap().groups,
            "{patch}"
        );
        assert_eq!(
            undone.can(),
            Mol::from_molfile(&start).unwrap().can(),
            "{patch}"
        );
    }
}

#[test]
fn group_numbers_are_literal_in_the_patch_and_renumbered_at_the_end() {
    let spec = format!("smiles:{THREE}");
    // and1 empties: and2 becomes and1, and the atom whose number changed is logged
    let s = scene(&spec, r#"[{"op":"stereo.group","atoms":[1],"g":"or1"}]"#);
    let g = stg(&s);
    assert_eq!((&*g[1], &*g[4], &*g[7]), ("or1", "or1", "and1"));
    assert_eq!(choice(&s, "edit_group_renumbered").as_deref(), Some("7"));
    // a new number past the last one is the next group of its kind
    let s = scene(&spec, r#"[{"op":"stereo.group","atoms":[4],"g":"and9"}]"#);
    assert_eq!(stg(&s)[4], "and3");
    assert_eq!(choice(&s, "edit_group_renumbered").as_deref(), Some("4"));
    // no renumbering, no choice
    let s = scene(&spec, r#"[{"op":"stereo.group","atoms":[4],"g":"and2"}]"#);
    assert_eq!(choice(&s, "edit_group_renumbered"), None);
    assert_eq!(choice(&s, "edit").as_deref(), Some("1"));
}

#[test]
fn a_group_spans_the_pieces_of_a_drawing() {
    let spec = "smiles:C[C@H](O)CC.N[C@@H](C)C(=O)O";
    let s = scene(spec, r#"[{"op":"stereo.group","atoms":[1,6],"g":"or1"}]"#);
    let g = stg(&s);
    assert_eq!((&*g[1], &*g[6]), ("or1", "or1"));
    assert_eq!(s.get("fragments").unwrap().arr().len(), 2);
    let m = Mol::from_molfile(&molfile(
        spec,
        r#"[{"op":"stereo.group","atoms":[1,6],"g":"or1"}]"#,
    ))
    .unwrap();
    assert_eq!(m.groups.len(), 1);
    assert_eq!(m.groups[0].atoms, vec![1, 6]);
}

#[test]
fn an_atom_that_is_no_defined_centre_is_refused_and_nothing_changes() {
    let spec = format!("smiles:{THREE}");
    for (patch, index, detail) in [
        (
            r#"[{"op":"stereo.group","atoms":[0],"g":"and1"}]"#,
            "0",
            "0",
        ),
        (
            r#"[{"op":"atom.set","id":0,"chg":0},{"op":"stereo.group","atoms":[1,3],"g":"or1"}]"#,
            "1",
            "3",
        ),
        // the centre loses a bond in the same patch: its stereo goes, so it cannot be grouped
        (
            r#"[{"op":"stereo.group","atoms":[1],"g":"and1"},{"op":"remove","atoms":[2]}]"#,
            "0",
            "1",
        ),
        // an atom added by the patch has no stereo
        (
            r#"[{"op":"atom.add","el":"C"},{"op":"bond.add","a":0,"b":10},{"op":"stereo.group","atoms":[10],"g":"abs"}]"#,
            "2",
            "10",
        ),
    ] {
        let e = failure(&spec, patch);
        assert_eq!(e.key, "err.mol.draw.edit", "{patch}");
        assert_eq!(
            (
                param(&e, "reason"),
                param(&e, "index"),
                param(&e, "op"),
                param(&e, "detail")
            ),
            ("not_stereocentre", index, "stereo.group", detail),
            "{patch}"
        );
    }
    // unset on any existing atom is accepted: an undo never fails because the centre went
    let s = scene(&spec, r#"[{"op":"stereo.group","atoms":[0,3],"g":"none"}]"#);
    assert_eq!(stg(&s), stg(&scene(&spec, "[]")));
    for (patch, reason) in [
        (
            r#"[{"op":"stereo.group","atoms":[99],"g":"and1"}]"#,
            "unknown_atom",
        ),
        (
            r#"[{"op":"stereo.group","atoms":[],"g":"and1"}]"#,
            "missing_field",
        ),
        (r#"[{"op":"stereo.group","atoms":[1]}]"#, "missing_field"),
        (
            r#"[{"op":"stereo.group","atoms":[1],"g":"and0"}]"#,
            "bad_field",
        ),
        (
            r#"[{"op":"stereo.group","atoms":[1],"g":"and01"}]"#,
            "bad_field",
        ),
        (
            r#"[{"op":"stereo.group","atoms":[1],"g":"rac1"}]"#,
            "bad_field",
        ),
        (
            r#"[{"op":"stereo.group","atoms":[1],"g":"or2001"}]"#,
            "bad_field",
        ),
        (r#"[{"op":"stereo.group","atoms":[1],"g":1}]"#, "bad_field"),
        (
            r#"[{"op":"stereo.group","atoms":[1],"g":"and1","kind":"x"}]"#,
            "unknown_field",
        ),
    ] {
        assert_eq!(param(&failure(&spec, patch), "reason"), reason, "{patch}");
    }
}

#[test]
fn a_centre_whose_bonds_change_leaves_its_group_and_both_are_logged() {
    let spec = format!("smiles:{THREE}");
    let s = scene(&spec, r#"[{"op":"remove","atoms":[2]}]"#);
    assert_eq!(choice(&s, "edit_stereo_dropped").as_deref(), Some("1"));
    assert_eq!(choice(&s, "edit_group_dropped").as_deref(), Some("1"));
    // the other groups keep their atoms; the and2 centre is now the first and group
    let g = stg(&s);
    assert_eq!((&*g[1], &*g[3], &*g[6]), ("", "or1", "and1"));
    assert_eq!(choice(&s, "edit_group_renumbered").as_deref(), Some("6"));
}

#[test]
fn a_wedge_on_a_drawing_with_groups_keeps_them_and_can_make_a_centre_a_group_names() {
    let spec = format!("smiles:{THREE}");
    // a wedge elsewhere: the groups survive the read-back (before, a drawing with groups refused every wedge)
    let s = scene(&spec, r#"[{"op":"bond.set","a":1,"b":0,"wedge":-1}]"#);
    assert_eq!(stg(&s), stg(&scene(&spec, "[]")));
    // a centre made by a wedge of the same patch can go into a group, whatever the order of the two operations
    let base = "smiles:CC(O)CC";
    for patch in [
        r#"[{"op":"bond.set","a":1,"b":0,"wedge":1},{"op":"stereo.group","atoms":[1],"g":"or1"}]"#,
        r#"[{"op":"stereo.group","atoms":[1],"g":"or1"},{"op":"bond.set","a":1,"b":0,"wedge":1}]"#,
    ] {
        assert_eq!(stg(&scene(base, patch))[1], "or1", "{patch}");
    }
}

#[test]
fn a_patch_without_a_group_operation_keeps_the_groups_byte_for_byte() {
    let spec = format!("smiles:{THREE}");
    let plain = draw_json(&request(&spec, "molfile", "[]")).unwrap();
    let moved = draw_json(&request(
        &spec,
        "molfile",
        r#"[{"op":"move","atoms":[0],"dx":0,"dy":0}]"#,
    ))
    .unwrap();
    assert_eq!(plain, moved);
}

#[test]
fn two_abs_groups_are_both_abs_and_never_logged_as_renumbered() {
    let spec = "smiles:C[C@H](O)C[C@@H](N)C |a:1,a:4|";
    let s = scene(spec, r#"[{"op":"stereo.group","atoms":[1],"g":"none"}]"#);
    assert_eq!((&*stg(&s)[1], &*stg(&s)[4]), ("", "abs"));
    assert_eq!(choice(&s, "edit_group_renumbered"), None);
}
