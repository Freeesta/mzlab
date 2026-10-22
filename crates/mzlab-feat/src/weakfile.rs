//! The weak candidates of one file stored beside its `.mzfeatf`: the traces `series::weak_candidates` finds from `WEAK_RATIO` of the minimum height up.
//! The step that detects the file makes them (in parallel with the other files); the alignment and the gap filling read them instead of detecting the
//! file again. The bytes carry the fingerprint of what they depend on (`Recipe::weak_hash`), so a recipe that differs reads as "not stored".

use mzlab_core::error::Result;

use crate::spill::{decode_range, encode};
use crate::table::FileFeatures;

const MAGIC: &[u8; 8] = b"MZWEAK1\n";
const KEY: usize = 32;

/// The stored form of `candidates` made with the recipe whose `Recipe::weak_hash` is `key` (32 hex digits).
pub fn to_bytes(candidates: &FileFeatures, key: &str) -> Vec<u8> {
    let body = encode(candidates);
    let mut b = Vec::with_capacity(MAGIC.len() + KEY + body.len());
    b.extend_from_slice(MAGIC);
    b.extend_from_slice(key.as_bytes());
    b.extend_from_slice(&body);
    b
}

/// The candidates of stored `bytes` when they were made with the recipe of `key`; `None` for another recipe or for bytes that are not a stored table
/// (the caller detects the file again).
pub fn from_bytes(bytes: &[u8], key: &str) -> Result<Option<FileFeatures>> {
    let head = MAGIC.len() + KEY;
    if bytes.len() < head
        || &bytes[..MAGIC.len()] != MAGIC
        || &bytes[MAGIC.len()..head] != key.as_bytes()
    {
        return Ok(None);
    }
    Ok(decode_range(&bytes[head..], f64::NEG_INFINITY, f64::INFINITY).ok())
}
