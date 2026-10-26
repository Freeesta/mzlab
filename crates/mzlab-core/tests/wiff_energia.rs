//! The collision energy and the isolation window of the product-ion experiments of a `.wiff` reach the spectra, the mzML and the scan table of the index
//! (`energia` of `tools/wiff_riferimento.py`). Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::wiff::{container, samples, Wiff};
use serde_json::Value;

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_energia_{}", std::process::id()));
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
        eprintln!("SKIP wiff_energia: python3 not available");
        return None;
    }
    let json =
        serde_json::from_slice(&std::fs::read(out.join("riferimento.json")).unwrap()).unwrap();
    Some((out, json))
}

fn pair(dir: &Path, name: &str) -> FileSet {
    let member = |file: String| {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        Membro {
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

#[test]
fn the_collision_energy_of_each_experiment_is_read_and_written() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let want = &want["energia"];
    let ces: Vec<Option<f64>> = want["ce"]
        .as_array()
        .unwrap()
        .iter()
        .map(Value::as_f64)
        .collect();
    let bytes = std::fs::read(dir.join("energia.wiff")).unwrap();
    let ole = container(bytes, "energia").unwrap();
    let scan_len = std::fs::metadata(dir.join("energia.wiff.scan"))
        .unwrap()
        .len();
    let w = Wiff::open(&ole, "energia", Some(scan_len), samples(&ole)[0]).unwrap();
    let got: Vec<Option<f64>> = w.experiments.iter().map(|e| e.ce).collect();
    assert_eq!(got, ces);
    assert!(w
        .experiments
        .iter()
        .all(|e| e.iso_half.is_some() == (e.level == 2)));

    // the mzML: one activation per product-ion spectrum, with its energy and the offsets of the isolation window
    let mut l = lettori::apri(pair(&dir, "energia"), &Limiti::sito()).unwrap();
    let mzml = String::from_utf8(
        mzlab_core::lettori_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap(),
    )
    .unwrap();
    assert_eq!(mzml.matches("<activation>").count(), 6);
    assert_eq!(
        mzml.matches("accession=\"MS:1000045\" name=\"collision energy\" value=\"25\"")
            .count(),
        2
    );
    assert_eq!(
        mzml.matches("accession=\"MS:1000045\" name=\"collision energy\" value=\"35\"")
            .count(),
        2
    );
    assert_eq!(mzml.matches("MS:1000045").count(), 4);
    assert_eq!(
        mzml.matches("isolation window lower offset\" value=\"0.35\"")
            .count(),
        6
    );

    // the scan table of the index: energy (column 7), isolation window (11) and activation (12) of each product-ion row
    let mut t = mzlab_core::mzidx_lettore::righe(&mut *l, 0).unwrap();
    let rows: Value = serde_json::from_str(&mzlab_core::mzidx_lettore::rows_json(
        &mut t,
        l.indice(0).unwrap(),
    ))
    .unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 8);
    for (k, r) in rows.iter().enumerate() {
        assert_eq!(r[7].as_f64(), ces[k % 4], "row {k}");
        if k % 4 != 0 {
            assert_eq!(r[12].as_str(), Some("CID"), "row {k}");
            let (lo, hi) = (r[11][0].as_f64().unwrap(), r[11][1].as_f64().unwrap());
            assert!((hi - lo - 0.7).abs() < 1e-9, "row {k}: {lo} {hi}");
        }
    }
}
