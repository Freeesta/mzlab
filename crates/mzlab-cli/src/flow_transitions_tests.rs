//! `chrom.transitions@1` through `flow run` on the SIM `.wiff` of `tools/wiff_sim.py` written as mzML (skipped without python3): the CSV is the bytes
//! of `transitions_text` on the same files, the two isomers of 237.1 keep their own RT, the negative ion needs a negative row, and the example preset
//! gives candidates for its rows without an RT.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::json::Out;
use mzlab_core::json::Value;
use mzlab_core::lettori::{self, FileSet, Limiti, Membro};

const TABLE: &str = "name;q1;q3;polarity;rt;rt_window;isomer_group\n\
isoA;237.1;;+;2.5;0.3;237\n\
isoB;237.1;;+;4.0;;237\n\
ionN;235.1;;-;3.0;;\n\
wrongPol;235.1;;+;3.0;;\n";

/// `s1.mzML` and `s2.mzML` (the two samples of the SIM `.wiff`) in a folder of this test, or None without python3.
fn fixture(tag: &str) -> Option<PathBuf> {
    let d = std::env::temp_dir().join(format!("mzlab_cli_tr_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(d.join("work")).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_sim.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&d)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP transitions: python3 not available");
        return None;
    }
    let member = |file: &str| {
        let bytes = std::fs::read(d.join(file)).unwrap();
        Membro {
            nome: file.into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    let set = FileSet {
        membri: vec![member("sim.wiff"), member("sim.wiff.scan")],
    };
    let mut l = lettori::apri(set, &Limiti::sito()).unwrap();
    for c in 0..2 {
        let bytes =
            mzlab_core::lettori_mzml::scrivi(&mut *l, c, Vec::new(), &mut |_, _| {}).unwrap();
        std::fs::write(d.join(format!("s{}.mzML", c + 1)), bytes).unwrap();
    }
    Some(d)
}

/// The CSV of a plan with one `chrom.transitions` instance; `extra` are more keys of the instance (`"params":{…}` and the like).
fn run(d: &Path, extra: &str) -> String {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"s1.mzML","type":"mzml"}},{{"name":"s2.mzML","type":"mzml"}}],"steps":[
        {{"id":"tr","step":"chrom.transitions@1","in":{{"source":["file:s1.mzML","file:s2.mzML"]}}{extra}}}]}}"#
    );
    let (r, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None).unwrap();
    assert!(
        r.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(d.join("work/tr.csv")).unwrap()
}

fn params(output: &str, table: &str) -> String {
    let mut o = Out::default();
    o.raw(",\"params\":{\"output\":")
        .str(output)
        .raw(",\"transitions\":")
        .str(table)
        .raw("}");
    o.0
}

fn cell<'a>(text: &'a str, sample: &str, name: &str, col: &str) -> &'a str {
    let head: Vec<&str> = text.lines().next().unwrap().split(',').collect();
    let k = head.iter().position(|h| *h == col).unwrap();
    let line = text
        .lines()
        .find(|l| l.starts_with(&format!("{sample},{name},")))
        .unwrap_or_else(|| panic!("{sample} {name} in {text}"));
    line.split(',').nth(k).unwrap()
}

#[test]
fn the_cli_writes_the_bytes_of_the_shared_text_and_each_isomer_keeps_its_rt() {
    let Some(d) = fixture("peaks") else {
        return;
    };
    let csv = run(&d, &params("peaks", TABLE));
    let files: Vec<(String, Vec<u8>)> = ["s1.mzML", "s2.mzML"]
        .iter()
        .map(|n| (n.to_string(), std::fs::read(d.join(n)).unwrap()))
        .collect();
    let named: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let p = vec![("output".to_string(), Value::Str("peaks".into()))];
    assert_eq!(
        csv,
        mzlab_feat::transitions::transitions_text(&p, TABLE, &named).unwrap()
    );
    for (sample, name, apex) in [
        ("s1.mzML", "isoA", 2.5),
        ("s1.mzML", "isoB", 4.0),
        ("s2.mzML", "isoA", 2.55),
        ("s2.mzML", "isoB", 4.05),
    ] {
        assert_eq!(cell(&csv, sample, name, "peak"), "ok", "{csv}");
        let rt: f64 = cell(&csv, sample, name, "apex_rt").parse().unwrap();
        assert!((rt - apex).abs() <= 0.021, "{sample} {name} {rt}");
    }
    assert_eq!(cell(&csv, "s1.mzML", "ionN", "peak"), "ok");
    assert_eq!(cell(&csv, "s1.mzML", "wrongPol", "peak"), "no_trace");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_example_preset_lists_candidates_for_its_rows_without_rt() {
    let Some(d) = fixture("preset") else {
        return;
    };
    let csv = run(
        &d,
        r#","preset":"example_1","params":{"output":"candidates"}"#,
    );
    // the parent row (237.1) sees the two peaks of its channel, each in both files, the larger first
    let parent: Vec<&str> = csv
        .lines()
        .filter(|l| l.starts_with("carbamazepine,"))
        .collect();
    assert_eq!(parent.len(), 2, "{csv}");
    assert!(
        parent.iter().all(|l| l.split(',').nth(11) == Some("2")),
        "{csv}"
    );
    // the four rows of m/z 253 share the one peak of their channel and none is told which isomer it is
    assert_eq!(
        csv.lines().filter(|l| l.contains(",253,253.0972,")).count(),
        4,
        "{csv}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_browser_entry_on_packed_files_gives_the_same_bytes() {
    let Some(d) = fixture("packed") else {
        return;
    };
    let csv = run(&d, &params("table", TABLE));
    let (a, b) = (
        std::fs::read(d.join("s1.mzML")).unwrap(),
        std::fs::read(d.join("s2.mzML")).unwrap(),
    );
    let manifest = format!(
        r#"[{{"name":"s1.mzML","len":{}}},{{"name":"s2.mzML","len":{}}}]"#,
        a.len(),
        b.len()
    );
    let blob = [a, b].concat();
    let packed = mzlab_feat::transitions::transitions_packed(
        r#"{"output":"table"}"#,
        TABLE,
        &manifest,
        &blob,
    )
    .unwrap();
    assert_eq!(packed, csv);
    let e = mzlab_feat::transitions::transitions_packed(
        "{}",
        TABLE,
        r#"[{"name":"x","len":9e12}]"#,
        &blob,
    )
    .unwrap_err();
    assert_eq!(e.key, "err.transitions.manifest");
    let _ = std::fs::remove_dir_all(&d);
}
