//! A file that holds only MS3 scans (the synthetic QTRAP pair of `tools/wiff_riferimento.py`, sample 2) is an MSn file of level 3: the file
//! info names the level, and the chromatogram, scan list, spectrum and precursor table asked at that level are the MS3 scans, never empty for
//! lack of an MS2 section. Synthetic data only; without python3 the test says so and stops. The MS3 reader is in the readers module (feature `sciex`).

#![cfg(feature = "sciex")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::iteminfo::{facts_from, item_info, Meta};
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::routes::{Query, Routes};
use mzlab_core::upload::prepare;
use serde_json::Value;

/// The mzML of the MS3 sample, written by the reader the site and the app share.
fn ms3_mzml() -> Option<Vec<u8>> {
    let out: PathBuf = std::env::temp_dir().join(format!("mzlab_ms3_only_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
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
        eprintln!("SKIP ms3_only: python3 not available");
        return None;
    }
    let member = |file: &str| {
        let bytes = std::fs::read(out.join(file)).unwrap();
        Member {
            nome: file.into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    let set = FileSet {
        membri: vec![member("ms3.wiff"), member("ms3.wiff.scan")],
    };
    let mut l = readers::apri(set, &Limits::sito()).unwrap();
    let xml = mzlab_core::readers_mzml::scrivi(&mut *l, 1, Vec::new(), &mut |_, _| {}).unwrap();
    let _ = std::fs::remove_dir_all(&out);
    Some(xml)
}

fn index_of(bytes: &[u8]) -> Index<Vec<u8>> {
    let mut src = bytes.to_vec();
    let p = prepare(&mut src, bytes.len() as u64, "t.mzML").unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, bytes, 64 << 20).unwrap();
    let len = idx.len() as u64;
    Index::open(idx, len).unwrap()
}

fn get(r: &mut Routes<Vec<u8>>, path: &str, q: &str) -> Value {
    let body = r
        .handle(path, &Query::parse(q))
        .unwrap()
        .unwrap_or_else(|| panic!("{path}?{q}: declined"));
    serde_json::from_str(&body).unwrap()
}

#[test]
fn an_ms3_only_file_is_read_at_level_3() {
    let Some(xml) = ms3_mzml() else {
        return;
    };
    let mut idx = index_of(&xml);
    assert!(
        idx.scans.iter().all(|s| s.level == 3),
        "the sample holds MS3 scans only"
    );
    let end = idx.scans.last().map_or(0, |s| s.end);
    let facts = facts_from(&mut Mzml::new(xml.clone(), xml.len() as u64), end, 0, false).unwrap();
    let meta = Meta {
        file: "t.mzML".into(),
        label: "t".into(),
        ..Meta::default()
    };
    let info: Value =
        serde_json::from_str(&item_info(&mut idx, &facts, &meta, None).unwrap()).unwrap();
    assert_eq!(info["kind"], "ms2", "{info}");
    assert_eq!(info["msn_level"], 3, "{info}");

    let n = idx.scans.len();
    let mut r = Routes::new(idx);
    let chrom = get(&mut r, "/api/chrom", "k=0&kind=tic&level=3");
    assert_eq!(chrom["rt"].as_array().map_or(0, Vec::len), n, "{chrom}");
    assert!(
        chrom["y"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_f64().unwrap() > 0.0),
        "{chrom}"
    );
    // the precursor of an MS3 scan is its second isolated mass (194.1)
    let by_prec = get(
        &mut r,
        "/api/chrom",
        "k=0&kind=tic&level=3&prec=194.1&ptol=0.6",
    );
    assert_eq!(by_prec["rt"], chrom["rt"]);
    let list = get(&mut r, "/api/scanlist", "k=0&level=3");
    assert_eq!(list["sid"].as_array().map_or(0, Vec::len), n, "{list}");
    let tr = get(&mut r, "/api/ms2tr", "k=0&level=3");
    let tr = tr.as_array().unwrap();
    assert!(
        !tr.is_empty()
            && tr
                .iter()
                .all(|t| (t["q1"].as_f64().unwrap() - 194.1).abs() < 0.06),
        "{tr:?}"
    );
    // at level 2 nothing is there: empty, never the MS3 scans under another name
    assert_eq!(
        get(&mut r, "/api/ms2tr", "k=0&level=2"),
        serde_json::json!([])
    );
}
