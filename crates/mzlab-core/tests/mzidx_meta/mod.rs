//! Shared by the tests that compare the meta of an mzML index with the one of a vendor-file index or a reader table: the object without some keys,
//! and without the chromatogram list (`run.chromatograms`), which only an mzML index carries.
#![allow(dead_code)]

use mzlab_core::json::Value;

/// `run` without its `chromatograms` key.
pub fn run_without_list(run: &Value) -> Value {
    match run {
        Value::Obj(r) => Value::Obj(
            r.iter()
                .filter(|(n, _)| n != "chromatograms")
                .cloned()
                .collect(),
        ),
        v => v.clone(),
    }
}

/// The object without the keys `skip`, and its `run` without the list.
pub fn without(v: &Value, skip: &[&str]) -> Vec<(String, Value)> {
    match v {
        Value::Obj(o) => o
            .iter()
            .filter(|(k, _)| !skip.contains(&k.as_str()))
            .map(|(k, v)| match k.as_str() {
                "run" => (k.clone(), run_without_list(v)),
                _ => (k.clone(), v.clone()),
            })
            .collect(),
        _ => panic!("not an object"),
    }
}
