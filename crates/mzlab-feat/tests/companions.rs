//! `features.companions@1` on a synthetic unit-mass table (invented masses): a compound M with its M+1 at 11 % (10 carbons), an in-source loss of 18 Da and
//! an NH4 adduct; an unrelated ion at M2+1 with another time profile; a co-eluting ion with no mass relation. Edges need the correlation, never the mass
//! alone; no row is removed; the text is the same bytes on every call.

use mzlab_core::json::{parse, Value};
use mzlab_feat::companions::{companions_text, WARN_CORR_FILES};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const LEVELS: [[f64; 5]; 2] = [[1.0, 2.0, 3.0, 4.0, 5.0], [5.0, 1.0, 4.0, 2.0, 3.0]];

/// (m/z, RT, level set, share): rows 0-3 are M, M+1, M-18 and M+17 of compound 0; 4 is M2 and 5 its "M2+1" with the profile of compound 1; 6 has the profile of M and no mass relation.
const ROWS: [(f64, f64, usize, f64); 7] = [
    (301.0, 3.00, 0, 1.0),
    (302.0, 3.00, 0, 0.11),
    (283.0, 3.01, 0, 0.30),
    (318.0, 3.00, 0, 0.20),
    (401.0, 4.00, 0, 1.0),
    (402.0, 4.00, 1, 0.11),
    (350.0, 3.02, 0, 0.5),
];

fn table(files: usize) -> Vec<u8> {
    let mut t = Aligned::with_capacity(files, ROWS.len());
    for (mz, rt, c, share) in ROWS {
        t.mz.push(mz);
        t.rt.push(rt);
        for f in 0..files {
            let h = 1e5 * LEVELS[c][f % 5] * share * (1.0 + 0.001 * (f / 5) as f64);
            t.area.push(h * 6.0);
            t.height.push(h as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.08);
            t.state.push(CELL_DETECTED);
        }
    }
    let origin = Origin {
        inputs: (0..files)
            .map(|k| (format!("c{k}.mzML"), format!("{k:02x}").repeat(32)))
            .collect(),
        ..Origin::default()
    };
    write_table(&t, &Recipe::default(), &origin)
}

fn run(files: usize, params: &[(String, Value)]) -> Value {
    let m = companions_text(params, &table(files)).unwrap();
    parse(m.text.as_bytes()).unwrap()
}

fn edges_of<'a>(v: &'a Value, key: &str) -> Vec<&'a Value> {
    v.get(key).unwrap().arr().iter().collect()
}

fn id(e: &Value, k: &str) -> i64 {
    e.get(k).unwrap().f64().unwrap() as i64
}

#[test]
fn isotope_loss_and_adduct_edges_with_their_evidence() {
    let v = run(5, &[]);
    let edges = edges_of(&v, "edges");
    let of = |kind: &str| -> Vec<&&Value> {
        edges
            .iter()
            .filter(|e| e.get("kind").and_then(Value::str) == Some(kind))
            .collect()
    };
    let iso = of("isotope");
    assert_eq!(iso.len(), 1, "{edges:?}");
    let e = iso[0];
    assert_eq!((id(e, "from"), id(e, "to")), (1, 2));
    // ids are the 1-based rows: M = 1, M+1 = 2
    let c: Vec<f64> = e
        .get("carbons")
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().unwrap())
        .collect();
    assert!(c[0] <= 10.0 && 10.0 <= c[1], "{c:?}");
    assert!(e.get("r").unwrap().f64().unwrap() > 0.999);
    assert_eq!(e.get("n_files").unwrap().f64(), Some(5.0));
    assert_eq!(id(e, "parent"), 1);
    // the loss of 18 (rows 301 and 283) and the NH4 adduct (301, 318)
    let rel = of("relation");
    let names: Vec<(i64, i64, Vec<&str>)> = rel
        .iter()
        .map(|e| {
            let r = e
                .get("relations")
                .unwrap()
                .arr()
                .iter()
                .filter_map(Value::str)
                .collect();
            (id(e, "from"), id(e, "to"), r)
        })
        .collect();
    assert!(names.contains(&(3, 1, vec!["loss_h2o"])), "{names:?}");
    assert!(
        names.contains(&(1, 4, vec!["adduct_nh4", "loss_nh3"])),
        "{names:?}"
    );
    let loss = rel.iter().find(|e| id(e, "from") == 3).unwrap();
    assert_eq!(
        (id(loss, "parent"), loss.get("rule").and_then(Value::str)),
        (1, Some("heavier"))
    );
    let nh = rel.iter().find(|e| id(e, "to") == 4).unwrap();
    assert_eq!(nh.get("rule").and_then(Value::str), Some("intensity"));
    assert_eq!(id(nh, "parent"), 1);
    // no row removed; M is the parent of its three companions
    assert_eq!(v.get("rows").unwrap().f64(), Some(7.0));
    let co: Vec<Option<f64>> = v
        .get("companion_of")
        .unwrap()
        .arr()
        .iter()
        .map(Value::f64)
        .collect();
    assert_eq!(
        co,
        vec![None, Some(1.0), Some(1.0), Some(1.0), None, None, None]
    );
}

#[test]
fn an_unrelated_ion_at_plus_one_gets_no_edge() {
    let v = run(5, &[]);
    let touches = |e: &&Value| {
        id(e, "from") == 6 || id(e, "to") == 6 || id(e, "from") == 5 || id(e, "to") == 5
    };
    assert!(!edges_of(&v, "edges").iter().any(touches), "{v:?}");
    // the near miss is listed with its numbers (or it is far below the weak level), never an edge
    let weak = edges_of(&v, "weak");
    assert!(weak
        .iter()
        .all(|e| e.get("r").unwrap().f64().is_none_or(|r| r < 0.9)));
}

#[test]
fn a_correlated_ion_without_a_mass_relation_is_one_coelution_edge() {
    let v = run(5, &[]);
    // row 7 (350) has the profile of M and no relation: one edge to the group of M, the strongest; the pairs inside the group are implied
    let co: Vec<&Value> = edges_of(&v, "edges")
        .into_iter()
        .filter(|e| e.get("kind").and_then(Value::str) == Some("coelution"))
        .collect();
    assert_eq!(co.len(), 1, "{co:?}");
    assert_eq!(id(co[0], "to"), 7);
    assert!(co[0].get("parent").unwrap().is_null());
    let c = v.get("counts").unwrap();
    assert_eq!(c.get("coelution").unwrap().f64(), Some(1.0));
    assert!(c.get("implied").unwrap().f64().unwrap() >= 3.0, "{c:?}");
}

#[test]
fn few_files_write_no_edge_and_warn() {
    let m = companions_text(&[], &table(2)).unwrap();
    assert_eq!(m.warnings, vec![WARN_CORR_FILES]);
    let v = parse(m.text.as_bytes()).unwrap();
    assert!(edges_of(&v, "edges").is_empty());
    assert!(
        !edges_of(&v, "weak").is_empty(),
        "the untested candidates are listed"
    );
    assert!(
        v.get("counts")
            .unwrap()
            .get("untested")
            .unwrap()
            .f64()
            .unwrap()
            >= 3.0
    );
}

#[test]
fn parameters_reach_the_engine_and_a_bad_list_is_refused() {
    let tight = vec![("losses".to_string(), Value::Str(String::new()))];
    let v = run(5, &tight);
    assert!(!edges_of(&v, "edges").iter().any(|e| id(e, "from") == 3
        && id(e, "to") == 1
        && e.get("kind").and_then(Value::str) == Some("relation")));
    let bad = vec![("adducts".to_string(), Value::Str("17,x".into()))];
    let e = companions_text(&bad, &table(5)).unwrap_err();
    assert_eq!(e.key, "err.flow.companions.list");
    let a = companions_text(&[], &table(5)).unwrap().text;
    assert_eq!(a, companions_text(&[], &table(5)).unwrap().text);
}

/// The committed table and text: the shared golden of the engine, the `flowtab` `.wasm` (`tests/test_flowtab_companions_wasm.py`) and the CLI.
/// `MZLAB_UPDATE_COMPANIONS=1 cargo test -p mzlab-feat --test companions` rewrites both.
#[test]
fn the_golden_text_is_the_engine_text() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/companions");
    let text = companions_text(&[], &table(5)).unwrap().text;
    if std::env::var_os("MZLAB_UPDATE_COMPANIONS").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("table.mzfeat"), table(5)).unwrap();
        std::fs::write(dir.join("companions.json"), &text).unwrap();
    }
    let bytes = std::fs::read(dir.join("table.mzfeat")).unwrap();
    let golden = std::fs::read_to_string(dir.join("companions.json")).unwrap();
    assert_eq!(
        companions_text(&[], &bytes).unwrap().text,
        golden,
        "MZLAB_UPDATE_COMPANIONS=1 rewrites it"
    );
    assert_eq!(
        bytes,
        table(5),
        "the committed table is the synthetic one of this test"
    );
}
