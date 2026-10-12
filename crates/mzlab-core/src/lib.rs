//! mzLab core: pure Rust, readable from `Read` or `&[u8]`, no native dependencies in the WASM graph.

pub mod analysis;
pub mod components;
pub mod composition;
pub mod comproute;
pub mod elements;
pub mod entropy;
pub mod error;
pub mod fileroutes;
pub mod hrroutes;
pub mod hrscanbin;
pub mod hrscans;
pub mod idxread;
pub mod isoscore;
pub mod iteminfo;
pub mod json;
pub mod jsonb;
pub mod kinetics;
pub mod lr;
pub mod mapcells;
pub mod mapgrid;
pub mod maproute;
pub mod ms2sim;
pub mod msntree;
pub mod mzidx;
pub mod mzml;
pub mod mzmlhead;
pub mod origin;
pub mod originroute;
pub mod pca;
pub mod peakext;
pub mod peakq;
pub mod peaks;
pub mod peaktable;
pub mod profspec;
pub mod project;
pub mod pysum;
pub mod ratio;
pub mod raw;
pub mod rng;
pub mod rois;
pub mod roles;
pub mod routes;
pub mod scaninfo;
pub mod scantable;
pub mod signal;
pub mod similarity;
pub mod sniff;
pub mod specdiff;
pub mod specpeaks;
pub mod spectrum;
pub mod statistica;
pub mod statmat;
pub mod statref;
pub mod statroute;
pub mod stats;
pub mod subformulas;
pub mod subxic;
pub mod tracestat;
pub mod upload;
pub mod window;

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
