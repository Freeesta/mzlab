//! The graph of the proofs on a table built by hand: every identity has its table of contributions, an undetermined reading is said, the decoys without
//! an MS2 stay under level 4, the candidates the proofs do not tell apart share 1/N, a proof against puts the identity last whatever its total, two
//! runs give the same bytes, and the `evid` section of the `.mzfeat` is read back.

use std::path::{Path, PathBuf};

use mzlab_core::json::{self, Value};

use mzlab_feat::evid::*;
use mzlab_feat::family::evidence::EdgeKind as FamEdge;
use mzlab_feat::hypo::{generate, HypoParams, Inputs as HypoInputs, Registry, Rules, SRC_RULE};
use mzlab_feat::kinetic::{parent_kinetics, ChainBank, FormBank, Setup, K_PARENT_MIN};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{read_evid, read_table, write_table_with_evid, Origin};
use mzlab_feat::table::*;

const TIMES: [f64; 8] = [0.0, 2.0, 5.0, 10.0, 20.0, 40.0, 60.0, 90.0];
const FILES: usize = 10;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn registry() -> Registry {
    let rules =
        Rules::parse(&std::fs::read(root().join("TP_Mine/data/regole_tp.json")).unwrap()).unwrap();
    let inp = HypoInputs {
        parent: "C9H13NO3",
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &[],
    };
    generate(&rules, &inp, &HypoParams::default()).unwrap()
}

struct Spec {
    mz: f64,
    rt: f64,
    members: Vec<(f64, f64, u8)>,
    treated: Vec<f64>,
    controls: [f64; 2],
}

fn decay() -> Vec<f64> {
    TIMES.iter().map(|t| 1e8 * (-0.05 * t).exp()).collect()
}

fn forming() -> Vec<f64> {
    TIMES
        .iter()
        .map(|t| 1e7 * (1.0 - (-0.05f64 * t).exp()))
        .collect()
}

fn table(rows: &[Spec]) -> Aligned {
    let mut t = Aligned::with_capacity(FILES, rows.len());
    t.member_start.push(0);
    for r in rows {
        t.mz.push(r.mz);
        t.rt.push(r.rt);
        for k in 0..FILES {
            let a = if k < TIMES.len() {
                r.treated[k]
            } else {
                r.controls[k - TIMES.len()]
            };
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(r.rt as f32);
            t.width.push(0.1);
            t.state
                .push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
        for m in &r.members {
            t.member_mz.push(m.0);
            t.member_rt.push(m.1 as f32);
            t.member_role.push(m.2);
        }
        t.member_start.push(t.member_mz.len() as u32);
    }
    t
}

struct World {
    reg: Registry,
    aligned: Aligned,
    libs: Vec<LibHit>,
    arcs: Vec<RowArc>,
    facts: Vec<Facts>,
}

fn rule_hyps(reg: &Registry) -> Vec<usize> {
    let mut v: Vec<usize> = (0..reg.hyps.len())
        .filter(|&i| reg.hyps[i].steps == 1 && reg.hyps[i].sources & SRC_RULE != 0)
        .collect();
    v.sort_by(|&a, &b| reg.hyps[a].mz.total_cmp(&reg.hyps[b].mz));
    v.dedup_by(|a, b| (reg.hyps[*a].mz - reg.hyps[*b].mz).abs() < 0.01);
    v
}

fn spec(
    mz: f64,
    rt: f64,
    members: Vec<(f64, f64, u8)>,
    treated: Vec<f64>,
    controls: [f64; 2],
) -> Spec {
    Spec {
        mz,
        rt,
        members,
        treated,
        controls,
    }
}

/// The seven rows of the world: the parent, a true product with an adduct, a false ISF, an isomer pair, a decoy without MS2 and an "against" one.
fn rows_of(reg: &Registry) -> Vec<Spec> {
    let h = rule_hyps(reg);
    let mz = |k: usize| reg.hyps[h[k]].mz;
    vec![
        spec(
            168.1019,
            6.0,
            vec![(168.1019, 6.0, ROLE_M), (169.1052, 6.0, ROLE_ISOTOPE)],
            decay(),
            [1e8, 0.0],
        ),
        spec(
            mz(0),
            4.0,
            vec![(mz(0), 4.0, ROLE_M), (mz(0) + 21.98194, 4.0, ROLE_ADDUCT)],
            forming(),
            [0.0, 0.0],
        ),
        spec(
            120.0,
            6.0,
            vec![(120.0, 6.0, ROLE_M), (140.0, 6.0, ROLE_UNDETERMINED)],
            forming(),
            [0.0, 0.0],
        ),
        spec(
            mz(1),
            3.0,
            vec![(mz(1), 3.0, ROLE_M)],
            forming(),
            [0.0, 0.0],
        ),
        spec(
            mz(1),
            3.5,
            vec![(mz(1), 3.5, ROLE_M)],
            forming(),
            [0.0, 0.0],
        ),
        spec(
            mz(2),
            2.0,
            vec![(mz(2), 2.0, ROLE_M)],
            forming(),
            [0.0, 0.0],
        ),
        spec(
            mz(3),
            2.5,
            vec![(mz(3), 2.5, ROLE_M)],
            forming(),
            [8e6, 9e6],
        ),
    ]
}

fn world() -> World {
    let reg = registry();
    let lib = |row, entropy, ppm| LibHit {
        row,
        name: format!("candidato {row}"),
        library: "LIB".into(),
        entropy,
        shared: 8,
        ppm,
    };
    let rows = rows_of(&reg);
    let arc = RowArc {
        row: 2,
        lo: 0,
        up: 1,
        r: 0.95,
        kind: FamEdge::Undetermined,
        readings: vec!["NH4/NH3".into(), "FA/HCOOH".into()],
        proofs: vec![],
    };
    let mut facts = vec![Facts::default(); rows.len()];
    facts[1] = Facts {
        isotope: Some(0.95),
        fragments: Some(0.85),
        purity: Some(0.9),
        network: Some(0.7),
    };
    World {
        aligned: table(&rows),
        libs: vec![
            lib(1, 0.85, 1.0),
            lib(3, 0.80, 2.0),
            lib(4, 0.78, 2.0),
            lib(6, 0.9, 1.5),
        ],
        arcs: vec![arc],
        facts,
        reg,
    }
}

fn graph(w: &World, t: &PointsTable) -> Vec<Identity> {
    let (forms, chain) = (FormBank::new(&TIMES), ChainBank::new(&TIMES, K_PARENT_MIN));
    let y = decay();
    let parent = parent_kinetics(&forms, &TIMES, &y, None);
    let setup = Setup {
        times: &TIMES,
        forms: &forms,
        chain: &chain,
        parent,
        pseudo: 1e4,
    };
    let treated: Vec<usize> = (0..TIMES.len()).collect();
    let inp = Inputs {
        table: t,
        aligned: &w.aligned,
        facts: &w.facts,
        arcs: &w.arcs,
        registry: &w.reg,
        libs: &w.libs,
        external: &[],
        kinetics: Some(Kinetics {
            setup: &setup,
            treated: &treated,
            controls: &[8, 9],
        }),
        parent: Some((168.1019, 6.0)),
    };
    build_graph(&inp)
}

fn by_row(ids: &[Identity], row: u32) -> &Identity {
    ids.iter().find(|i| i.id == row).unwrap()
}

fn has(i: &Identity, code: &str) -> bool {
    i.breakdown.reasons.iter().any(|r| r.code == code)
}

#[test]
fn every_identity_has_its_table_and_the_reasons_of_its_decisions() {
    let t = PointsTable::bundled().unwrap();
    let w = world();
    let ids = graph(&w, &t);
    assert_eq!(ids.len(), 7);
    for i in &ids {
        assert!(
            !i.breakdown.rows.is_empty(),
            "identity {} without contributions",
            i.id
        );
        assert!(i.breakdown.rows.iter().any(|e| e.kind == Kind::Frequency));
        assert!(i.breakdown.rows.windows(2).all(|p| p[0].kind <= p[1].kind));
    }
    let tp = by_row(&ids, 1);
    let kinds: Vec<Kind> = tp.breakdown.rows.iter().map(|e| e.kind).collect();
    for k in [
        Kind::Mass,
        Kind::Isotopes,
        Kind::Ms2,
        Kind::Fragments,
        Kind::Network,
        Kind::Kinetics,
        Kind::Controls,
        Kind::Rt,
        Kind::Frequency,
    ] {
        assert!(kinds.contains(&k), "the true product misses {k:?}");
    }
    assert_eq!((tp.max_level, tp.hypothesis.is_some()), ("2a", true));
    assert!(tp
        .edges
        .iter()
        .any(|e| e.kind == EdgeKind::ParentProduct && e.from == Node::Identity(0)));
    assert!(tp.edges.iter().any(|e| e.kind == EdgeKind::Library));
    // no kinetic proof for the parent itself
    assert!(by_row(&ids, 0)
        .breakdown
        .rows
        .iter()
        .all(|e| e.kind != Kind::Kinetics));
    assert!(by_row(&ids, 0)
        .edges
        .iter()
        .any(|e| e.kind == EdgeKind::Isotope));
}

#[test]
fn a_false_in_source_fragment_is_not_told_from_the_other_reading() {
    let t = PointsTable::bundled().unwrap();
    let ids = graph(&world(), &t);
    let isf = by_row(&ids, 2);
    assert!(isf
        .edges
        .iter()
        .any(|e| e.kind == EdgeKind::Undetermined
            && e.label == "NH4/NH3/FA/HCOOH".replace("/FA", "/FA")));
    assert!(has(isf, R_UNDETERMINED_EDGE));
}

#[test]
fn decoys_without_an_ms2_never_go_above_level_4() {
    let t = PointsTable::bundled().unwrap();
    let ids = graph(&world(), &t);
    for row in [0, 2, 5] {
        let i = by_row(&ids, row);
        assert_eq!(i.max_level, "4", "row {row}");
        assert!(has(i, R_NO_MS2));
    }
    assert!(!has(by_row(&ids, 1), R_NO_MS2));
}

#[test]
fn candidates_the_proofs_do_not_tell_apart_share_one_over_n() {
    let t = PointsTable::bundled().unwrap();
    let ids = graph(&world(), &t);
    for row in [3, 4] {
        let i = by_row(&ids, row);
        assert_eq!((i.breakdown.n, i.breakdown.probability), (2, 0.5));
        assert!(has(i, R_INDISTINGUISHABLE));
    }
    assert_eq!(by_row(&ids, 1).breakdown.n, 1);
}

#[test]
fn a_proof_against_puts_the_identity_last_even_with_a_high_total() {
    let t = PointsTable::bundled().unwrap();
    let ids = graph(&world(), &t);
    let blank = by_row(&ids, 6);
    assert!(blank.breakdown.against >= 1 && has(blank, R_AGAINST_OPEN));
    let ctl = blank
        .breakdown
        .rows
        .iter()
        .find(|e| e.kind == Kind::Controls)
        .unwrap();
    assert_eq!((ctl.sign, ctl.points), (Sign::Against, 0.0));
    let others_min = ids
        .iter()
        .filter(|i| i.id != 6)
        .map(|i| i.breakdown.total)
        .fold(f64::INFINITY, f64::min);
    assert!(
        blank.breakdown.total > others_min,
        "its total ({}) is not low",
        blank.breakdown.total
    );
    assert_eq!(ids.last().unwrap().id, 6, "last in the order");
    assert!(ids[..ids.len() - 1]
        .iter()
        .all(|i| i.breakdown.against == 0));
    assert!(ids
        .windows(2)
        .all(|p| p[0].breakdown.rank(&p[1].breakdown) != std::cmp::Ordering::Greater));
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .map(|x| x.str().unwrap().to_string())
        .collect()
}

#[test]
fn the_document_follows_the_schema_and_is_the_same_twice() {
    let t = PointsTable::bundled().unwrap();
    let w = world();
    let a = graph_json(&graph(&w, &t), &t);
    assert_eq!(a, graph_json(&graph(&w, &t), &t), "two runs, same bytes");
    let schema =
        json::parse(&std::fs::read(root().join("docs/schemi/evid-1.json")).unwrap()).unwrap();
    let defs = schema.get("$defs").unwrap();
    let doc = json::parse(a.as_bytes()).unwrap();
    assert_eq!(doc.get("formato").and_then(Value::str), Some("evid/1"));
    let (need, need_edge) = (
        strings(defs.get("identita").unwrap().get("required")),
        strings(defs.get("arco").unwrap().get("required")),
    );
    let (levels, edge_kinds) = (
        strings(
            defs.get("identita")
                .unwrap()
                .get("properties")
                .unwrap()
                .get("livello_max")
                .unwrap()
                .get("enum"),
        ),
        strings(
            defs.get("arco")
                .unwrap()
                .get("properties")
                .unwrap()
                .get("tipo")
                .unwrap()
                .get("enum"),
        ),
    );
    for i in doc.get("identita").unwrap().arr() {
        for k in &need {
            assert!(i.get(k).is_some(), "identity without `{k}`");
        }
        assert!(levels.contains(&i.get("livello_max").unwrap().str().unwrap().to_string()));
        for e in i.get("archi").unwrap().arr() {
            for k in &need_edge {
                assert!(e.get(k).is_some(), "arc without `{k}`");
            }
            assert!(edge_kinds.contains(&e.get("tipo").unwrap().str().unwrap().to_string()));
        }
    }
}

#[test]
fn the_evid_section_of_the_mzfeat_is_written_and_read_back() {
    let t = PointsTable::bundled().unwrap();
    let w = world();
    let json = graph_json(&graph(&w, &t), &t);
    let (r, o) = (Recipe::default(), Origin::default());
    let a = write_table_with_evid(&w.aligned, &r, &o, json.as_bytes());
    let b = write_table_with_evid(&w.aligned, &r, &o, json.as_bytes());
    assert_eq!(a, b, "same bytes");
    assert_eq!(read_evid(&a).unwrap(), json.as_bytes());
    let (back, _) = read_table(&a).unwrap();
    assert_eq!(back.to_bytes(), w.aligned.to_bytes());
    assert!(
        read_evid(&mzlab_feat::store::write_table(&w.aligned, &r, &o))
            .unwrap()
            .is_empty()
    );
}
