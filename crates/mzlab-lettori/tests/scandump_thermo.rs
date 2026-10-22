//! The scandump/1 dumps of RFR (private corpus, `scratchpad/rfr/<file>.scandump.jsonl.gz`) against our reader (docs/agenti/scandump.md): for each dump the example `thermo_scandump`
//! reads the same `.raw` and `tools/rosina/scandump_compare.py` writes the report. The test reports; it fails only on the fields listed in `VERIFICATI`
//! (compared at least once, no difference, nothing missing on our side). Skipped without `MZLAB_DATI`.
#![cfg(feature = "thermo")]

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Fields that the bench has proved equal to RFR on the corpus; a new name here makes a difference in it a failure. Empty at the start: the test only reports.
const VERIFICATI: &[&str] = &[];

fn dumps(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".scandump.jsonl.gz"))
        .collect();
    v.sort();
    v
}

/// The `.raw` named `nome` anywhere under `dir` (the corpus has a few folders deep).
fn trova(dir: &Path, nome: &str, fondo: usize) -> Option<PathBuf> {
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() && fondo > 0 {
            if let Some(f) = trova(&p, nome, fondo - 1) {
                return Some(f);
            }
        } else if p.file_name().is_some_and(|n| n == nome) {
            return Some(p);
        }
    }
    None
}

/// The built example next to the test binary (`cargo test` builds it), else `cargo run`.
fn nostro(raw: &Path, out: &Path) -> bool {
    let exe = std::env::current_exe().ok();
    let ex = exe.as_ref().and_then(|e| {
        e.parent()?
            .parent()
            .map(|d| d.join("examples/thermo_scandump"))
    });
    let st = match ex.filter(|p| p.is_file()) {
        Some(p) => Command::new(p).arg(raw).arg(out).status(),
        None => Command::new(env!("CARGO"))
            .args([
                "run",
                "--release",
                "-q",
                "-p",
                "mzlab-lettori",
                "--features",
                "thermo",
                "--example",
                "thermo_scandump",
                "--",
            ])
            .arg(raw)
            .arg(out)
            .status(),
    };
    st.is_ok_and(|s| s.success())
}

#[test]
fn scandump_thermo() {
    let Some(dir) = dati_percorsi::percorso("scratchpad/rfr") else {
        eprintln!("SKIP scandump_thermo: no data repository or no dumps of RFR");
        return;
    };
    let Some(dati) = dati_percorsi::percorso("dati") else {
        return;
    };
    let tool = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/rosina/scandump_compare.py");
    let tmp = std::env::temp_dir().join(format!("scandump_thermo_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("tmp");
    let (mut fatti, mut rotti) = (0, Vec::new());
    for (i, rfr) in dumps(&dir).iter().enumerate() {
        let stem = rfr
            .file_name()
            .unwrap()
            .to_string_lossy()
            .replace(".scandump.jsonl.gz", "");
        let Some(raw) = trova(&dati, &stem, 4) else {
            eprintln!("dump {i}: its .raw is not in the corpus, skipped");
            continue;
        };
        let (mio, rep) = (
            tmp.join(format!("{i}.jsonl.gz")),
            tmp.join(format!("{i}.json")),
        );
        if !nostro(&raw, &mio) {
            eprintln!("dump {i}: our reader does not open the file (exit code 2 of the example)");
            continue;
        }
        let run = Command::new("python3")
            .arg(&tool)
            .arg(rfr)
            .arg(&mio)
            .args(["--tol", "hr", "--out"])
            .arg(&rep)
            .output();
        let Some(_) = run.ok().filter(|o| o.status.success()) else {
            eprintln!("python3 is not available or the comparator failed");
            break;
        };
        let r: Value = serde_json::from_slice(&std::fs::read(&rep).expect("report")).expect("json");
        let campi = r["fields"].as_object().expect("fields");
        let (uguali, diversi, assenti) = campi.values().fold((0, 0, 0), |a, v| {
            (
                a.0 + v["equal"].as_u64().unwrap_or(0),
                a.1 + v["different"].as_u64().unwrap_or(0),
                a.2 + v["absent_b"].as_u64().unwrap_or(0),
            )
        });
        eprintln!("dump {i}: {} fields, equal {uguali}, different {diversi}, missing on our side {assenti}", campi.len());
        for nome in VERIFICATI {
            match campi.get(*nome) {
                Some(v)
                    if v["compared"].as_u64().unwrap_or(0) > 0
                        && v["different"] == 0
                        && v["absent_b"] == 0 => {}
                other => rotti.push(format!(
                    "dump {i}: {nome}: {}",
                    other.map_or("not compared".into(), |v| v["examples"].to_string())
                )),
            }
        }
        fatti += 1;
    }
    std::fs::remove_dir_all(&tmp).ok();
    eprintln!("scandump_thermo: {fatti} dumps compared");
    assert!(
        rotti.is_empty(),
        "fields declared verified differ from RFR:\n{}",
        rotti.join("\n")
    );
}
