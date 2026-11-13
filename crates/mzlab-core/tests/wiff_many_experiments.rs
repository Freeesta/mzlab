//! A `.wiff` method with more than sixteen experiments keeps the collision energy of every one of them, in `Wiff::open` and in the mzML.
//! Checked on `tools/wiff_esperimenti.py` (an independent writer) and against the Python reference `mzlab/reader/methodinfo.py`; fails without python3.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::wiff::{container, samples, Wiff};
use serde_json::Value;

fn reference() -> (PathBuf, Value) {
    // One directory per call: the tests run in parallel and must not read a file another is rewriting.
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let out = std::env::temp_dir().join(format!("mzlab_wiff_many_exp_{}_{n}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_esperimenti.py");
    let o = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .expect("python3 writes the synthetic .wiff");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let json =
        serde_json::from_slice(&std::fs::read(out.join("esperimenti.json")).unwrap()).unwrap();
    (out, json)
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
        membri: vec![member("esperimenti.wiff"), member("esperimenti.wiff.scan")],
    }
}

#[test]
fn experiments_past_the_sixteenth_keep_their_collision_energy() {
    let (dir, want) = reference();
    let ces: Vec<Option<f64>> = want["ce"]
        .as_array()
        .unwrap()
        .iter()
        .map(Value::as_f64)
        .collect();
    let bytes = std::fs::read(dir.join("esperimenti.wiff")).unwrap();
    let ole = container(bytes, "esperimenti").unwrap();
    let scan_len = std::fs::metadata(dir.join("esperimenti.wiff.scan"))
        .unwrap()
        .len();
    let w = Wiff::open(&ole, "esperimenti", Some(scan_len), samples(&ole)[0]).unwrap();
    let got: Vec<Option<f64>> = w.experiments.iter().map(|e| e.ce).collect();
    assert_eq!(got, ces);

    let mut l = readers::apri(set(&dir), &Limits::sito()).unwrap();
    let mzml = String::from_utf8(
        mzlab_core::readers_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap(),
    )
    .unwrap();
    for ce in ["21", "32.5", "44"] {
        let term = format!("accession=\"MS:1000045\" name=\"collision energy\" value=\"{ce}\"");
        assert_eq!(mzml.matches(&term).count(), 2, "CE {ce}");
    }
}

/// The method document names the acquisition software the log names (SCIEX OS), Analyst otherwise; the model comes from `Component Id:` as SCIEX OS spells it.
#[test]
fn the_method_names_sciex_os_when_its_log_does() {
    let (dir, _) = reference();
    let bytes = std::fs::read(dir.join("esperimenti.wiff")).unwrap();
    let ole = container(bytes, "esperimenti").unwrap();
    let src = mzlab_core::method::Source::default();
    let doc = |n| -> Value {
        serde_json::from_str(&mzlab_core::method_wiff::from_wiff(&ole, n, &src)).unwrap()
    };
    let name = |d: &Value| {
        d["strumento"]["software"][0]["nome"]
            .as_str()
            .map(String::from)
    };
    assert_eq!(name(&doc(1)).as_deref(), Some("Analyst"));
    let os = doc(2);
    assert_eq!(name(&os).as_deref(), Some("SCIEX OS"), "{os}");
    assert_eq!(
        os["strumento"]["modello"].as_str(),
        Some("QTRAP 6500+"),
        "{os}"
    );
}

fn energies(want: &Value, key: &str) -> Vec<Option<f64>> {
    want[key]
        .as_array()
        .unwrap()
        .iter()
        .map(Value::as_f64)
        .collect()
}

/// A MassRangeEx stream cut inside its energy is not fatal: the experiment is still counted, without energy, and the ones after it keep theirs.
#[test]
fn a_cut_experiment_stream_is_counted_without_its_energy() {
    let (dir, want) = reference();
    let bytes = std::fs::read(dir.join("esperimenti.wiff")).unwrap();
    let ole = container(bytes.clone(), "esperimenti").unwrap();
    let scan_len = std::fs::metadata(dir.join("esperimenti.wiff.scan"))
        .unwrap()
        .len();
    let w = Wiff::open(&ole, "esperimenti", Some(scan_len), samples(&ole)[2]).unwrap();
    let got: Vec<Option<f64>> = w.experiments.iter().map(|e| e.ce).collect();
    assert_eq!(got, energies(&want, "ce_cut"));
    let methods = mzlab_core::methodinfo::read_methods(bytes, "esperimenti.wiff").unwrap();
    assert_eq!(methods[2].experiments.len(), 20);
}

/// The experiments of every method (index and energies) are the ones the Python reference reads.
#[test]
fn the_experiments_match_the_python_reference() {
    let (dir, _) = reference();
    let wiff = dir.join("esperimenti.wiff");
    let code = "import json, sys\nfrom mzlab.reader.methodinfo import read_methods\nprint(json.dumps([[[e['index'], e.get('ce')] for e in m['experiments']] for m in read_methods(sys.argv[1])]))";
    let o = Command::new("python3")
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args(["-c", code])
        .arg(&wiff)
        .output()
        .expect("python3 reads the methods");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let py: Value = serde_json::from_slice(&o.stdout).unwrap();
    let methods =
        mzlab_core::methodinfo::read_methods(std::fs::read(&wiff).unwrap(), "esperimenti.wiff")
            .unwrap();
    let rust: Value = methods
        .iter()
        .map(|m| {
            m.experiments
                .iter()
                .map(|e| match &e.kind {
                    mzlab_core::methodinfo::Kind::Scan { ce, .. } => {
                        serde_json::json!([e.index, ce])
                    }
                    mzlab_core::methodinfo::Kind::Mrm(_) => serde_json::json!([e.index, null]),
                })
                .collect::<Value>()
        })
        .collect();
    assert_eq!(rust, py);
    assert_eq!(rust[0].as_array().unwrap().len(), 20);
}
