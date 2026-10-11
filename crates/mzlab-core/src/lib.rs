//! mzLab core: pure Rust, readable from `Read` or `&[u8]`, no native dependencies in the WASM graph.

pub mod analysis;
pub mod entropy;
pub mod error;
pub mod idxread;
pub mod json;
pub mod mzidx;
pub mod mzml;
pub mod peakq;
pub mod peaks;
pub mod project;
pub mod raw;
pub mod rois;
pub mod routes;
pub mod scaninfo;
pub mod scantable;
pub mod specpeaks;

pub use error::{Error, Result};

/// Version of the core crate.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!version().is_empty());
    }
}
