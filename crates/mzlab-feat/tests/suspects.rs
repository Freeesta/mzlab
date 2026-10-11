//! `suspects::Suspects` answers like `leggi_sospetti().cerca` of Python on a synthetic mini index (`tools/sospetti_riferimento.py`; skipped without
//! python3, `MZLAB_GOLDEN_STRICT=1` fails instead), rejects bad files, and searches 10^6 synthetic rows in well under a millisecond.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use mzlab_feat::suspects::{Suspects, SuspectsError};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pad8(v: &mut Vec<u8>) {
    while !v.len().is_multiple_of(8) {
        v.push(0);
    }
}

/// A valid `.mzsus` with `n` rows: mass 100 + i*1e-3, key of letters, one formula, no reactions.
fn synth(n: usize) -> Vec<u8> {
    let man = b"{}";
    let mut v = Vec::new();
    v.extend_from_slice(b"MZSU");
    for x in [1u16, 1] {
        v.extend_from_slice(&x.to_le_bytes());
    }
    for x in [n as u32, 1, 0, man.len() as u32, 0, 0] {
        v.extend_from_slice(&x.to_le_bytes());
    }
    let sec = |bytes: Vec<u8>, v: &mut Vec<u8>| {
        v.extend_from_slice(&bytes);
        pad8(v);
    };
    sec(man.to_vec(), &mut v);
    sec(
        (0..n)
            .flat_map(|i| (100.0 + i as f64 * 1e-3).to_le_bytes())
            .collect(),
        &mut v,
    );
    sec(
        (0..n)
            .flat_map(|i| {
                format!("{:014}", i)
                    .bytes()
                    .map(|d| b'A' + (d - b'0'))
                    .collect::<Vec<_>>()
            })
            .collect(),
        &mut v,
    );
    sec(vec![0u8; 4 * n], &mut v);
    sec(vec![2u8; n], &mut v);
    sec(vec![0u8; 2 * n], &mut v);
    sec(vec![0u8; 2 * n], &mut v);
    sec(vec![], &mut v);
    sec(
        [0u32, 7].iter().flat_map(|x| x.to_le_bytes()).collect(),
        &mut v,
    );
    sec(b"C6H12O6".to_vec(), &mut v);
    v
}

#[test]
fn rejects_bad_files() {
    assert!(matches!(
        Suspects::open(b"nope nope nope nope nope nope nope nope"),
        Err(SuspectsError::Corrupt(_))
    ));
    let mut v = synth(10);
    v[6] = 9;
    assert_eq!(
        Suspects::open(&v).err(),
        Some(SuspectsError::Version { found: 9, need: 1 })
    );
    let v = synth(10);
    assert!(matches!(
        Suspects::open(&v[..v.len() - 40]),
        Err(SuspectsError::Corrupt(_))
    ));
}

#[test]
fn search_speed_one_million_rows() {
    let s = Suspects::open(&synth(1_000_000)).unwrap();
    assert_eq!(s.len(), 1_000_000);
    let t = Instant::now();
    let mut found = 0;
    for i in 0..1000 {
        found += s.search(100.0 + (i * 997) as f64 * 1e-3, 5.0).len();
    }
    let per = t.elapsed().as_secs_f64() / 1000.0;
    println!("search over 10^6 rows: {:.2} us", per * 1e6);
    assert!(found >= 1000 && per < 1e-3, "{per} s per search");
    assert_eq!(s.formula(&s.search(100.0, 0.0)[0]), "C6H12O6");
}

#[test]
fn parity_with_python_reader() {
    let dir = std::env::temp_dir().join(format!("mzsus-{}", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/sospetti_riferimento.py"))
        .arg(&dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "reference script failed"
        );
        eprintln!("skipped: python3 reference not available");
        return;
    }
    let s = Suspects::open(&std::fs::read(dir.join("mini.mzsus")).unwrap()).unwrap();
    let want: Value =
        serde_json::from_slice(&std::fs::read(dir.join("casi.json")).unwrap()).unwrap();
    assert_eq!(s.len() as u64, want["rows"].as_u64().unwrap());
    assert!(s.manifest().contains("mzsus/1"));
    let mut hits = 0;
    for c in want["casi"].as_array().unwrap() {
        let got = s.search(c["mass"].as_f64().unwrap(), c["ppm"].as_f64().unwrap());
        let exp = c["hits"].as_array().unwrap();
        assert_eq!(got.len(), exp.len(), "{c}");
        for (g, e) in got.iter().zip(exp) {
            assert_eq!(
                std::str::from_utf8(&g.key).unwrap(),
                e["key"].as_str().unwrap()
            );
            assert_eq!(g.mass, e["mass"].as_f64().unwrap());
            assert_eq!(s.formula(g), e["formula"].as_str().unwrap());
            assert_eq!(g.sources as u64, e["sources"].as_u64().unwrap());
            assert_eq!(
                (g.pubmed as u64, g.patents as u64, g.reaction as u64),
                (
                    e["pubmed"].as_u64().unwrap(),
                    e["patents"].as_u64().unwrap(),
                    e["reaction"].as_u64().unwrap()
                )
            );
        }
        hits += got.len();
    }
    assert!(hits > 100);
    let _ = std::fs::remove_dir_all(dir);
}
