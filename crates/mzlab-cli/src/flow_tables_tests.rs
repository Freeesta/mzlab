//! `flow tables` on an invented run folder: a table with its `prov.json`, a table without a step, a file that is not a table; the exit codes.

use mzlab_core::json::parse;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};
use std::path::PathBuf;

fn cli(args: &[&str]) -> crate::Outcome {
    crate::execute(&args.iter().map(|x| x.to_string()).collect::<Vec<_>>())
}

fn table(polarity: i8) -> Vec<u8> {
    let mut t = Aligned::with_capacity(3, 2);
    for (mz, rt) in [(200.1, 1.5), (250.2, 2.5)] {
        t.mz.push(mz);
        t.rt.push(rt);
        t.area.push(1e6);
        t.height.push(1e5);
        t.apex_rt.push(rt as f32);
        t.width.push(0.1);
        t.state.push(CELL_DETECTED);
    }
    let mut r = Recipe::default();
    r.detect.polarity = Some(polarity);
    write_table(&t, &r, &Origin::default())
}

fn folder(tag: &str, bad: bool) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mzlab_flow_tables_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("gap.mzfeat"), table(1)).unwrap();
    std::fs::write(dir.join("old.mzfeat"), table(-1)).unwrap();
    std::fs::write(dir.join("notes.txt"), "not a table").unwrap();
    std::fs::write(
        dir.join("prov.json"),
        r#"{"schema":"prov/1","entity":{},"activity":{"gap":{"step":"features.gapfill@1","ms":12.34,"warnings":[]}}}"#,
    )
    .unwrap();
    if bad {
        std::fs::write(dir.join("bad.mzfeat"), "no").unwrap();
    }
    dir
}

#[test]
fn the_tables_of_a_folder_are_listed_with_their_step() {
    let dir = folder("ok", false);
    let o = cli(&["flow", "tables", dir.to_str().unwrap(), "--json"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = parse(&o.stdout).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("tables/1"));
    assert!(v.get("citation").is_some());
    let t = v.get("tables").unwrap().arr();
    let names: Vec<_> = t
        .iter()
        .map(|e| e.get("name").unwrap().str().unwrap())
        .collect();
    assert_eq!(names, ["gap.mzfeat", "old.mzfeat"]);
    assert_eq!(t[0].get("step").unwrap().str(), Some("features.gapfill@1"));
    assert_eq!(t[0].get("polarity").unwrap().f64(), Some(1.0));
    assert!(t[1].get("step").is_none());
    assert_eq!(t[1].get("polarity").unwrap().f64(), Some(-1.0));
    assert!(v.get("skipped").unwrap().arr().is_empty());
    let text = cli(&["flow", "tables", dir.to_str().unwrap()]);
    let s = String::from_utf8(text.stdout).unwrap();
    assert!(
        s.starts_with("gap.mzfeat  2 rows  3 files  +  features.gapfill@1  ok  12.3 ms\n"),
        "{s}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_that_cannot_be_read_is_skipped_and_the_exit_code_is_one() {
    let dir = folder("bad", true);
    let o = cli(&["flow", "tables", dir.to_str().unwrap(), "--json"]);
    assert_eq!(o.code, 1);
    assert!(o.stderr.contains("bad.mzfeat"), "{}", o.stderr);
    // a nonzero exit is `error/1` that carries the document as its `report`
    let e = parse(&o.stdout).unwrap();
    assert_eq!(e.get("error_key").unwrap().str(), Some("err.run.data"));
    let v = e.get("report").unwrap();
    assert_eq!(v.get("tables").unwrap().arr().len(), 2);
    let s = &v.get("skipped").unwrap().arr()[0];
    assert_eq!(s.get("name").unwrap().str(), Some("bad.mzfeat"));
    assert_eq!(s.get("error_key").unwrap().str(), Some("error.badMzfeat"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_exit_codes_are_pinned() {
    let empty =
        std::env::temp_dir().join(format!("mzlab_flow_tables_empty_{}", std::process::id()));
    std::fs::create_dir_all(&empty).unwrap();
    let o = cli(&["flow", "tables", empty.to_str().unwrap(), "--json"]);
    assert_eq!(o.code, 0);
    assert!(String::from_utf8_lossy(&o.stdout).contains("\"tables\":[]"));
    assert_eq!(cli(&["flow", "tables"]).code, 2);
    assert_eq!(cli(&["flow", "tables", "a", "b"]).code, 2);
    assert_eq!(cli(&["flow", "tables", "--nope", "a"]).code, 2);
    assert_eq!(cli(&["flow", "tables", "/no/such/run"]).code, 4);
    assert_eq!(cli(&["flow", "tables", "--help"]).code, 0);
    let _ = std::fs::remove_dir_all(&empty);
}
