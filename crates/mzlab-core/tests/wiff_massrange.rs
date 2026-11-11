//! The MRM table of a `.wiff` read by the counted grammar of `MassRangeEx`: the pairs of `tools/wiff_massrange.py` (an encoder written from the grammar)
//! give every announced transition, an unnamed, a non-ASCII and a 151-character name included, through `Wiff::open`/`run` and the `Lettore` the site reads; a stream that
//! does not hold the count of its `ExperimentHeader` refuses the sample with `err.file.wiffMassRange`, in both. Skipped without python3.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::methodinfo::Kind;
use mzlab_core::mzml::ChromKind;
use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_massrange_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_massrange.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_massrange: python3 not available");
        return None;
    }
    let json = serde_json::from_slice(&std::fs::read(out.join("massrange.json")).unwrap()).unwrap();
    Some((out, json))
}

fn set(dir: &Path, name: &str) -> FileSet {
    let member = |file: String| {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        Member {
            nome: file,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    FileSet {
        membri: vec![
            member(format!("{name}.wiff")),
            member(format!("{name}.wiff.scan")),
        ],
    }
}

fn open(dir: &Path, name: &str) -> (mzlab_core::error::Result<Wiff>, Vec<u8>) {
    let ole = container(
        std::fs::read(dir.join(format!("{name}.wiff"))).unwrap(),
        name,
    )
    .unwrap();
    let scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
    (Wiff::open(&ole, name, Some(scan.len() as u64), 1), scan)
}

fn param<'a>(e: &'a mzlab_core::error::Error, k: &str) -> &'a str {
    e.params
        .iter()
        .find(|(n, _)| *n == k)
        .map(|(_, v)| v.as_str())
        .unwrap_or("")
}

#[test]
fn every_announced_transition_is_read() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let rows = want["tabella"].as_array().unwrap();
    let (w, mut scan) = open(&dir, "tabella");
    let w = w.unwrap();
    let Some(Kind::Mrm(tr)) = w.method.as_ref().map(|m| &m.experiments[0].kind) else {
        panic!("no MRM table");
    };
    assert_eq!(tr.len(), rows.len());
    for (t, r) in tr.iter().zip(rows) {
        assert_eq!(
            (t.q1, t.q3, t.dwell, t.name.as_str(), t.ce, t.dp, t.cxp),
            (
                r["q1"].as_f64().unwrap(),
                r["q3"].as_f64().unwrap(),
                r["dwell"].as_f64().unwrap(),
                r["name"].as_str().unwrap(),
                r["ce"].as_f64(),
                r["dp"].as_f64(),
                r["cxp"].as_f64()
            )
        );
    }
    let run = w.run(&mut scan).unwrap();
    let srm: Vec<_> = run
        .chromatograms
        .iter()
        .filter(|c| c.kind == ChromKind::Srm)
        .collect();
    assert_eq!(srm.len(), rows.len());
    assert!(srm[2].id.ends_with("name=Qual-é"), "{}", srm[2].id);
    let mut l = readers::apri(set(&dir, "tabella"), &Limits::sito()).unwrap();
    let chroms = l.chromatograms(0).unwrap();
    let ids: Vec<&str> = chroms.iter().map(|c| c.id.as_str()).collect();
    let want_ids: Vec<&str> = run.chromatograms.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, want_ids, "engine and Lettore");
}

#[test]
fn a_table_that_disagrees_with_its_header_is_refused() {
    let Some((dir, want)) = reference() else {
        return;
    };
    for (name, expected) in want["rifiuti"].as_object().unwrap() {
        let e = open(&dir, name)
            .0
            .err()
            .unwrap_or_else(|| panic!("{name} opened"));
        assert_eq!(e.key, "err.file.wiffMassRange", "{name}");
        assert_eq!(
            (param(&e, "experiment"), param(&e, "expected")),
            ("1", expected.to_string().as_str()),
            "{name}"
        );
        let e = readers::apri(set(&dir, name), &Limits::sito())
            .err()
            .unwrap_or_else(|| panic!("{name}: Lettore"));
        assert_eq!(e.key, "err.file.wiffMassRange", "{name}: Lettore");
    }
}
