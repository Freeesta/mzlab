//! `features.pair@1` on two synthetic tables (invented neutral masses: two known pairs, an RT decoy at the mass of a third species, a mass decoy at the
//! right RT and one positive row alone): the pairs and the `pair/1` are the ones of `mzlab_core::polarity_pair`, every row is listed as paired or unmatched
//! and the bytes are the same on every call.

use mzlab_core::json::{parse, Value};
use mzlab_feat::pair::pair_text;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const H: f64 = 1.007276;

fn table(rows: &[(f64, f64)]) -> Vec<u8> {
    let mut t = Aligned::with_capacity(2, rows.len());
    for (mz, rt) in rows {
        t.mz.push(*mz);
        t.rt.push(*rt);
        for h in [1e5, 2e5] {
            t.area.push(h * 6.0);
            t.height.push(h as f32);
            t.apex_rt.push(*rt as f32);
            t.width.push(0.08);
            t.state.push(CELL_DETECTED);
        }
    }
    let origin = Origin {
        inputs: (0..2)
            .map(|k| (format!("c{k}.mzML"), format!("{k:02x}").repeat(32)))
            .collect(),
        ..Origin::default()
    };
    write_table(&t, &Recipe::default(), &origin)
}

/// Neutral masses A 180.06339, B 250.12, C 310.2 (alone in the positive table; in the negative table at 9.9 min and 0.1 Da off at 3.12 min).
fn tables() -> [(&'static str, Vec<u8>); 2] {
    [
        (
            "pos.mzfeat",
            table(&[(180.06339 + H, 5.0), (250.12 + H, 8.2), (310.2 + H, 3.1)]),
        ),
        (
            "neg.mzfeat",
            table(&[
                (180.06339 - H, 5.03),
                (250.12 - H, 8.18),
                (310.2 - H, 9.9),
                (310.3 - H, 3.12),
            ]),
        ),
    ]
}

fn pair(params: &[(String, Value)]) -> mzlab_core::error::Result<mzlab_feat::pair::Made> {
    let t = tables();
    pair_text(params, (t[0].0, &t[0].1), (t[1].0, &t[1].1))
}

#[test]
fn the_known_pairs_are_found_the_decoys_are_listed_as_unmatched() {
    let m = pair(&[]).unwrap();
    let v = parse(m.doc.as_bytes()).unwrap();
    assert_eq!(v.get("schema").and_then(Value::str), Some("pair/1"));
    let pairs = v.get("pairs").unwrap().arr();
    assert_eq!(pairs.len(), 2);
    assert_eq!(
        pairs[0].get("rule").and_then(Value::str),
        Some("[M+H]+ / [M-H]-")
    );
    // every row of both tables is in a pair or in the unmatched lists
    let c = v.get("counts").unwrap();
    let n = |k: &str| c.get(k).and_then(Value::f64).unwrap() as usize;
    assert_eq!(n("pairs") + n("pos_unmatched"), 3);
    assert_eq!(n("pairs") + n("neg_unmatched"), 4);
    assert_eq!(m.csv, pair(&[]).unwrap().csv);
}

#[test]
fn parameters_reach_the_engine_and_a_bad_one_is_refused() {
    let p = vec![("rt_tol".to_string(), Value::Num(0.01))];
    let v = parse(pair(&p).unwrap().doc.as_bytes()).unwrap();
    assert_eq!(v.get("pairs").unwrap().arr().len(), 0);
    for bad in [
        ("rt_tol", Value::Num(-1.0)),
        ("mass_unit", Value::Str("mmu".into())),
        ("pos_adducts", Value::Str("[M-H]-".into())),
        ("neg_adducts", Value::Str("".into())),
    ] {
        assert_eq!(
            pair(&[(bad.0.to_string(), bad.1)]).unwrap_err().key,
            "err.pair.param",
            "{}",
            bad.0
        );
    }
}

#[test]
fn no_row_and_a_broken_table_are_refused() {
    let none = table(&[]);
    assert_eq!(
        pair_text(&[], ("p", &none), ("n", &none)).unwrap_err().key,
        "err.pair.empty"
    );
    assert!(pair_text(&[], ("p", b"not a table"), ("n", &none)).is_err());
    // one empty side is not an error: every row of the other side is unmatched
    let t = tables();
    let v = parse(
        pair_text(&[], ("p", &t[0].1), ("n", &none))
            .unwrap()
            .doc
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(
        v.get("unmatched").unwrap().get("pos").unwrap().arr().len(),
        3
    );
}

/// The committed tables and texts: the shared golden of the engine, the `flowtab` `.wasm` (`tests/test_flowtab_pair_wasm.py`) and the CLI.
/// `MZLAB_UPDATE_PAIR=1 cargo test -p mzlab-feat --test pair_step` rewrites them.
#[test]
fn the_golden_text_is_the_engine_text() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/pair");
    let m = pair(&[]).unwrap();
    if std::env::var_os("MZLAB_UPDATE_PAIR").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        for (n, b) in tables() {
            std::fs::write(dir.join(n), b).unwrap();
        }
        std::fs::write(dir.join("pair.csv"), &m.csv).unwrap();
        std::fs::write(dir.join("pair.json"), &m.doc).unwrap();
    }
    let read = |n: &str| std::fs::read(dir.join(n)).unwrap();
    for (n, b) in tables() {
        assert_eq!(
            read(n),
            b,
            "the committed tables are the synthetic ones of this test"
        );
    }
    assert_eq!(
        m.csv.as_bytes(),
        read("pair.csv"),
        "MZLAB_UPDATE_PAIR=1 rewrites it"
    );
    assert_eq!(m.doc.as_bytes(), read("pair.json"));
}
