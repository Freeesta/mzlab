//! The cache of the detections: the name of the stored `.mzfeatf` of a file (`<sha256 of the .mzidx>.<fingerprint of what detection depends on>`) and the check of
//! a stored file against it. `lib.rs` exports `file_key` and `check_stored`; `feat-run.js` keeps the files in OPFS under that name.

use super::run::recipe_for;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Value;
use mzlab_feat::series::source_sha256;

pub(crate) fn key_of(sha: &str, hash: &str) -> String {
    if sha.is_empty() {
        String::new()
    } else {
        format!("{sha}.{hash}")
    }
}

/// Name of the cached `.mzfeatf` of a file, empty when the index has no SHA-256 (nothing to key on). The recipe is the one `detect` would use (`recipe_for`), so
/// the file name, the alignment and the proofs never change the key.
pub fn file_key<R: ReadAt>(text: &str, index: &mut Index<R>) -> Result<String> {
    let r = recipe_for(text, index)?;
    Ok(key_of(&source_sha256(index), &r.file_hash()))
}

/// Checks that the bytes of a cached `.mzfeatf` are the file `key` names (SHA-256 of the source and, but for the untargeted engine, the fingerprint of the
/// recipe, both in the meta) and returns the name of the source in the meta. A file of a newer format is `error.newerFormat`; any other mismatch `error.badMzfeatf`.
pub fn check_stored(bytes: &[u8], key: &str) -> Result<String> {
    let bad = || Error::new("error.badMzfeatf");
    let (sha, hash) = key.split_once('.').ok_or_else(bad)?;
    let (meta, _) = mzlab_feat::store::read_meta_mz(bytes)?;
    let input = meta
        .get("inputs")
        .map(Value::arr)
        .and_then(<[Value]>::first)
        .ok_or_else(bad)?;
    let same_sha = input.get("sha256").and_then(Value::str) == Some(sha);
    let same_recipe = hash.starts_with('u')
        || meta
            .get("recipe_hash")
            .and_then(Value::str)
            .is_some_and(|h| h.starts_with(hash));
    if !same_sha || !same_recipe {
        return Err(bad());
    }
    Ok(input
        .get("name")
        .and_then(Value::str)
        .unwrap_or("")
        .to_string())
}
