//! `cache::file_key` and `cache::check_stored` (behind the exports `file_key` and `check_stored`): the key of the cached `.mzfeatf` moves with the detection
//! and with nothing else. Synthetic series of `tools/feat_sintetici.py`; skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).
#![cfg(feature = "detect")]

use std::path::Path;

use mzlab_core::idxread::Index;
use mzlab_feat_wasm::{cache, run};

fn make() -> Option<Vec<u8>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let out = std::env::temp_dir().join(format!("mzlab_file_key_{}_{n}", std::process::id()));
    let ok = std::process::Command::new("python3")
        .arg(root.join("tools/feat_sintetici.py"))
        .arg(&out)
        .args(["--solo", "degdma"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        return None;
    }
    let bytes = std::fs::read(out.join("degdma.mzidx")).unwrap();
    let _ = std::fs::remove_dir_all(&out);
    Some(bytes)
}

fn key(bytes: &[u8], recipe: &str) -> String {
    let mut ix = Index::open(bytes.to_vec(), bytes.len() as u64).unwrap();
    cache::file_key(recipe, &mut ix).unwrap()
}

#[test]
fn the_key_follows_the_detection_only() {
    let Some(bytes) = make() else { return };
    let base = key(&bytes, "");
    let (sha, hash) = base.split_once('.').expect("sha256.fingerprint");
    assert_eq!((sha.len(), hash.len()), (64, 16));
    assert_eq!(base, key(&bytes, ""), "same file, same recipe");
    for same in [
        r#"{"rt_tol":0.3}"#,
        r#"{"ppm":8}"#,
        r#"{"fill":false}"#,
        r#"{"mode":"ion"}"#,
    ] {
        assert_eq!(
            base,
            key(&bytes, same),
            "{same} is not read by the detection"
        );
    }
    for other in [
        r#"{"min_height":12345}"#,
        r#"{"min_points":9}"#,
        r#"{"floor":777}"#,
        r#"{"polarity":-1}"#,
    ] {
        assert_ne!(base, key(&bytes, other), "{other} changes the detection");
    }
    let layers = |h: f64| {
        format!(r#"{{"layers":[{{"step":"features.detect","params":{{"min_height":{h}}}}}]}}"#)
    };
    assert_ne!(key(&bytes, &layers(1e4)), key(&bytes, &layers(2e4)));
    let a = r#"{"layers":[{"step":"features.align","params":{"rt_gap":0.2}}]}"#;
    let b = r#"{"layers":[{"step":"features.align","params":{"rt_gap":0.5}}]}"#;
    assert_eq!(key(&bytes, a), key(&bytes, b), "a step of the alignment");
}

#[test]
fn a_renamed_file_keeps_its_key_and_reports_the_old_name() {
    let Some(bytes) = make() else { return };
    let mut ix = Index::open(bytes.clone(), bytes.len() as u64).unwrap();
    let r = run::recipe_for("", &mut ix).unwrap();
    let k = key(&bytes, "");
    for name in ["a.mzidx", "renamed.mzidx"] {
        let stored = run::detect(&mut ix, name, &r).unwrap();
        // the key is the same, so the page compares the name in the meta with the current one before it uses the entry
        assert_eq!(cache::check_stored(&stored, &k).unwrap(), name);
    }
}

#[test]
fn a_stored_file_is_checked_against_its_key() {
    let Some(bytes) = make() else { return };
    let mut ix = Index::open(bytes.clone(), bytes.len() as u64).unwrap();
    let r = run::recipe_for("", &mut ix).unwrap();
    let stored = run::detect(&mut ix, "a.mzidx", &r).unwrap();
    let k = key(&bytes, "");
    assert_eq!(cache::check_stored(&stored, &k).unwrap(), "a.mzidx");
    let other = key(&bytes, r#"{"min_height":12345}"#);
    assert_eq!(
        cache::check_stored(&stored, &other).unwrap_err().key,
        "error.badMzfeatf"
    );
    let wrong_sha = format!("{}.{}", "0".repeat(64), k.split_once('.').unwrap().1);
    assert_eq!(
        cache::check_stored(&stored, &wrong_sha).unwrap_err().key,
        "error.badMzfeatf"
    );
    assert!(cache::check_stored(&stored[..40], &k).is_err(), "truncated");
    // a newer container is reported as such (and the page never deletes it)
    let mut newer = stored.clone();
    newer[..8].copy_from_slice(b"MZFEAT09");
    assert_eq!(
        cache::check_stored(&newer, &k).unwrap_err().key,
        "error.newerFormat"
    );
}
