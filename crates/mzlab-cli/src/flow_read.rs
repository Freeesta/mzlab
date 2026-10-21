//! Files read whole for the executors that take names and bytes, never paths (`flow.rs`).

use std::path::PathBuf;

use mzlab_core::error::Result;

use crate::flow::io;
use crate::flow_match::Named;

/// A file read whole: its name and its bytes (the executors take names and bytes, never paths).
pub(crate) type Loaded = (String, Vec<u8>);

pub(crate) fn load(path: PathBuf) -> Result<Loaded> {
    let name = path
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    Ok((name, std::fs::read(&path).map_err(io)?))
}

pub(crate) fn named(v: &[Loaded]) -> Vec<Named<'_>> {
    v.iter().map(|f| (f.0.as_str(), f.1.as_slice())).collect()
}

pub(crate) fn blobs(v: &[Loaded]) -> Vec<&[u8]> {
    v.iter().map(|f| f.1.as_slice()).collect()
}
