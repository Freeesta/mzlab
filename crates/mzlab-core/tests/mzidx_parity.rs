//! The Rust `.mzidx` builder writes byte for byte the file of the Python builder (`tools/mzidx_riferimento.py`), on the example
//! files and the synthetic ones, with a large budget (peaks kept from the first pass) and a tiny one (rounds of buckets).
//! Without python3 and numpy the test is skipped (`MZLAB_GOLDEN_STRICT=1` fails instead).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::mzidx::{build_mzml_to_vec, ScanTable};
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Deserialize)]
struct Meta {
    source: Box<RawValue>,
    run: Box<RawValue>,
    sections: Vec<Section>,
}
#[derive(Deserialize)]
struct Section {
    dtype: String,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn u64_at(b: &[u8], at: usize) -> usize {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap()) as usize
}

/// The caller's side: everything the Python reader knew, taken from the reference file.
fn table_of(reference: &[u8]) -> ScanTable {
    let (meta_off, meta_len, scans_off, scans_len) = (
        u64_at(reference, 16),
        u64_at(reference, 24),
        u64_at(reference, 32),
        u64_at(reference, 40),
    );
    let meta: Meta = serde_json::from_slice(&reference[meta_off..meta_off + meta_len]).unwrap();
    let scans = &reference[scans_off..scans_off + scans_len];
    let rows: Vec<Vec<serde_json::Value>> = serde_json::from_slice(scans).unwrap();
    let col = |c: usize| {
        rows.iter()
            .map(|r| r[c].as_f64().unwrap())
            .collect::<Vec<f64>>()
    };
    ScanTable {
        level: col(2).iter().map(|&v| v as u8).collect(),
        polarity: col(4).iter().map(|&v| v as i8).collect(),
        rt: col(3),
        tic: col(5),
        scans_json: scans.to_vec(),
        source_json: meta.source.get().to_string(),
        run_json: meta.run.get().to_string(),
        hr1: meta.sections.iter().any(|s| s.dtype == "<f4"),
    }
}

fn compare(name: &str, mzml: &Path, idx: &Path) {
    let reference = std::fs::read(idx).unwrap();
    let data = std::fs::read(mzml).unwrap();
    let table = table_of(&reference);
    let (got, st) =
        build_mzml_to_vec(&table, &data, 64 << 20).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(st.passes <= 2, "{name}: {} passes", st.passes);
    same(name, 64 << 20, &got, &reference);
    // a budget of a fifth of the peaks: several rounds, each decoding the file again
    let mut budget = (st.peaks as usize * 24 / 5).max(1 << 14);
    loop {
        match build_mzml_to_vec(&table, &data, budget) {
            Ok((got, st)) => {
                if st.peaks > 2000 {
                    assert!(
                        st.passes >= 3,
                        "{name}: {} passes with a budget of {budget}",
                        st.passes
                    );
                }
                return same(name, budget, &got, &reference);
            }
            Err(e) if e.key == "error.memoryBudget" && budget < 1 << 30 => budget *= 2,
            Err(e) => panic!("{name}: {e}"),
        }
    }
}

fn same(name: &str, budget: usize, got: &[u8], reference: &[u8]) {
    if got != reference {
        let at = got.iter().zip(reference).position(|(a, b)| a != b);
        panic!(
            "{name} (budget {budget}): {} bytes against {}, first difference at {:?}",
            got.len(),
            reference.len(),
            at
        );
    }
}

#[test]
fn rust_index_is_identical_to_the_python_one() {
    let out = std::env::temp_dir().join(format!("mzlab-idx-{}", std::process::id()));
    let mut cmd = Command::new("python3");
    cmd.current_dir(root())
        .arg("tools/mzidx_riferimento.py")
        .arg(&out);
    if let Ok(rd) = std::fs::read_dir(root().join("mzlab/web/esempi")) {
        let mut v: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "mzML"))
            .collect();
        v.sort();
        cmd.args(v);
    }
    let res = cmd.output();
    let listing = match res {
        Ok(o) if o.status.success() => String::from_utf8(o.stdout).unwrap(),
        other => {
            if std::env::var("MZLAB_GOLDEN_STRICT").is_ok() {
                panic!("the reference builder did not run: {other:?}");
            }
            eprintln!("python3/numpy not available: test skipped");
            return;
        }
    };
    let mut n = 0;
    for line in listing.lines().filter(|l| !l.is_empty()) {
        let mzml = PathBuf::from(line);
        let stem = mzml.file_stem().unwrap().to_string_lossy().to_string();
        compare(&stem, &mzml, &out.join(format!("{stem}.mzidx")));
        n += 1;
    }
    assert!(n >= 8);
    let _ = std::fs::remove_dir_all(&out);
}
