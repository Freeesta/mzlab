//! The recognition of an infusion (`infusione.rs`) on whole files: the committed cycled MSn file (`tests/golden/ref/msntree_rotta`) is NOT an
//! infusion without a hint, the synthetic infusion of `TP_Mine/tests/msn_synth.py` (one path at a time) is (skipped without python3 and
//! numpy), and the files of the private data repository answer as `scratchpad/verita_infusione_msn.json` says (skipped without it).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::{Index, Scan};
use mzlab_core::infusione::infusione;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The scan rows of an mzML, indexed in memory.
fn scans_of(mzml: Vec<u8>, name: &str) -> Vec<Scan> {
    let mut src = mzml.clone();
    let p = prepare(&mut src, mzml.len() as u64, name).unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &mzml, 256 << 20).unwrap();
    let len = idx.len() as u64;
    Index::open(idx, len).unwrap().scans
}

#[test]
fn cycled_msn_needs_a_hint() {
    let d = root().join("tests/golden/ref/msntree_rotta/msn.mzidx");
    let len = std::fs::metadata(&d).unwrap().len();
    let scans = Index::open(std::fs::File::open(&d).unwrap(), len)
        .unwrap()
        .scans;
    // fixed precursors (B) but MS1 -> MS2 -> MS3 cycled (no C): the LC targeted run looks the same
    assert_eq!(infusione(&scans, None, "msn.mzML"), None);
    assert_eq!(infusione(&scans, Some(true), "msn.mzML"), None);
    let m = infusione(&scans, Some(false), "msn.mzML").unwrap();
    assert_eq!(m.motivo, "metodo_senza_lc");
    assert_eq!(
        infusione(&scans, None, "msn_FIA.mzML").map(|m| m.motivo),
        Some("nome")
    );
}

#[test]
fn synthetic_tune_infusion_is_recognised() {
    let out = std::env::temp_dir().join(format!("mzlab-infusione-{}.mzML", std::process::id()));
    let code = "import sys; sys.path.insert(0, 'TP_Mine/tests'); import msn_synth as s; s.write_msn(sys.argv[1], s.caffeine_nodes())";
    let run = Command::new("python3")
        .current_dir(root())
        .args(["-c", code])
        .arg(&out)
        .output();
    if !matches!(&run, Ok(o) if o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "the synthetic file was not written: {run:?}"
        );
        eprintln!("python3/numpy not available: test skipped");
        return;
    }
    let scans = scans_of(std::fs::read(&out).unwrap(), "sample.mzML");
    let _ = std::fs::remove_file(&out);
    let m = infusione(&scans, None, "sample.mzML").unwrap();
    // 6 paths with their energies, 5 nominal nodes (195>138 at two energies), down to MS5
    assert_eq!((m.motivo, m.livelli, m.nodi), ("blocchi", 5, 5));
}

#[test]
fn real_files_like_the_truth() {
    let Some(truth_file) = dati_percorsi::percorso("scratchpad/verita_infusione_msn.json") else {
        eprintln!("skipped: no data repository (MZLAB_DATI)");
        return;
    };
    // The `file` entries of the truth are relative to `dati/` (`HRMS/...`).
    let truth: serde_json::Value =
        serde_json::from_slice(&std::fs::read(truth_file).unwrap()).unwrap();
    let mut seen = 0;
    for (group, positive) in [("positivi", true), ("negativi", false)] {
        for f in truth[group].as_array().unwrap() {
            let Some(p) = dati_percorsi::percorso(&format!("dati/{}", f["file"].as_str().unwrap()))
            else {
                continue;
            };
            let Ok(mzml) = std::fs::read(&p) else {
                continue;
            };
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            let scans = scans_of(mzml, &name);
            assert_eq!(
                scans.len() as u64,
                f["scansioni"].as_u64().unwrap(),
                "{group} {seen}"
            );
            let got = infusione(&scans, None, &name);
            match positive {
                true => {
                    let m = got.unwrap_or_else(|| panic!("{group} {seen}: not an infusion"));
                    assert_eq!(m.motivo, f["motivo"].as_str().unwrap());
                    assert_eq!(m.nodi as u64, f["nominali"].as_u64().unwrap());
                    let top = f["livelli"]
                        .as_object()
                        .unwrap()
                        .keys()
                        .max_by_key(|k| k.parse::<u8>().unwrap());
                    assert_eq!(m.livelli.to_string(), *top.unwrap());
                }
                false => assert_eq!(got, None, "{group} {seen}"),
            }
            seen += 1;
        }
    }
    eprintln!("{seen} real files checked");
}
