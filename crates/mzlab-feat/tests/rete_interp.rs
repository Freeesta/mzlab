//! `rows_from_interp`: the rows of the network from the interpretation of invented products of the timolol formula (a hydroxylation with an MS2 compared,
//! a dehydrogenation without, a row with no modification) and the arcs they give.

use mzlab_core::elements::{mass, parse_formula};
use mzlab_feat::hypo::Rules;
use mzlab_feat::interp::confronto::VsParent;
use mzlab_feat::interp::modifica::Candidate;
use mzlab_feat::interp::ms2link::{Isomers, Pooled};
use mzlab_feat::interp::RowInterp;
use mzlab_feat::legacy::ms2::Consensus;
use mzlab_feat::legacy::series::Parent;
use mzlab_feat::rete::{build, rows_from_interp, ReteParams};
use mzlab_feat::table::Aligned;
use mzlab_feat::tp::{TpBlock, KIN_PROBABLE, ST_PARENT, ST_TP};

const PROTON: f64 = 1.007276;

fn neutral_mass(f: &str) -> f64 {
    mass(&parse_formula(f).unwrap())
}

fn table(formulas: &[(&str, f64)]) -> Aligned {
    let mut t = Aligned::with_capacity(1, formulas.len());
    for (f, rt) in formulas {
        t.mz.push(neutral_mass(f) + PROTON);
        t.rt.push(*rt);
        t.area.push(1e6);
        t.height.push(1e5);
        t.apex_rt.push(*rt as f32);
        t.width.push(0.1);
        t.state.push(mzlab_feat::table::CELL_DETECTED);
    }
    t
}

fn interp(row: usize, neutral: &str, vs: Option<VsParent>) -> RowInterp {
    let mut f = parse_formula(neutral).unwrap();
    f.iter_mut().find(|(e, _)| e == "H").unwrap().1 += 1;
    let ion = mzlab_core::elements::fmt(&f);
    RowInterp {
        row,
        id: row as u32 + 1,
        change: vec![Candidate {
            delta: String::new(),
            neutral_formula: neutral.into(),
            ion_formula: ion,
            ppm: 0.4,
            source: "rule",
        }],
        iso: None,
        mz_obs: None,
        scans: vec![],
        files_with_ms2: 0,
        pooled: Pooled {
            consensus: Consensus::default(),
            groups: vec![],
            isomers: Isomers::Single,
        },
        vs_parent: vs,
        tree: vec![],
        site: None,
        drawing: None,
        hidden: None,
    }
}

#[test]
fn the_rows_and_the_arcs_come_from_the_interpretation() {
    let t = table(&[
        ("C13H24N4O3S", 5.0),
        ("C13H24N4O4S", 4.5),
        ("C13H22N4O3S", 5.5),
        ("C9H8N2O2", 6.0),
    ]);
    let parent = Parent {
        name: "timolol".into(),
        neutral: parse_formula("C13H24N4O3S").unwrap(),
        ion: parse_formula("C13H25N4O3S").unwrap(),
        mz: t.mz[0],
        adduct: "[M+H]+".into(),
        rt: Some(5.0),
        smiles: None,
    };
    let vs = VsParent {
        equal: vec![100.0, 116.0],
        shifted: vec![132.0],
        ..VsParent::default()
    };
    let mut none = interp(3, "C9H8N2O2", None);
    none.change.clear();
    let rows = vec![
        interp(1, "C13H24N4O4S", Some(vs)),
        interp(2, "C13H22N4O3S", None),
        none,
    ];
    let tp = TpBlock {
        status: vec![ST_PARENT, ST_TP, ST_TP, ST_TP],
        notes: vec![0; 4],
        via: vec![0; 4],
        kinship: vec![0, KIN_PROBABLE, KIN_PROBABLE, 0],
        parent_row: 0,
        ..TpBlock::default()
    };
    let tmax = [Some(0.0), Some(10.0), Some(20.0), Some(30.0)];
    let (p, v) = rows_from_interp(&t, &tp, &parent, &rows, &tmax);
    assert_eq!((p.row, p.id, p.ion_formula.as_str()), (0, 1, "C13H25N4O3S"));
    assert_eq!(v.len(), 2, "a row with no modification is not a node");
    assert!(v[0].has_ms2 && !v[1].has_ms2);
    assert_eq!(
        (v[0].equal.len(), v[0].shifted.len(), v[0].tmax),
        (2, 1, Some(10.0))
    );

    let rules = Rules::parse(include_bytes!("../../../TP_Mine/data/regole_tp.json")).unwrap();
    let r = build(&tp, &p, &v, &rules, &ReteParams::default());
    let arcs: Vec<_> = r
        .arcs
        .iter()
        .map(|a| (a.from, a.to, a.rule.as_str()))
        .collect();
    assert_eq!(
        arcs,
        vec![
            (1, 2, "hydroxylation"),
            (1, 3, "dehydrogenation"),
            (2, 3, "dehydration")
        ]
    );
    assert_eq!(
        r.arcs[0].reason,
        "shares 3 fragments, 1 shifted by +15.995; the source peaks first"
    );
    assert!(r.arcs[1].ms2.is_none());
}
