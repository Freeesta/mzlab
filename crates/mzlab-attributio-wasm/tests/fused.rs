//! The fused `.wasm` (feature `all`): its `export_with` serves both the spreadsheet exports (the cascade of `attributio-export`) and the evidence export (the review of
//! `attributio-rev`), and an unknown kind is an error, as in the separate modules.
#![cfg(feature = "all")]
use mzlab_attributio_wasm::{export_with, Files};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/attributio")
}

fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
    for e in std::fs::read_dir(dir).expect("folder") {
        let p = e.expect("entry").path();
        if p.is_dir() {
            walk(base, &p, out);
        } else {
            let rel = p
                .strip_prefix(base)
                .expect("inside")
                .to_string_lossy()
                .replace('\\', "/");
            out.retain(|(r, _)| *r != rel);
            out.push((rel, std::fs::read(&p).expect("file")));
        }
    }
}

#[test]
fn the_fused_module_serves_the_exports_and_the_evidence() {
    let mut data = Vec::new();
    for d in ["campagna_sintetica", "campagna_banco"] {
        walk(&root().join(d), &root().join(d), &mut data);
    }
    let list: Vec<String> = data
        .iter()
        .map(|(p, d)| format!("{{\"path\":{p:?},\"size\":{}}}", d.len()))
        .collect();
    let names = format!(
        "{{\"root\":\"campagna_banco\",\"files\":[{}]}}",
        list.join(",")
    );
    let files = Files::from_json(&names).expect("names");
    let fetch = |k: usize, off: u64, len: u64| {
        Some(data[k].1[off as usize..(off + len) as usize].to_vec())
    };
    for kind in ["schymanski", "evidence"] {
        let r = export_with(&files, &fetch, &names, kind).expect(kind);
        assert!(r.contains("\"files\":["), "{kind}: {}", &r[..120.min(r.len())]);
    }
    assert!(export_with(&files, &fetch, &names, "nonsense").is_err());
}
