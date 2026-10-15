//! The MS2 scans of the rows of the Dig on the real 3200 series of the private data repository (skipped without it). The numbers come from
//! its `scratchpad/tabui_ms2_lr_truth.json`; none is written here.

use super::*;
use mzlab_feat::legacy::lr::ms2::pool;
use std::path::PathBuf;

fn root() -> Option<PathBuf> {
    let p = match std::env::var("MZLAB_DATI") {
        Ok(v) => PathBuf::from(v),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../mzlab-dati"),
    };
    p.join("scratchpad/tabui_ms2_lr_truth.json")
        .exists()
        .then_some(p)
}

fn index_of_file(path: &std::path::Path) -> Vec<u8> {
    let mzml = std::fs::read(path).expect("mzML");
    let mut src = mzml.clone();
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let p = prepare(&mut src, mzml.len() as u64, &name).unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

#[test]
fn the_row_without_signal_gets_no_ms2_scans() {
    let Some(dati) = root() else {
        eprintln!("SKIP lr_ms2_real: no data repository (MZLAB_DATI)");
        return;
    };
    let truth: Value = serde_json::from_str(
        &std::fs::read_to_string(dati.join("scratchpad/tabui_ms2_lr_truth.json")).unwrap(),
    )
    .unwrap();
    let list = |k: &str| -> Vec<String> {
        truth[k]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                f.as_str()
                    .or_else(|| f["mzml"].as_str())
                    .unwrap()
                    .to_string()
            })
            .collect()
    };
    let (full, ms2) = (list("full_files"), list("ms2_files"));
    let time = |name: &str| -> Option<f64> {
        let t = name
            .rsplit("-t")
            .next()?
            .split(|c: char| !c.is_ascii_digit())
            .next()?;
        t.parse().ok()
    };
    let mut files = Vec::new();
    let mut idxs = Vec::new();
    for (name, kind) in full
        .iter()
        .map(|n| (n, "full"))
        .chain(ms2.iter().map(|n| (n, "ms2")))
    {
        files.push(format!(
            r#"{{"name": "{name}", "time": {}, "type": "sample", "kind": "{kind}"}}"#,
            time(name).unwrap()
        ));
        idxs.push(index_of_file(&dati.join("mzML").join(name)));
    }
    let files = format!("[{}]", files.join(","));
    let parent = format!(
        r#"{{"neutral": "{}"}}"#,
        truth["parent"]["formula"].as_str().unwrap()
    );
    let d = dig_for(&files, &parent, &idxs[..full.len()]);
    let mut op = |k: usize| Ok(open(&idxs[k]));
    let want = truth["experiments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["expect"] == "no_signal")
        .expect("a no_signal experiment");
    let rt = want["no_signal_row_rt"].as_f64().unwrap();
    let row = d
        .entries
        .iter()
        .find(|e| e.unexpected && e.ref_rt.is_some_and(|r| (r - rt).abs() < 0.05))
        .expect("the unexpected row of the truth");
    assert_eq!(pool(&d, row, 6, &mut op).unwrap().scans, 0);
}
