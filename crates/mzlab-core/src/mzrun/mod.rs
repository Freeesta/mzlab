//! The `mzrun/1` record (`docs/agenti/mzrun.md`, schema `docs/schemi/mzrun-1.json`): `build` writes one from a plan, its `prov/1` and the roles of the
//! files, `verify` checks the fingerprints of a written one, `migrate` brings a file of older step versions to the current ones. Used by `mzlab-cli flow record|verify`; text is the canonical JSON of `flow::value_text`.

mod build;
mod migrate;
mod verify;

pub use build::{build, Parts};
pub use migrate::migrate;
pub use verify::{verify, Verdict};

use crate::budget::sha256_hex;
use crate::json::Value;

/// SHA-256 of the lines `name<TAB>sha256\n` sorted by name: the only thing two records are compared by (`ms`, `heap_peak_bytes`, `cached` and the agent
/// are not in it).
pub fn results_digest(entities: &[(String, String)]) -> String {
    let mut v: Vec<&(String, String)> = entities.iter().collect();
    crate::sortc::slice_by(&mut v, |a, b| a.0.cmp(&b.0));
    let mut text = String::new();
    for (name, sha) in v {
        text.push_str(name);
        text.push('\t');
        text.push_str(sha);
        text.push('\n');
    }
    sha256_hex(text.as_bytes())
}

/// The fingerprint of a record: SHA-256 of the canonical text of the object without `self_sha256`.
pub(crate) fn self_sha256(entries: &[(String, Value)]) -> String {
    let rest: Vec<(String, Value)> = entries
        .iter()
        .filter(|(k, _)| k != "self_sha256")
        .cloned()
        .collect();
    sha256_hex(crate::flow::value_text(&Value::Obj(rest)).as_bytes())
}

#[cfg(test)]
mod tests;
