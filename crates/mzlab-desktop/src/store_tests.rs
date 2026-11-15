//! Tests of the store of the app for native samples that share files, and of the reset «Files» (`clear_files`).

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
        members: vec![
            ("x.wiff".into(), wiff.clone()),
            ("x.wiff.scan".into(), scan.clone()),
        ],
    };
    s.put_nat("x#1", nat(0)).unwrap();
    s.put_nat("x#2", nat(1)).unwrap();
    s.remove("x#1");
    assert!(
        wiff.exists() && scan.exists(),
        "the sibling still uses the members"
    );
    assert_eq!(s.list().len(), 1, "the sibling survives");
    s.remove("x#2");
    assert!(
        !wiff.exists() && !scan.exists(),
        "the last sample takes the files with it"
    );
}

#[test]
fn the_reset_files_keeps_the_drawing_and_forgets_the_files() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("a.mzML");
    std::fs::write(&f, b"<mzML/>").unwrap();
    let mut s = Store::default();
    s.set_dir(d.path().join("data"));
    s.put("a.mzML", f.clone()).unwrap();
    s.kv_set("notebook", r#"{"ket":"K","session":["a.mzML"],"ui":{},"note":"n"}"#);
    s.kv_set("cartella.puntata", "x");
    s.kv_set("schema", "3");
    s.clear_files(&["cartella.puntata"], &["session", "ui", "sampleCols", "sampleEx"]);
    let mut t = Store::default();
    t.set_dir(d.path().join("data"));
    assert!(t.list().is_empty(), "no file in the list");
    let nb: Value = serde_json::from_str(&t.kv_get("notebook").unwrap()).unwrap();
    assert_eq!((nb["ket"].as_str(), nb["note"].as_str()), (Some("K"), Some("n")), "{nb}");
    assert!(nb.get("session").is_none() && nb.get("ui").is_none() && nb["_t"].as_u64().unwrap() > 0, "{nb}");
    assert_eq!((t.kv_get("cartella.puntata"), t.kv_get("schema").as_deref()), (None, Some("3")));
    assert!(f.exists(), "the file of the student is never touched");
}
