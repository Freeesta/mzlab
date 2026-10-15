//! Families and identities of the ions of the `untargeted` table (`untargeted::ident`): an [M+H]+ and an [M+Na]+ at one RT are one identity with the
//! neutral mass they share, an ion with no link is read as [M+H]+, isotopologues copy their representative, and the signs survive the `.mzfeat`.

use mzlab_core::elements::ion_mz;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::rowident::{adduct_names, SRC_DETECTED};
use mzlab_feat::store::{read_table, write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};
use mzlab_feat::untargeted::ident::{ident, IdentParams};
use mzlab_feat::untargeted::isotopes::{group, IsoGroups, IsoParams};

fn table(ions: &[(f64, f64, f32)]) -> Aligned {
    let mut t = Aligned::with_capacity(1, ions.len());
    for &(mz, rt, h) in ions {
        t.mz.push(mz);
        t.rt.push(rt);
        t.area.push(f64::from(h) * 10.0);
        t.height.push(h);
        t.apex_rt.push(rt as f32);
        t.width.push(0.1);
        t.state.push(CELL_DETECTED);
    }
    t
}

fn code(adduct: &str) -> u8 {
    adduct_names()
        .iter()
        .position(|a| *a == adduct)
        .expect("adduct") as u8
        + 1
}

#[test]
fn an_h_and_an_na_ion_at_one_rt_are_one_identity() {
    let m = 301.1234;
    let t = table(&[
        (ion_mz(m, "[M+H]+").unwrap(), 4.2, 9e5),
        (ion_mz(m, "[M+Na]+").unwrap(), 4.22, 3e5),
        (450.3, 4.2, 1e5),
        (ion_mz(m, "[M+K]+").unwrap(), 7.0, 2e5),
    ]);
    let r = ident(&t, &IsoGroups::default(), 1, &IdentParams::default());
    assert_eq!(r.family, vec![1, 1, 0, 0]);
    assert!((r.ident.neutral[0] - m).abs() < 1e-6 && (r.ident.neutral[1] - m).abs() < 1e-6);
    assert_eq!(
        (r.ident.ion[0], r.ident.ion[1]),
        (code("[M+H]+"), code("[M+Na]+"))
    );
    assert_eq!(
        r.ident.ion[2],
        code("[M+H]+"),
        "alone: read as the protonated ion"
    );
    assert_eq!(
        r.ident.ion[3],
        code("[M+H]+"),
        "same mass but another RT: not linked"
    );
    assert!(r.ident.src.iter().all(|s| s & SRC_DETECTED != 0));
}

#[test]
fn the_anchor_can_be_the_adduct_that_is_not_protonated() {
    // the most intense ion is the sodium adduct: the link is found with the anchor read as [M+Na]+
    let m = 250.0;
    let t = table(&[
        (ion_mz(m, "[M+Na]+").unwrap(), 3.0, 9e5),
        (ion_mz(m, "[M+H]+").unwrap(), 3.0, 1e5),
    ]);
    let r = ident(&t, &IsoGroups::default(), 1, &IdentParams::default());
    assert_eq!(r.family, vec![1, 1]);
    assert_eq!(
        (r.ident.ion[0], r.ident.ion[1]),
        (code("[M+Na]+"), code("[M+H]+"))
    );
}

#[test]
fn negative_mode_links_deprotonated_and_chloride() {
    let m = 180.0634;
    let t = table(&[
        (ion_mz(m, "[M-H]-").unwrap(), 2.0, 5e5),
        (ion_mz(m, "[M+Cl]-").unwrap(), 2.01, 2e5),
    ]);
    let r = ident(&t, &IsoGroups::default(), -1, &IdentParams::default());
    assert_eq!(r.family, vec![1, 1]);
    assert_eq!(
        (r.ident.ion[0], r.ident.ion[1]),
        (code("[M-H]-"), code("[M+Cl]-"))
    );
    assert!((r.ident.neutral[1] - m).abs() < 1e-6);
}

#[test]
fn an_isotopologue_copies_the_identity_of_its_representative() {
    let m = 301.1234;
    let h = ion_mz(m, "[M+H]+").unwrap();
    let t = table(&[
        (h, 4.2, 9e5),
        (h + 1.0033, 4.2, 3e5),
        (ion_mz(m, "[M+Na]+").unwrap(), 4.2, 2e5),
    ]);
    let iso = group(&t, &IsoParams::default());
    let r = ident(&t, &iso, 1, &IdentParams::default());
    assert_eq!(iso.groups.len(), 1);
    assert_eq!(r.family, vec![1, 1, 1]);
    assert_eq!(r.ident.ion[1], r.ident.ion[0]);
    assert_eq!(r.ident.neutral[1], r.ident.neutral[0]);
}

#[test]
fn the_signs_survive_the_container() {
    let m = 301.1234;
    let mut t = table(&[
        (ion_mz(m, "[M+H]+").unwrap(), 4.2, 9e5),
        (ion_mz(m, "[M+Na]+").unwrap(), 4.2, 3e5),
    ]);
    let iso = group(&t, &IsoParams::default());
    let r = ident(&t, &iso, 1, &IdentParams::default());
    t.iso = mzlab_feat::table::IsoBlock {
        group: iso.per_row(2).0,
        charge: iso.per_row(2).1,
        family: r.family.clone(),
    };
    t.ident = r.ident;
    let bytes = write_table(&t, &Recipe::default(), &Origin::default());
    let (back, meta) = read_table(&bytes).expect("read");
    assert_eq!(back.iso, t.iso);
    assert_eq!(back.ident, t.ident);
    assert!(meta.get("isotopes").is_some());
    // a table without the signs does not carry the columns
    let plain = write_table(
        &table(&[(200.0, 1.0, 1e5)]),
        &Recipe::default(),
        &Origin::default(),
    );
    assert!(!String::from_utf8_lossy(&plain).contains("iso_group"));
}

#[test]
fn the_recipe_sets_or_leaves_out_the_signs() {
    use mzlab_feat::untargeted::isotopes::Representative;
    use mzlab_feat::untargeted::pipeline::UntParams;
    let on = UntParams::default();
    assert!(on.iso.is_some(), "on unless the recipe says otherwise");
    let p = UntParams::from_json(
        r#"{"iso":{"max_charge":3,"monotonic":true,"representative":"lowest_mz","rt_tol":0.2},"ident":{"rt_tol":0.3,"win":-0.5}}"#,
    )
    .unwrap();
    let (i, d) = p.iso.unwrap();
    assert_eq!(
        (i.max_charge, i.monotonic, i.representative, i.rt_tol),
        (3, true, Representative::LowestMz, 0.2)
    );
    assert_eq!((d.rt_tol, d.win), (0.3, -0.5));
    assert!(UntParams::from_json(r#"{"iso":null}"#)
        .unwrap()
        .iso
        .is_none());
    assert!(UntParams::from_json(r#"{"iso":false}"#)
        .unwrap()
        .iso
        .is_none());
    assert!(UntParams::from_json(r#"{"iso":{"representative":"x"}}"#).is_err());
    assert!(UntParams::from_json(r#"{"iso":{"rt_tol":-1}}"#).is_err());
}
