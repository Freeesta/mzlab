//! `features.union@1` on two synthetic tables (invented masses; the second series is 0.04 min later and holds one row of its own): the series CSV is the one
//! of `export::csv_full`, the list and the `union/1` are the ones of `mzlab_core::series_union`, no row is hidden and the bytes are the same on every call.

use mzlab_core::json::{parse, Value};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};
use mzlab_feat::union::{series_csv, union_text};

fn table(rows: &[(f64, f64)], shift: f64) -> Vec<u8> {
    let mut t = Aligned::with_capacity(2, rows.len());
    for (mz, rt) in rows {
        t.mz.push(*mz);
        t.rt.push(rt + shift);
        for h in [1e5, 2e5] {
            t.area.push(h * 6.0);
            t.height.push(h as f32);
            t.apex_rt.push((rt + shift) as f32);
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

fn series() -> Vec<(String, Vec<u8>)> {
    vec![
        (
            "a.mzfeat".into(),
            table(&[(301.0, 3.0), (402.2, 4.0), (512.1, 6.0)], 0.0),
        ),
        (
            "b.mzfeat".into(),
            table(&[(301.1, 3.0), (402.2, 4.0), (230.0, 2.0)], 0.04),
        ),
    ]
}

#[test]
fn the_series_csv_is_the_table_with_its_stable_ids() {
    let csv = series_csv(&series()[0].1).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "id,mz,rt,rt_range:min,rt_range:max");
    assert_eq!(lines.len(), 4);
    // the rows are in the order of the table; ids are 1-based positions, m/z 5 and RT 4 decimals
    assert!(lines[1].starts_with("1,301,3,"), "{csv}");
}

#[test]
fn two_series_make_one_list_with_every_row_listed() {
    let m = union_text(&[], &series()).unwrap();
    let v = parse(m.doc.as_bytes()).unwrap();
    assert_eq!(v.get("schema").and_then(Value::str), Some("union/1"));
    // 301/301.1 and the two 402.2 are merged; 512.1 and 230 are alone: four targets from six rows
    assert_eq!(v.get("targets").unwrap().arr().len(), 4);
    assert_eq!(m.csv.lines().count(), 5);
    assert_eq!(m.csv, union_text(&[], &series()).unwrap().csv);
}

#[test]
fn parameters_reach_the_engine_and_a_bad_one_is_refused() {
    let p = vec![("min_series".to_string(), Value::Num(2.0))];
    let m = union_text(&p, &series()).unwrap();
    assert_eq!(m.csv.lines().count(), 3);
    let v = parse(m.doc.as_bytes()).unwrap();
    assert_eq!(v.get("dropped").unwrap().arr().len(), 2);
    let bad = vec![("da".to_string(), Value::Num(9.0))];
    assert_eq!(
        union_text(&bad, &series()).unwrap_err().key,
        "err.union.param"
    );
}

#[test]
fn no_table_and_a_broken_one_are_refused() {
    assert_eq!(union_text(&[], &[]).unwrap_err().key, "err.union.empty");
    assert!(union_text(&[], &[("x.mzfeat".into(), b"not a table".to_vec())]).is_err());
}

/// The committed tables and texts: the shared golden of the engine, the `flowtab` `.wasm` (`tests/test_flowtab_union_wasm.py`) and the CLI.
/// `MZLAB_UPDATE_UNION=1 cargo test -p mzlab-feat --test union_step` rewrites them.
#[test]
fn the_golden_text_is_the_engine_text() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/union");
    let m = union_text(&[], &series()).unwrap();
    if std::env::var_os("MZLAB_UPDATE_UNION").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        for (n, b) in series() {
            std::fs::write(dir.join(n), b).unwrap();
        }
        std::fs::write(dir.join("union.csv"), &m.csv).unwrap();
        std::fs::write(dir.join("union.json"), &m.doc).unwrap();
    }
    let read = |n: &str| std::fs::read(dir.join(n)).unwrap();
    let from_disk: Vec<(String, Vec<u8>)> =
        series().iter().map(|(n, _)| (n.clone(), read(n))).collect();
    assert_eq!(
        from_disk,
        series(),
        "the committed tables are the synthetic ones of this test"
    );
    let got = union_text(&[], &from_disk).unwrap();
    assert_eq!(
        got.csv.as_bytes(),
        read("union.csv"),
        "MZLAB_UPDATE_UNION=1 rewrites it"
    );
    assert_eq!(got.doc.as_bytes(), read("union.json"));
}
