use super::*;
use serde_json::Value;

fn tree(files: &[&str]) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    for f in files {
        let p = d.path().join(f);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b"xy").unwrap();
    }
    d
}

fn rows(d: &Path) -> Vec<String> {
    list(d)
        .into_iter()
        .map(|i| format!("{}:{}:{}", i.path, i.kind, i.bundle.unwrap_or("-")))
        .collect()
}

#[test]
fn kinds() {
    assert_eq!(kind_of("A.MZML"), Some("data"));
    assert_eq!(kind_of("s.wiff.scan"), Some("scan"));
    assert_eq!(kind_of("m.DAM"), Some("method"));
    assert_eq!(kind_of("notes.txt"), None);
}

#[test]
fn list_is_recursive_skips_hidden_and_enters_no_bundle() {
    let d = tree(&[
        "a.mzML",
        "note.txt",
        "sub/w.wiff",
        "sub/w.wiff.scan",
        "sub/.git/x.mzML",
        "a/b/c/d/deep.mzML",
        "gc/x.D/data.ms",
        "gc/x.D/AcqData/Contents.xml",
        "gc/x.D/AcqData/hidden.mzML",
        "lc/Y.d/analysis.tdf",
        "lc/Y.d/analysis.tdf_bin",
        "wat/z.raw/_FUNC001.DAT",
        "wat/z.raw/_extern.inf",
        "notwaters/q.raw/readme.csv",
        "empty.D/x.txt",
    ]);
    assert_eq!(
        rows(d.path()),
        [
            "a.mzML:data:-",
            "empty.D:data:d",
            "gc/x.D:data:d",
            "lc/Y.d:data:d",
            "notwaters/q.raw/readme.csv:table:-",
            "sub/w.wiff:data:-",
            "sub/w.wiff.scan:scan:-",
            "wat/z.raw:data:waters-raw",
        ]
    );
    let gc = list(d.path())
        .into_iter()
        .find(|i| i.name == "x.D")
        .unwrap();
    assert_eq!((gc.members, gc.size, gc.dir.as_str()), (Some(3), 6, "gc/"));
}

#[test]
fn the_command_prints_folder_1_and_has_exit_codes() {
    let d = tree(&["a.mzML", "r.D/data.ms"]);
    let a = |v: &[&str]| crate::execute(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let p = d.path().to_str().unwrap();
    let o = a(&["folder", "list", p, "--json"]);
    assert_eq!(o.code, 0);
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["schema"], "folder/1");
    assert_eq!(v["items"][1]["bundle"], "d");
    assert_eq!(v["items"][0]["bundle"], Value::Null);
    assert_eq!(a(&["folder", "list", "/nonexistent-mzlab"]).code, 4);
    assert_eq!(a(&["folder", "list"]).code, 2);
    assert_eq!(a(&["folder", "nope", p]).code, 2);
    assert_eq!(a(&["folder", "--help"]).code, 0);
}

#[test]
fn resolve_refuses_escapes() {
    let r = Path::new("/data");
    assert_eq!(
        resolve(r, "sub/a.mzML"),
        Some(PathBuf::from("/data/sub/a.mzML"))
    );
    for bad in ["", "../x.mzML", "sub/../../x", "/etc/passwd"] {
        assert_eq!(resolve(r, bad), None, "{bad}");
    }
}
