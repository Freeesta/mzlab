//! The network of a `.mzfeat` with the blocks `tp/1` and `interp/1` (`rete::network_text`), written by the real store on invented rows of the timolol
//! formula: the arcs, the time course from the roles of the files, the table and the interpretation in two files or in one, and the errors of a file
//! without a block.

use mzlab_core::campioni::{proponi, to_json};
use mzlab_core::json::parse;
use mzlab_feat::interp::store::append;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::rete::{network_text, ReteParams};
use mzlab_feat::store::{write_table, Origin, Slice};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::tp::{TpBlock, KIN_PROBABLE, ST_PARENT, ST_TP};

const NAMES: [&str; 7] = [
    "X_blank",
    "X_TiO2_t000min",
    "X_TiO2_t005min",
    "X_TiO2_t010min",
    "X_TiO2_t020min",
    "X_TiO2_t040min",
    "X_TiO2_t080min",
];
const PROTON: f64 = 1.007276;
const O: f64 = 15.994915;

/// The parent and two products that rise one after the other (+O, +2O).
#[rustfmt::skip]
const ROWS: [(f64, f64, [f64; 7]); 3] = [
    (317.0 + PROTON - 1.0, 5.0, [0.0, 1e7, 9e6, 7e6, 5e6, 3e6, 1e6]),
    (317.0 + PROTON - 1.0 + O, 4.5, [0.0, 0.0, 2e5, 5e5, 8e5, 9e5, 9e5]),
    (317.0 + PROTON - 1.0 + 2.0 * O, 4.0, [0.0, 0.0, 0.0, 1e5, 3e5, 6e5, 1e6]),
];

fn rules() -> Vec<u8> {
    include_bytes!("../../../TP_Mine/data/regole_tp.json").to_vec()
}

fn campioni() -> String {
    let files: Vec<_> = NAMES
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    to_json(&proponi(&files))
}

/// A `.mzfeat` with the block `tp` (the parent and two accepted products).
fn table(with_tp: bool) -> Vec<u8> {
    let mut t = Aligned::with_capacity(NAMES.len(), ROWS.len());
    for (mz, rt, a) in ROWS {
        t.mz.push(mz);
        t.rt.push(rt);
        for v in a {
            t.area.push(v);
            t.height.push((v / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state
                .push(if v > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    if with_tp {
        t.tp = TpBlock {
            status: vec![ST_PARENT, ST_TP, ST_TP],
            notes: vec![0; 3],
            via: vec![0; 3],
            kinship: vec![0, KIN_PROBABLE, KIN_PROBABLE],
            parent_row: 0,
            ..TpBlock::default()
        };
    }
    write_table(&t, &Recipe::default(), &Origin::default())
}

/// The same file with the block `interp`: the second row has an MS2 compared with the parent's.
fn with_interp(bytes: &[u8]) -> Vec<u8> {
    let row = |id: u32, f: &str, ion: &str, vs: &str| {
        let s = format!(
            "{{\"id\":{id},\"change\":{{\"delta\":\"x\",\"neutral_formula\":\"{f}\",\"ion_formula\":\"{ion}\",\"ppm\":0.3,\"source\":\"rule\",\"alternatives\":[]}},\"vs_parent\":{vs}}}"
        );
        (id, s)
    };
    let rows = vec![
        row(
            2,
            "C13H24N4O4S",
            "C13H25N4O4S",
            "{\"ref\":\"dda\",\"equal\":[100.1,116.2],\"shifted\":[132.2],\"losses\":[]}",
        ),
        row(3, "C13H24N4O5S", "C13H25N4O5S", "null"),
    ];
    let meta = "{\"version\":1,\"parent\":{\"ion_formula\":\"C13H25N4O3S\",\"mz\":317.1642}}";
    let mut out = Vec::new();
    append(&mut Slice(bytes), bytes.len() as u64, &mut out, meta, &rows).unwrap();
    out
}

fn run(file: &[u8], campioni: &str) -> mzlab_core::error::Result<String> {
    let n = file.len() as u64;
    network_text(
        (&mut Slice(file), n),
        (&mut Slice(file), n),
        campioni,
        &rules(),
        &ReteParams::default(),
    )
}

#[test]
fn the_arcs_come_from_the_interpretation_and_the_time_course_from_the_roles() {
    let file = with_interp(&table(true));
    let v = parse(run(&file, &campioni()).unwrap().as_bytes()).unwrap();
    assert_eq!(v.get("schema").and_then(|s| s.str()), Some("rete/1"));
    let arcs: Vec<(u32, u32, &str, &str)> = v
        .get("arcs")
        .unwrap()
        .arr()
        .iter()
        .map(|a| {
            let n = |k: &str| a.get(k).and_then(|x| x.f64()).unwrap() as u32;
            let s = |k: &str| a.get(k).and_then(|x| x.str()).unwrap();
            (n("from"), n("to"), s("rule"), s("kinetic"))
        })
        .collect();
    // 1 -> 2 (+O), 1 -> 3 (+2O) and 2 -> 3 (+O), the source peaking first (5 min, 40 min, 80 min)
    assert_eq!(
        arcs,
        vec![
            (1, 2, "hydroxylation", "consistent"),
            (1, 3, "dihydroxylation", "consistent"),
            (2, 3, "hydroxylation", "consistent")
        ]
    );
    let first = &v.get("arcs").unwrap().arr()[0];
    assert_eq!(
        first.get("reason").and_then(|r| r.str()),
        Some("shares 3 fragments, 1 shifted by +15.995; the source peaks first")
    );
    assert_eq!(v.get("parent").and_then(|p| p.f64()), Some(1.0));
}

#[test]
fn the_table_and_the_interpretation_can_be_two_files() {
    let (plain, full) = (table(true), with_interp(&table(true)));
    let (n, m) = (plain.len() as u64, full.len() as u64);
    let two = network_text(
        (&mut Slice(&plain), n),
        (&mut Slice(&full), m),
        &campioni(),
        &rules(),
        &ReteParams::default(),
    )
    .unwrap();
    assert_eq!(two, run(&full, &campioni()).unwrap());
}

#[test]
fn without_the_roles_there_is_no_time_course() {
    let file = with_interp(&table(true));
    let v = parse(run(&file, "").unwrap().as_bytes()).unwrap();
    let kinetic: Vec<_> = v
        .get("arcs")
        .unwrap()
        .arr()
        .iter()
        .map(|a| a.get("kinetic").and_then(|k| k.str()).unwrap().to_string())
        .collect();
    assert_eq!(kinetic.len(), 3);
    assert_eq!(kinetic[0], "n/a");
}

#[test]
fn a_file_without_a_block_is_an_error() {
    let no_interp = table(true);
    assert_eq!(
        run(&no_interp, &campioni()).unwrap_err().key,
        "error.rete.noInterp"
    );
    let no_tp = with_interp(&table(false));
    assert_eq!(run(&no_tp, &campioni()).unwrap_err().key, "error.tpNoBlock");
}
