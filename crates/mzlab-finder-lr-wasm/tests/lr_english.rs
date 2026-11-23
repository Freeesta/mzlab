//! The JSON the low-resolution Dig gives an agent or the page (summary, the detail of every row, the network of mass differences) is English:
//! no key and no value holds an Italian word of `tools/italian_words.tsv` (the labels `parent`/`strong`/`possible`/`weak` are stable English values).

use mzlab_feat::legacy::lr::{detail::detail, summary::summary};
use serde_json::Value;

mod common;
use common::*;

#[test]
fn the_dig_json_has_no_italian() {
    let (files, idxs) = series();
    let d = dig(&files, &idxs);
    let s: Value = serde_json::from_str(&summary(&d, r#"{"total": 1}"#)).unwrap();
    let mut out = Vec::new();
    english(&s, "summary", &mut out);
    for r in s["rows"].as_array().unwrap() {
        let id = r["id"].as_i64().unwrap();
        let det: Value =
            serde_json::from_str(&detail(&d, id, |k| Ok(open(&idxs[k]))).unwrap()).unwrap();
        english(&det, &format!("detail[{id}]"), &mut out);
    }
    let out = kinds(out);
    assert!(
        out.is_empty(),
        "Italian in the Dig JSON:\n{}",
        out.join("\n")
    );
}
