//! The bridge of the Banco 2b on the synthetic campaign plus `campagna_banco`: the review of a feature and the saving of a decision through `review_with`, `decision_with`,
//! `text_to` (pieces at offsets) give the same text as the engine's, and the decisions file is the golden one.
use mzlab_attributio_wasm::{decision_with, review_with, saved_summary, text_to, Files};
use std::path::{Path, PathBuf};

const CLASSES: &str = include_str!("../../mzlab-feat/data/classi-1.json");

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

fn folder() -> (Files, Vec<(String, Vec<u8>)>) {
    let mut v = Vec::new();
    for d in ["campagna_sintetica", "campagna_banco"] {
        walk(&root().join(d), &root().join(d), &mut v);
    }
    let list: Vec<String> = v
        .iter()
        .map(|(p, d)| format!("{{\"path\":{p:?},\"size\":{}}}", d.len()))
        .collect();
    let names = format!(
        "{{\"root\":\"campagna_banco\",\"files\":[{}]}}",
        list.join(",")
    );
    (Files::from_json(&names).expect("names"), v)
}

#[test]
fn the_bridge_reviews_a_feature_and_saves_a_decision() {
    let (files, data) = folder();
    let fetch =
        |k: usize, off: u64, len: u64| Some(data[k].1[off as usize..(off + len) as usize].to_vec());
    let r = review_with(&files, &fetch, "POS", 15, CLASSES).expect("review");
    if let Ok(p) = std::env::var("DUMP_REVIEW") {
        std::fs::write(p, &r).expect("dump");
    }
    assert!(
        r.contains("\"2b-check\"") && r.contains("\"checklist\""),
        "{}",
        &r[..200.min(r.len())]
    );
    // The classes of the rules table ride along with the review: the contract is present, never an identification.
    assert!(
        r.contains("\"classes\":null") || r.contains("\"format\":\"classi/1\""),
        "classes key missing"
    );
    assert!(review_with(&files, &fetch, "POS", 999, "").is_err());

    let d = r#"{"pol":"POS","id":15,"level":"2b","justification":"diagnostic ions explained","decided_on":"2026-10-16","decided_by":"rev",
        "checklist":["diagnostic_fragment","neutral_loss","alternatives_excluded"],
        "evidence":{"type":"fragments","library":"library/banco.json","entry":"","cosine":0.99,"matched":4,"delta_mz":0,"delta_formula":"","note":"x"}}"#;
    let saved = decision_with(&files, &fetch, d).expect("saved");
    assert_eq!(
        saved_summary(&saved),
        format!(
            "{{\"path\":{:?},\"added\":1,\"replaced\":0,\"unchanged\":0}}",
            saved.path
        )
    );
    let golden =
        std::fs::read_to_string(root().join("../../golden/attributio/banco/feature_decisions.csv"))
            .expect("golden")
            .replace("\r\n", "\n");
    let mut out: Vec<u8> = Vec::new();
    let mut next = 0u64;
    assert!(text_to(&saved.text, &mut |off, b| {
        assert_eq!(off, next);
        next += b.len() as u64;
        out.extend_from_slice(b);
        true
    }));
    assert_eq!(String::from_utf8(out).expect("utf-8"), golden);
    assert!(
        decision_with(&files, &fetch, &d.replace("diagnostic ions explained", "")).is_err(),
        "a decision against the proposal needs a note"
    );
}

#[test]
fn text_to_cuts_on_character_boundaries() {
    let s = "é".repeat(40000);
    let mut parts = 0;
    assert!(text_to(&s, &mut |_, b| {
        assert!(std::str::from_utf8(b).is_ok());
        parts += 1;
        true
    }));
    assert_eq!(parts, 2);
}
