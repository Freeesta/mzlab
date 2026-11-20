//! The 2D layout on the two synthetic sets of `tools/draw_quality.py` (tests/dati/draw_set_a.tsv, draw_set_b.tsv): every drawn
//! molecule survives `from_molfile(to_molfile(..))` with the same `mol/1` core, stereo included; defined double bonds are drawn as
//! written; pieces sit at least 1.5 bond lengths apart; the same input gives the same bytes; fixed atoms stay where they are.

use mzlab_mol::{Mol, Stereo};

fn rows(name: &str) -> Vec<(String, String)> {
    let p = format!("{}/../../tests/dati/{name}.tsv", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
    text.lines()
        .skip(1)
        .map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            (c[0].to_string(), c[1].to_string())
        })
        .collect()
}

fn drawn(smiles: &str) -> Mol {
    let mut m = Mol::from_smiles(smiles).unwrap_or_else(|e| panic!("{smiles}: {e}"));
    m.coords = Some(m.layout_2d());
    m
}

#[test]
fn every_drawn_molecule_reads_back_with_the_same_core() {
    let (mut n, mut bad) = (0, Vec::new());
    for set in ["draw_set_a", "draw_set_b"] {
        for (id, smi) in rows(set) {
            let m = drawn(&smi);
            let back = Mol::from_molfile(&m.to_molfile("")).unwrap_or_else(|e| panic!("{id} {smi}: {e}"));
            if back.core_json() != m.core_json() {
                bad.push(format!("{id} {smi}"));
            }
            n += 1;
        }
    }
    assert_eq!(n, 300);
    assert!(bad.is_empty(), "{} differ: {:#?}", bad.len(), bad);
}

#[test]
fn defined_double_bonds_come_out_as_written() {
    let mut defined = 0;
    for set in ["draw_set_a", "draw_set_b"] {
        for (id, smi) in rows(set) {
            let m = drawn(&smi);
            defined += m.stereo.iter().filter(|s| matches!(s, Stereo::Db { .. })).count();
            assert!(m.ez_mismatches().is_empty(), "{id} {smi}: {:?}", m.ez_mismatches());
        }
    }
    assert!(defined >= 100, "only {defined} defined double bonds in the sets");
}

#[test]
fn pieces_are_at_least_one_and_a_half_bonds_apart() {
    for (id, smi) in rows("draw_set_a") {
        let m = drawn(&smi);
        let xy = &m.coords.as_ref().map(|c| c.xy.clone()).unwrap_or_default();
        let pieces = m.layout_pieces();
        for (i, p) in pieces.iter().enumerate() {
            for q in &pieces[..i] {
                for &a in p {
                    for &b in q {
                        let d = ((xy[a][0] - xy[b][0]).powi(2) + (xy[a][1] - xy[b][1]).powi(2)).sqrt();
                        assert!(d >= 1.5 * 1.5, "{id} {smi}: {d}");
                    }
                }
            }
        }
    }
}

#[test]
fn the_layout_is_deterministic_and_keeps_fixed_atoms() {
    let m = Mol::from_smiles("CC(=O)Oc1ccccc1C(=O)O").unwrap();
    assert_eq!(m.layout_2d(), m.layout_2d());
    let mut fixed = vec![None; m.atoms.len()];
    fixed[0] = Some([10.0, 5.0]);
    fixed[1] = Some([11.5, 5.0]);
    let c = m.layout_2d_fixed(&fixed);
    assert_eq!(c.xy[0], vec![10.0, 5.0]);
    assert_eq!(c.xy[1], vec![11.5, 5.0]);
    let far = c.xy.iter().map(|p| (p[0] - 10.0).hypot(p[1] - 5.0)).fold(0.0, f64::max);
    assert!(far < 20.0, "the rest is placed around the fixed atoms: {far}");
}
