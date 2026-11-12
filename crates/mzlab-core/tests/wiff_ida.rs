//! Dependent (IDA) product-ion scans of a `.wiff` carry no precursor: the mass of their method header is not the one the survey picked, and the
//! real-time stream that holds it is not decoded. Checked on `ida` of `tools/wiff_riferimento.py` (an independent writer) through `Wiff::run`, the
//! `Lettore` index (what the site reads) and the mzML; the sample without that stream keeps its fixed masses. The index rows of the IDA sample hold no
//! point count, so none of its blocks is counted as damaged. Skipped without python3.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_ida_{}", std::process::id()));
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
        eprintln!("SKIP wiff_ida: python3 not available");
        return None;
    }
    let json =
        serde_json::from_slice(&std::fs::read(out.join("riferimento.json")).unwrap()).unwrap();
    Some((out, json))
}

fn set(dir: &Path) -> FileSet {
    let member = |file: &str| {
        let bytes = std::fs::read(dir.join(file)).unwrap();
        Member {
            nome: file.into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    FileSet {
        membri: vec![member("ida.wiff"), member("ida.wiff.scan")],
    }
}

#[test]
fn dependent_product_scans_get_no_invented_precursor() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let ole = container(std::fs::read(dir.join("ida.wiff")).unwrap(), "ida.wiff").unwrap();
    let mut scan = std::fs::read(dir.join("ida.wiff.scan")).unwrap();
    let mut l = readers::apri(set(&dir), &Limits::sito()).unwrap();
    for (c, n) in [1u32, 2].into_iter().enumerate() {
        let rows = want["ida"][n.to_string()].as_array().unwrap();
        let w = Wiff::open(&ole, "ida.wiff", Some(scan.len() as u64), n).unwrap();
        let run = w.run(&mut scan).unwrap();
        let index = l.index(c).unwrap().to_vec();
        assert_eq!(run.spectra.len(), rows.len(), "sample {n}");
        assert_eq!(index.len(), rows.len(), "sample {n}: index");
        let lost = l.losses(c).unwrap();
        assert!(
            !lost.partial(),
            "sample {n}: no block is damaged ({lost:?})"
        );
        for ((g, i), e) in run.spectra.iter().zip(&index).zip(rows) {
            let p = e["precursor"].as_f64();
            assert_eq!(g.id, e["id"].as_str().unwrap());
            assert_eq!(
                (i64::from(g.level), g.precursor),
                (e["level"].as_i64().unwrap(), p),
                "{}",
                g.id
            );
            assert_eq!((i.id.as_str(), i.precursore), (g.id.as_str(), p), "index");
        }
        let mut mzml = Vec::new();
        let conv = mzlab_core::readers_mzml::Conversion {
            steps: Some(&[]),
            ..Default::default()
        };
        conv.write(&mut *l, c, &mut mzml, &mut |_, _| {}).unwrap();
        let text = String::from_utf8(mzml).unwrap();
        assert_eq!(
            text.matches("<precursorList").count(),
            if n == 1 { 0 } else { 6 },
            "sample {n}: precursor lists in the mzML"
        );
    }
}
