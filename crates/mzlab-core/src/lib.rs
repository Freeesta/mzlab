//! mzLab core: pure Rust, readable from `Read` or `&[u8]`, no native dependencies in the WASM graph.

#![forbid(unsafe_code)]
pub mod agilent;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod analog;
pub mod budget;
pub mod centroidi;
pub mod analysis;
pub mod axisroutes;
pub mod components;
pub mod comproute;
pub mod composition;
pub mod differenza;
pub mod diffroute;
pub mod elements;
pub mod entropy;
pub mod fileroutes;
pub mod hrroutes;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod chromeleon;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod hrscanbin;
pub mod hrscans;
pub mod error;
pub mod gradient;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod idxcache;
pub mod idxread;
pub mod isoscore;
pub mod itemroutes;
pub mod iteminfo;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod json;
pub mod labmethods;
pub mod kinetics;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod jsonb;
pub mod lr;
pub mod mapcells;
pub mod mapgrid;
pub mod maproute;
pub mod ms2sim;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod ole;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod wiff;
pub mod origin;
pub mod originroute;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod mzidx;
pub mod mzidx_lettore;
pub mod msnroute;
pub mod msntree;
pub mod methodcheck;
pub mod methodinfo;
pub mod methodjson;
pub mod lettori;
pub mod lettori_mzml;
pub mod metodo;
pub mod metodo_raw;
pub mod metodo_wiff;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod mzml;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod mzmlhead;
pub mod numarray;
pub mod mzmlwrite;
pub mod peakq;
pub mod peaks;
pub mod peakext;
pub mod signal;
pub mod sortc;
pub mod profspec;
pub mod project;
pub mod pysum;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod raw;
pub mod rawfilter;
pub mod rawlettore;
pub mod rng;
pub mod rois;
pub mod roles;
pub mod routes;
pub mod scaninfo;
pub(crate) mod scanrow;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod scanparams;
pub mod mrmroutes;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod sniff;
pub mod sorgente;
pub mod scantable;
pub mod specdiff;
pub mod specpeaks;
pub mod spectrum;
pub mod subxic;
pub mod upload;
pub mod window;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod xmltree;
pub mod similarity;
pub mod ratio;
pub mod pca;
pub mod qaqc;
pub mod statistica;
pub mod statmat;
pub mod statref;
pub mod statroute;
pub mod stats;
pub mod tracestat;
pub mod subformulas;

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