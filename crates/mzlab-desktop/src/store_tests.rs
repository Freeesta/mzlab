//! Tests of the store of the app for native samples that share files.

use super::*;

#[test]
fn removing_one_sample_keeps_the_files_of_its_siblings() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::default();
    s.set_dir(d.path().join("data"));
    let own = s.own_dir().unwrap();
    let (wiff, scan) = (own.join("x.wiff"), own.join("x.wiff.scan"));
    std::fs::write(&wiff, b"w").unwrap();
    std::fs::write(&scan, b"s").unwrap();
    let nat = |c: usize| Nat {
        ds: "d".into(),
        c,
        id: 0,
        centroid: false,
        members: vec![("x.wiff".into(), wiff.clone()), ("x.wiff.scan".into(), scan.clone())],
    };
    s.put_nat("x#1", nat(0)).unwrap();
    s.put_nat("x#2", nat(1)).unwrap();
    s.remove("x#1");
    assert!(wiff.exists() && scan.exists(), "the sibling still uses the members");
    assert_eq!(s.list().len(), 1, "the sibling survives");
    s.remove("x#2");
    assert!(!wiff.exists() && !scan.exists(), "the last sample takes the files with it");
}
