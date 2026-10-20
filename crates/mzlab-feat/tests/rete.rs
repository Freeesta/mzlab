//! The network of the products (`rete/1`) on a synthetic series of invented products of the timolol formula: the arcs and their directions, the steps and
//! positions, the filters, the reason in words, byte-identical output for any order of the rows, and the keys of `docs/schemi/rete-1.json`.

use mzlab_core::elements::{mass, parse_formula};
use mzlab_core::json;
use mzlab_feat::hypo::Rules;
use mzlab_feat::rete::{build, Rete, ReteParams, ReteRow};
use mzlab_feat::tp::{TpBlock, KIN_POSSIBLE, KIN_PROBABLE, ST_PARENT, ST_TP, ST_UNEXPLAINED};

const PROTON: f64 = 1.007276;

fn rules() -> Rules {
    Rules::parse(include_bytes!("../../../TP_Mine/data/regole_tp.json")).unwrap()
}

fn neutral_mass(f: &str) -> f64 {
    mass(&parse_formula(f).unwrap())
}

fn row(i: usize, neutral: &str, ion: &str, rt: f64, tmax: f64) -> ReteRow {
    ReteRow {
        row: i,
        id: i as u32 + 1,
        mz: neutral_mass(neutral) + PROTON,
        rt,
        tmax: Some(tmax),
        neutral_formula: neutral.into(),
        ion_formula: ion.into(),
        ..ReteRow::default()
    }
}

/// Timolol, its hydroxylation (+O), dihydroxylation (+O2), dehydrogenation (−H2), a C3H6 loss, and an unrelated formed row.
fn rows() -> Vec<ReteRow> {
    vec![
        row(0, "C13H24N4O3S", "C13H25N4O3S", 5.0, 0.0),
        row(1, "C13H24N4O4S", "C13H25N4O4S", 4.5, 10.0),
        row(2, "C13H24N4O5S", "C13H25N4O5S", 4.0, 40.0),
        row(3, "C13H22N4O3S", "C13H23N4O3S", 5.5, 20.0),
        row(4, "C10H18N4O3S", "C10H19N4O3S", 3.0, 30.0),
        row(5, "C9H8N2O2", "C9H9N2O2", 6.0, 30.0),
    ]
}

fn tp() -> TpBlock {
    TpBlock {
        status: vec![ST_PARENT, ST_TP, ST_TP, ST_TP, ST_TP, ST_UNEXPLAINED],
        notes: vec![0; 6],
        via: vec![0; 6],
        kinship: vec![0, KIN_PROBABLE, KIN_POSSIBLE, KIN_PROBABLE, KIN_POSSIBLE, 0],
        parent_row: 0,
        ..TpBlock::default()
    }
}

fn net(rows: &[ReteRow], p: &ReteParams) -> Rete {
    let parent = rows.iter().find(|r| r.row == 0).unwrap().clone();
    build(&tp(), &parent, rows, &rules(), p)
}

fn arcs_of(r: &Rete) -> Vec<(u32, u32, &str)> {
    r.arcs
        .iter()
        .map(|a| (a.from, a.to, a.rule.as_str()))
        .collect()
}

#[test]
fn the_arcs_follow_the_rules_and_the_time_course() {
    let r = net(&rows(), &ReteParams::default());
    assert_eq!(
        arcs_of(&r),
        vec![
            (1, 2, "hydroxylation"),
            (1, 3, "dihydroxylation"),
            (1, 4, "dehydrogenation"),
            (1, 5, "loss_of_propene_n_deisopropylation"),
            (2, 3, "hydroxylation"),
            (2, 4, "dehydration"),
        ]
    );
    // the unexplained row and the unrelated formula are not nodes; the arc of a class rule says so
    assert!(r.nodes.iter().all(|n| n.row.id != 6));
    assert_eq!(r.arcs[3].scope, "class");
    assert_eq!(r.arcs[2].delta, "\u{2212}2H");
    assert!(r
        .arcs
        .iter()
        .all(|a| a.kinetic == "consistent" && a.ppm.abs() < 1e-3));
}

#[test]
fn the_columns_are_the_steps_and_the_places_go_by_mz() {
    let r = net(&rows(), &ReteParams::default());
    let at = |id: u32| {
        r.nodes
            .iter()
            .find(|n| n.row.id == id)
            .map(|n| (n.step, n.pos))
            .unwrap()
    };
    assert_eq!(
        (at(1), at(5), at(4), at(2), at(3)),
        ((0, 0), (1, 0), (1, 1), (1, 2), (1, 3))
    );
    assert_eq!(r.columns(), 2);
    assert_eq!(r.parent, 1);
}

#[test]
fn a_group_the_parent_does_not_reach_starts_at_step_one_from_its_earliest_node() {
    let mut v = rows();
    v.push(row(6, "C9H8N2O3", "C9H9N2O3", 5.8, 50.0));
    v.push(row(7, "C9H8N2O4", "C9H9N2O4", 5.6, 60.0));
    let mut t = tp();
    t.status = vec![ST_PARENT, ST_TP, ST_TP, ST_TP, ST_TP, ST_TP, ST_TP, ST_TP];
    t.kinship = vec![0; 8];
    let parent = v[0].clone();
    let r = build(&t, &parent, &v, &rules(), &ReteParams::default());
    let step = |id: u32| {
        r.nodes
            .iter()
            .find(|n| n.row.id == id)
            .map(|n| n.step)
            .unwrap()
    };
    // 6 -> 7 by +O and 6 -> 8 by +2O (tmax 30, 50, 60): 6 is the earliest of the group
    assert_eq!((step(6), step(7), step(8)), (1, 2, 2));
}

#[test]
fn the_ppm_and_the_fragments_filter_the_arcs() {
    let mut v = rows();
    v[1].mz += 0.01; // 30 ppm off the +O of the parent
    let r = net(&v, &ReteParams::default());
    assert!(!arcs_of(&r).contains(&(1, 2, "hydroxylation")));
    let loose = net(
        &v,
        &ReteParams {
            ppm: 50.0,
            ..ReteParams::default()
        },
    );
    assert!(arcs_of(&loose).contains(&(1, 2, "hydroxylation")));
    // an arc from the parent with an MS2 compared needs enough fragments; one without MS2 is kept
    let mut w = rows();
    w[1].has_ms2 = true;
    w[1].equal = vec![100.0];
    let strict = net(
        &w,
        &ReteParams {
            min_fragments: 3,
            ..ReteParams::default()
        },
    );
    assert!(!arcs_of(&strict).contains(&(1, 2, "hydroxylation")));
    assert!(arcs_of(&strict).contains(&(1, 3, "dihydroxylation")));
}

#[test]
fn the_reason_says_what_the_ms2_shares() {
    let mut v = rows();
    v[1].has_ms2 = true;
    v[1].equal = vec![100.0, 110.0, 120.0];
    v[1].shifted = vec![136.0, 146.0];
    let r = net(&v, &ReteParams::default());
    let a = &r.arcs[0];
    assert_eq!(
        a.ms2.as_ref().map(|m| (m.equal.len(), m.shifted.len())),
        Some((3, 2))
    );
    assert_eq!(
        a.reason,
        "shares 5 fragments, 2 shifted by +15.995; the source peaks first"
    );
    assert!(r.arcs[1]
        .reason
        .starts_with("mass difference +2O of rule dihydroxylation"));
}

#[test]
fn the_output_is_the_same_bytes_for_every_order_of_the_rows() {
    let base = rows();
    let want = net(&base, &ReteParams::default()).to_json();
    let mut idx: Vec<usize> = (0..base.len()).collect();
    let mut seen = 0;
    permute(&mut idx, 0, &mut |p| {
        let v: Vec<ReteRow> = p.iter().map(|&i| base[i].clone()).collect();
        assert_eq!(
            net(&v, &ReteParams::default()).to_json(),
            want,
            "order {p:?}"
        );
        seen += 1;
    });
    assert_eq!(seen, 720);
}

fn permute(a: &mut Vec<usize>, k: usize, f: &mut dyn FnMut(&[usize])) {
    if k == a.len() {
        f(a);
        return;
    }
    for i in k..a.len() {
        a.swap(k, i);
        permute(a, k + 1, f);
        a.swap(k, i);
    }
}

#[test]
fn the_json_has_the_keys_of_the_schema() {
    let schema = json::parse(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/schemi/rete-1.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let keys = |v: Option<&json::Value>| -> Vec<String> {
        v.map(|v| {
            v.arr()
                .iter()
                .filter_map(|s| s.str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
    };
    let doc = json::parse(net(&rows(), &ReteParams::default()).to_json().as_bytes()).unwrap();
    assert_eq!(doc.get("schema").and_then(|s| s.str()), Some("rete/1"));
    for k in keys(schema.get("required")) {
        assert!(doc.get(&k).is_some(), "document without `{k}`");
    }
    let defs = schema.get("$defs").unwrap();
    for (list, def) in [("nodes", "nodo"), ("arcs", "arco")] {
        let required = keys(defs.get(def).and_then(|d| d.get("required")));
        assert!(!required.is_empty());
        for item in doc.get(list).unwrap().arr() {
            for k in &required {
                assert!(item.get(k).is_some(), "{list} item without `{k}`");
            }
        }
    }
}

#[test]
fn no_arc_leaves_the_parent_alone() {
    let r = net(&rows()[..1], &ReteParams::default());
    assert_eq!((r.nodes.len(), r.arcs.len()), (1, 0));
    assert!(r.to_json().ends_with("\"n_arcs\":0}"));
}
