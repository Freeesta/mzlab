//! `interp/1`: the labels of the modification, the parts of the MS2 link (filter text, purity in time, isomers by the traces of the fragments), the
//! comparison with the tree, the calibration and the registry of the filters, and the block in the `.mzfeat` (written, read by row, replaced).

use mzlab_core::elements::parse_formula;
use mzlab_feat::interp::albero::{states, LibNode};
use mzlab_feat::interp::confronto::by_formula;
use mzlab_feat::interp::filtri::{calibrate, first_hidden, out_of_tolerance, RowFacts};
use mzlab_feat::interp::modifica::{candidates, diff, iso_check, label};
use mzlab_feat::interp::ms2link::{interpolate, parse_filter, pearson, separate, Isomers, Ms2Scan};
use mzlab_feat::interp::store::{append, read_meta, read_row};
use mzlab_feat::legacy::ms2::Consensus;
use mzlab_feat::legacy::series::Hyp;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{read_table, write_table, Origin};
use mzlab_feat::table::Aligned;

fn f(s: &str) -> Vec<(String, u64)> {
    parse_formula(s).unwrap()
}

#[test]
fn labels_use_the_true_minus_and_put_gains_first() {
    let p = f("C13H24N4O3S");
    assert_eq!(label(&diff(&f("C13H24N4O4S"), &p)), "+O");
    assert_eq!(label(&diff(&f("C13H22N4O3S"), &p)), "\u{2212}2H");
    assert_eq!(label(&diff(&f("C13H22N4O4S"), &p)), "+O\u{2212}2H");
    assert_eq!(label(&diff(&f("C12H22N4O3S"), &p)), "\u{2212}C\u{2212}2H");
    assert_eq!(label(&diff(&p, &p)), "");
}

fn hyp(formula: &str, mz: f64, src: &str) -> Hyp {
    Hyp {
        label: "x".into(),
        formula: formula.into(),
        mz,
        level: 3,
        sources: vec![src.into()],
        alternatives: vec![],
    }
}

#[test]
fn the_closest_hypothesis_is_the_change_and_the_others_alternatives() {
    let p = f("C13H24N4O3S");
    let (a, b, far) = (
        hyp("C13H24N4O4S", 333.1591, "regola"),
        hyp("C13H24N4O3S2", 333.1593, "rete"),
        hyp("C13H22N4O3S", 315.0, "regola"),
    );
    let c = candidates(333.1591, &p, &[&b, &a, &far]);
    assert_eq!(c.len(), 2);
    assert_eq!((c[0].delta.as_str(), c[0].source), ("+O", "rule"));
    assert_eq!(c[0].ion_formula, "C13H25N4O4S");
    assert!(c[0].ppm.abs() < 0.1 && c[1].ppm.abs() > c[0].ppm.abs());
}

#[test]
fn sulfur_count_follows_the_m_plus_2_of_the_formula() {
    let ion = "C13H25N4O3S";
    let (m1, m2) = mzlab_core::composition::isotope_ratios(
        &f(ion)
            .iter()
            .map(|(e, n)| (e.clone(), *n as i64))
            .collect::<Vec<_>>(),
    );
    let ok = iso_check(ion, [100.0, 100.0 * m1, 100.0 * m2]).unwrap();
    assert_eq!(ok.s_count, "consistent");
    assert_eq!(
        iso_check(ion, [100.0, 100.0 * m1, 100.0 * m2 * 0.2])
            .unwrap()
            .s_count,
        "inconsistent"
    );
    assert_eq!(
        iso_check(ion, [100.0, 20.0, 0.0]).unwrap().s_count,
        "unknown"
    );
    assert!(iso_check(ion, [0.0, 0.0, 0.0]).is_none());
}

#[test]
fn filter_text_gives_energy_and_scan_range() {
    let (ce, range) = parse_filter("FTMS + c ESI d Full ms2 337.1700@hcd30.00 [50.0000-355.0000]");
    assert_eq!((ce, range), (Some(30.0), Some((50.0, 355.0))));
    assert_eq!(
        parse_filter("FTMS + p ESI Full ms [100.0-1500.0]"),
        (None, Some((100.0, 1500.0)))
    );
    assert_eq!(parse_filter("no brackets"), (None, None));
}

#[test]
fn purity_is_interpolated_between_the_ms1_scans() {
    assert!((interpolate(1.5, Some((1.0, 0.6)), Some((2.0, 1.0))) - 0.8).abs() < 1e-12);
    assert_eq!(interpolate(1.5, Some((1.0, 0.6)), None), 0.6);
    assert_eq!(interpolate(1.5, None, Some((2.0, 0.9))), 0.9);
    assert!(interpolate(1.5, None, None).is_nan());
}

fn scan(rt: f64, peaks: &[(f64, f64)]) -> Ms2Scan {
    Ms2Scan {
        file: 0,
        scan: 0,
        rt,
        precursor: 400.0,
        iso: (399.0, 401.0),
        ce: None,
        scan_range: None,
        purity: 1.0,
        chimeric: false,
        mz: peaks.iter().map(|p| p.0).collect(),
        it: peaks.iter().map(|p| p.1).collect(),
    }
}

#[test]
fn fragments_that_rise_apart_split_the_scans_into_two_isomers() {
    // fragments 100 and 110 follow isomer A (early), 200 and 210 isomer B (late)
    let a = [10.0, 80.0, 100.0, 40.0, 5.0, 1.0, 0.0, 0.0];
    let b = [0.0, 0.0, 1.0, 5.0, 40.0, 100.0, 80.0, 10.0];
    let scans: Vec<Ms2Scan> = (0..8)
        .map(|i| {
            scan(
                i as f64,
                &[
                    (100.0, a[i]),
                    (110.0, a[i] * 0.5),
                    (200.0, b[i]),
                    (210.0, b[i] * 0.7),
                ],
            )
        })
        .collect();
    let refs: Vec<&Ms2Scan> = scans.iter().collect();
    let cons = Consensus {
        mz: vec![100.0, 110.0, 200.0, 210.0],
        rel: vec![100.0, 50.0, 90.0, 60.0],
        n_found: vec![8; 4],
        n_scans: 8,
    };
    match separate(&refs, &cons) {
        Isomers::Separated(g) => assert_eq!(g.iter().map(Vec::len).sum::<usize>(), 8),
        other => panic!("{other:?}"),
    }
    // one isomer: all the fragments rise and fall together
    let one: Vec<Ms2Scan> = (0..8)
        .map(|i| {
            scan(
                i as f64,
                &[(100.0, a[i]), (110.0, a[i] * 0.5), (200.0, a[i] * 0.3)],
            )
        })
        .collect();
    let refs: Vec<&Ms2Scan> = one.iter().collect();
    let cons = Consensus {
        mz: vec![100.0, 110.0, 200.0],
        rel: vec![100.0, 50.0, 30.0],
        n_found: vec![8; 3],
        n_scans: 8,
    };
    assert_eq!(separate(&refs, &cons), Isomers::Single);
    // fewer than four scans: no attempt
    assert_eq!(separate(&refs[..3], &cons), Isomers::Single);
    assert!((pearson(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]) - 1.0).abs() < 1e-12);
}

fn node(id: &str, mz: f64) -> LibNode {
    LibNode {
        id: id.into(),
        formula: "F".into(),
        mz,
    }
}

#[test]
fn tree_nodes_are_same_shifted_absent_or_not_checkable() {
    let nodes = [
        node("MS1", 317.16),
        node("1", 200.0),
        node("2", 150.0),
        node("3", 120.0),
        node("4", 20.0),
    ];
    let peaks = [200.0001, 166.0 + 0.0001, 90.0];
    let s = states(&nodes, &peaks, 16.0, Some((50.0, 335.0)), 333.16);
    let of = |id: &str| s.iter().find(|n| n.id == id).unwrap().state;
    assert_eq!(of("1"), "same");
    assert_eq!(of("2"), "shifted");
    assert_eq!(of("3"), "absent");
    assert_eq!(of("4"), "not_checkable");
    assert!(s.iter().all(|n| n.id != "MS1"));
}

#[test]
fn fragments_are_matched_to_the_library_by_formula_mass() {
    let lib = [node("", 200.0), node("", 150.0)];
    let (equal, shifted) = by_formula(&[200.0004, 166.0, 77.0], &lib, 16.0);
    assert_eq!((equal, shifted), (vec![200.0004], vec![166.0]));
}

#[test]
fn calibration_tolerance_is_between_three_and_five_ppm() {
    let c = calibrate(&[1.0, 1.2, 0.8, 1.1, 0.9]);
    assert!((c.median_ppm - 1.0).abs() < 1e-9 && c.tolerance == 3.0);
    assert_eq!(calibrate(&[-9.0, 0.0, 9.0, -9.0, 9.0]).tolerance, 5.0);
    assert_eq!(calibrate(&[]).tolerance, 3.0);
    assert!(!out_of_tolerance(3.9, &c) && out_of_tolerance(4.5, &c));
}

#[test]
fn the_first_filter_that_hides_a_row_names_it() {
    let base = RowFacts::default();
    assert_eq!(first_hidden(&base), None);
    assert_eq!(
        first_hidden(&RowFacts {
            bad_quality: true,
            tp_status: 1,
            ..base
        }),
        Some(0)
    );
    assert_eq!(
        first_hidden(&RowFacts {
            tp_status: 1,
            ..base
        }),
        Some(2)
    );
    assert_eq!(
        first_hidden(&RowFacts {
            tp_status: 2,
            mass_error: true,
            ..base
        }),
        Some(3)
    );
    assert_eq!(
        first_hidden(&RowFacts {
            mass_error: true,
            ..base
        }),
        Some(4)
    );
}

fn table() -> Vec<u8> {
    let mut t = Aligned::with_capacity(1, 2);
    for (mz, rt) in [(200.1, 5.0), (300.2, 6.0)] {
        t.mz.push(mz);
        t.rt.push(rt);
        t.area.push(1e6);
        t.height.push(1e5);
        t.apex_rt.push(rt as f32);
        t.width.push(0.1);
        t.state.push(0);
    }
    write_table(&t, &Recipe::default(), &Origin::default())
}

#[test]
fn the_block_is_written_read_by_row_and_replaced() {
    let bytes = table();
    assert!(
        read_meta(&mut mzlab_feat::store::Slice(&bytes), bytes.len() as u64)
            .unwrap()
            .is_none()
    );
    let rows = vec![
        (1u32, "{\"id\":1}".to_string()),
        (2, "{\"id\":2,\"x\":[1,2,3]}".to_string()),
    ];
    let mut out = Vec::new();
    append(
        &mut mzlab_feat::store::Slice(&bytes),
        bytes.len() as u64,
        &mut out,
        "{\"version\":1}",
        &rows,
    )
    .unwrap();
    let size = out.len() as u64;
    // the table still reads as before
    assert_eq!(
        read_table(&out).unwrap().0.to_bytes(),
        read_table(&bytes).unwrap().0.to_bytes()
    );
    let (meta, ids) = read_meta(&mut mzlab_feat::store::Slice(&out), size)
        .unwrap()
        .unwrap();
    assert_eq!(
        (meta.get("version").and_then(|v| v.f64()), ids),
        (Some(1.0), vec![1, 2])
    );
    assert_eq!(
        read_row(&mut mzlab_feat::store::Slice(&out), size, 2)
            .unwrap()
            .unwrap(),
        rows[1].1
    );
    assert_eq!(
        read_row(&mut mzlab_feat::store::Slice(&out), size, 9).unwrap(),
        None
    );
    // a second write replaces the first
    let mut again = Vec::new();
    append(
        &mut mzlab_feat::store::Slice(&out),
        size,
        &mut again,
        "{\"version\":1}",
        &rows[..1],
    )
    .unwrap();
    let (_, ids) = read_meta(&mut mzlab_feat::store::Slice(&again), again.len() as u64)
        .unwrap()
        .unwrap();
    assert_eq!(ids, vec![1]);
    assert_eq!(
        read_table(&again).unwrap().0.to_bytes(),
        read_table(&bytes).unwrap().0.to_bytes()
    );
}

#[test]
fn decoys_stay_beyond_ten_ppm_of_every_rule_and_count_the_passing_matches() {
    use mzlab_feat::interp::filtri::{
        decoy_masses, registry_decoys, DECOY_K, DECOY_PPM, DECOY_STEP,
    };
    let rules = [333.1591, 349.1540, 303.1485];
    let d = decoy_masses(&rules);
    assert_eq!(d.len(), rules.len() * DECOY_K as usize);
    assert!(d
        .iter()
        .all(|x| rules.iter().all(|r| (x - r).abs() / r * 1e6 > DECOY_PPM)));
    // a rule that sits one step above another one: the decoy that lands on it is dropped
    let close = decoy_masses(&[300.0, 300.0 + DECOY_STEP]);
    assert!(close.len() < 2 * DECOY_K as usize);
    // a row at a decoy mass passes or is hidden by the filters; a row elsewhere matches nothing
    let rows = [(d[0], true), (d[1], false), (500.0, true)];
    assert_eq!(registry_decoys(&rules, rows), Some((d.len(), 1)));
    assert_eq!(registry_decoys(&[], rows), None);
}
