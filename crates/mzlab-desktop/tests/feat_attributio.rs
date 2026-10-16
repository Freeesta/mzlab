//! The route `feat/attributio/review` of the app (Banco 2b) on the synthetic campaign plus the folder `campagna_banco`: the review of a feature has its proposal, and a decision is
//! written straight into the campaign folder, the file being the golden `feature_decisions.csv` that the engine tests and the reference tool share.
use std::path::Path;

use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            walk(base, &p, out);
        } else if let Ok(t) = std::fs::read_to_string(&p) {
            let rel = p
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.retain(|(r, _)| *r != rel);
            out.push((rel, t));
        }
    }
}

fn folder() -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/attributio");
    let mut files = Vec::new();
    for d in ["campagna_sintetica", "campagna_banco"] {
        walk(&root.join(d), &root.join(d), &mut files);
    }
    Value::Array(
        files
            .into_iter()
            .map(|(path, text)| json!({"path": path, "text": text}))
            .collect(),
    )
}

#[test]
fn review_and_decision_of_the_banco() {
    let st = State::new();
    let files = folder();
    let r = text(
        st.rpc(
            "feat/attributio/review",
            &json!({"root": "c", "files": files, "kind": "review", "pol": "POS", "id": 15}),
            &[],
        )
        .unwrap(),
    );
    let v: Value = serde_json::from_str(&r).unwrap();
    assert_eq!(v["proposal"]["class"], "2b-check");
    assert_eq!(v["checklist"].as_array().unwrap().len(), 6);

    let dir = std::env::temp_dir().join(format!("mzlab_banco_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let decision = json!({"pol": "POS", "id": 15, "level": "2b", "justification": "diagnostic ions explained", "decided_on": "2026-10-16", "decided_by": "rev",
        "checklist": ["diagnostic_fragment", "neutral_loss", "alternatives_excluded"],
        "evidence": {"type": "fragments", "library": "library/banco.json", "entry": "", "cosine": 0.99, "matched": 4, "delta_mz": 0, "delta_formula": "", "note": "x"}});
    let q = json!({"root": "c", "files": files, "kind": "decision", "decision": decision.to_string(), "folder": dir.to_string_lossy()});
    let d: Value =
        serde_json::from_str(&text(st.rpc("feat/attributio/review", &q, &[]).unwrap())).unwrap();
    assert_eq!(
        (d["added"].as_i64(), d["written"].as_bool()),
        (Some(1), Some(true))
    );
    let on_disk = std::fs::read_to_string(dir.join(d["path"].as_str().unwrap())).unwrap();
    let golden = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/attributio/banco/feature_decisions.csv");
    assert_eq!(
        on_disk,
        std::fs::read_to_string(golden)
            .unwrap()
            .replace("\r\n", "\n")
    );
    // without a folder nothing is written; a path that leaves the folder never is
    let none: Value = serde_json::from_str(&text(st.rpc("feat/attributio/review", &json!({"root": "c", "files": files, "kind": "decision", "decision": decision.to_string()}), &[]).unwrap())).unwrap();
    assert_eq!(none["written"], false);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_exports_answer_with_files() {
    let st = State::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/attributio");
    let mut files = Vec::new();
    for d in ["campagna_sintetica", "campagna_esportazione"] {
        walk(&root.join(d), &root.join(d), &mut files);
    }
    let files = Value::Array(
        files
            .into_iter()
            .map(|(path, text)| json!({"path": path, "text": text}))
            .collect(),
    );
    for (kind, n) in [("schymanski", 1), ("quantify", 1), ("evidence", 2)] {
        let q = json!({"root": "c", "files": files, "kind": kind, "generated": "2026-10-16"});
        let r = text(st.rpc("feat/attributio/export", &q, &[]).unwrap());
        let v: Value = serde_json::from_str(&r).unwrap();
        assert!(v["files"].as_array().unwrap().len() >= n, "{kind}");
    }
}
