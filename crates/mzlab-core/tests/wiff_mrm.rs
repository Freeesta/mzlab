//! The MRM transitions of a `.wiff` method (Q1, Q3, collision energy, dwell, name) reach the chromatograms, the mzML and the `/api/mrm` reader
//! (`transizioni` of `tools/wiff_riferimento.py`). Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::mzmlhead::Mzml;
use serde_json::Value;

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_mrm_{}", std::process::id()));
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
        eprintln!("SKIP wiff_mrm: python3 not available");
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
fn the_transitions_of_the_method_survive_wiff_to_mzml_to_read() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let want = want["transizioni"]["transitions"].as_array().unwrap();
    let mut l = lettori::apri(pair(&dir, "transizioni"), &Limiti::sito()).unwrap();
    let bytes = mzlab_core::lettori_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    // every precursor of a transition has its activation, the energy and the dwell are written for the readers that know them
    assert_eq!(text.matches("<activation>").count(), want.len());
    // the dwell is written only by the SCIEX readers module
    let with_dwell = cfg!(feature = "sciex");
    assert_eq!(
        text.matches("MS_dwell_time").count(),
        if with_dwell { want.len() } else { 0 }
    );

    let size = bytes.len() as u64;
    let mut m = Mzml::new(bytes, size);
    let chroms = m.chromatograms(0).unwrap();
    let srm: Vec<_> = chroms.iter().filter(|c| c.kind == "srm").collect();
    assert_eq!(srm.len(), want.len());
    for (c, w) in srm.iter().zip(want) {
        assert_eq!((c.q1, c.q3), (w["q1"].as_f64(), w["q3"].as_f64()));
        assert_eq!(c.ce, w["ce"].as_f64(), "{}", c.id);
        assert_eq!(
            c.dwell,
            w["dwell"].as_f64().filter(|_| with_dwell),
            "{}",
            c.id
        );
        assert_eq!(c.name, w["name"].as_str().unwrap());
    }
}

/// A batch run stores one method for several samples: every sample takes its transitions (Q1, Q3, name) from the nearest method before it,
/// in the engine build (no `estesi`) as well as in the readers module.
#[test]
fn samples_without_a_method_of_their_own_read_the_transitions_of_method_one() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let want = want["transizioni"]["transitions"].as_array().unwrap();
    let ole = mzlab_core::wiff::container(
        std::fs::read(dir.join("transizioni_lotto.wiff")).unwrap(),
        "transizioni_lotto",
    )
    .unwrap();
    let mut scan = std::fs::read(dir.join("transizioni_lotto.wiff.scan")).unwrap();
    for n in 1..=3 {
        let w = mzlab_core::wiff::Wiff::open(&ole, "transizioni_lotto", Some(scan.len() as u64), n)
            .unwrap();
        let run = w.run(&mut scan).unwrap();
        let srm: Vec<_> = run
            .chromatograms
            .iter()
            .filter(|c| c.kind == mzlab_core::mzml::ChromKind::Srm)
            .collect();
        assert_eq!(srm.len(), want.len(), "sample {n}");
        for (c, t) in srm.iter().zip(want) {
            assert_eq!(
                (c.q1, c.q3),
                (t["q1"].as_f64(), t["q3"].as_f64()),
                "sample {n}"
            );
            assert!(
                c.id.ends_with(&format!("name={}", t["name"].as_str().unwrap())),
                "{}",
                c.id
            );
        }
    }
}
