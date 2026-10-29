//! The `.wiff` engine without the readers module reads scan blocks of versions 1, 2 and 3 and one polarity rule: the header of the experiment (offset 86), else the
//! sign of the ion spray voltage. Synthetic pair `blocchi` of `tools/wiff_riferimento.py` (skipped without python3; `MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::Path;
use std::process::Command;

use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

#[test]
fn blocks_v1_v2_v3_are_read_with_the_polarity_of_the_header() {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_blocchi_{}", std::process::id()));
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
        eprintln!("SKIP wiff_blocchi_v13: python3 not available");
        return;
    }
    let want: Value =
        serde_json::from_slice(&std::fs::read(out.join("riferimento.json")).unwrap()).unwrap();
    let want = &want["blocchi"];
    let ole = container(std::fs::read(out.join("blocchi.wiff")).unwrap(), "blocchi").unwrap();
    let mut scan = std::fs::read(out.join("blocchi.wiff.scan")).unwrap();
    let w = Wiff::open(&ole, "blocchi", Some(scan.len() as u64), 1).unwrap();
    let run = w.run(&mut scan).unwrap();
    let sp = want["spectra"].as_array().unwrap();
    assert_eq!(run.spectra.len(), sp.len());
    let pol = want["polarity"].as_array().unwrap();
    for (k, (g, e)) in run.spectra.iter().zip(sp).enumerate() {
        assert_eq!(g.id, e["id"].as_str().unwrap());
        assert_eq!(g.level as i64, e["level"].as_i64().unwrap(), "{}", g.id);
        assert_eq!(g.polarity as i64, pol[k % 3].as_i64().unwrap(), "{}", g.id);
        let mz: Vec<f64> = e["mz"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        let it: Vec<f64> = e["intensity"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        assert_eq!(g.mz.len(), mz.len(), "{}: points", g.id);
        assert!(
            (0..mz.len()).all(|j| close(g.mz[j], mz[j]) && close(g.intensity_f64(j), it[j])),
            "{}",
            g.id
        );
    }
    // a block nobody can read is an error with the cause, never an empty scan
    for (name, text) in [
        ("versione4", "scan block version 4"),
        ("bloccotof", "TOF block"),
    ] {
        let ole = container(
            std::fs::read(out.join(format!("{name}.wiff"))).unwrap(),
            name,
        )
        .unwrap();
        let mut scan = std::fs::read(out.join(format!("{name}.wiff.scan"))).unwrap();
        let w = Wiff::open(&ole, name, Some(scan.len() as u64), 1).unwrap();
        let err = w
            .run(&mut scan)
            .err()
            .unwrap_or_else(|| panic!("{name}: read"));
        assert_eq!(err.key, "err.file.wiff", "{name}");
        assert!(
            err.params.iter().any(|(k, v)| *k == "detail" && v == text),
            "{name}"
        );
    }
    let _ = std::fs::remove_dir_all(&out);
}
