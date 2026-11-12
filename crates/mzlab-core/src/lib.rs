//! mzLab core: pure Rust, readable from `Read` or `&[u8]`, no native dependencies in the WASM graph.

#![forbid(unsafe_code)]
// the unit tests compare against the standard sorts
#![cfg_attr(test, allow(clippy::disallowed_methods))]
#[cfg(not(feature = "estesi"))]
pub mod agilent;
#[cfg(feature = "estesi")]
#[path = "agilent_ext/mod.rs"]
pub mod agilent;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod analog;
pub mod pairing;
pub mod bench_stats;
pub mod budget;
pub mod budget_auto;
pub mod centroids;
pub mod adduct_table;
pub mod analysis;
pub mod axisroutes;
pub mod components;
pub mod comproute;
pub mod composition;
pub mod ddafinestra;
pub mod difference;
pub mod diffroute;
pub mod elements;
pub mod entropy;
pub mod score_terms;
pub mod fileroutes;
pub mod hrroutes;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod chromeleon;
pub mod chrom_background;
#[cfg(test)]
mod chrom_background_test;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod hashio;
pub mod hrscanbin;
pub mod hrscans;
pub mod error;
pub mod provenance;
pub mod gradient;
pub mod gunzip;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod idxcache;
pub mod idx_newer;
pub mod blocks;
mod idxcheck;
pub mod idxread;
mod readat;
pub mod idxsearch;
pub mod idxsource;
pub mod idxwalk;
pub mod ionpattern;
pub mod isofine;
pub mod isoscore;
#[cfg(feature = "isotopi-tutti")]
pub mod isotable;
pub mod nuclides;
pub mod itemroutes;
pub mod infusion;
pub mod iteminfo;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod json;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod json_members;
pub mod labmethods;
pub mod kinetics;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod jsonb;
pub mod lr;
pub mod mapcells;
pub mod mapgrid;
pub mod mapscan;
pub mod mapscan_read;
pub mod mapzoom;
pub mod maproute;
pub mod maptiles;
pub mod ms2sim;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod ole;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod wiff;
pub mod origin;
pub mod originroute;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod mzidx;
pub mod mzidx_reader;
mod mzidx_order;
pub mod mzidx_spill;
pub mod coalesce;
pub mod msntree_build;
pub mod msngraph;
pub mod msnroute;
pub mod msntree;
pub mod methodcheck;
pub mod methodinfo;
pub mod methodjson;
pub mod readers;
pub mod transport;
pub mod readers_centroids;
pub mod readers_finite;
pub mod readers_mzml;
pub mod readers_session;
pub mod conv;
pub mod method;
pub mod method_fit;
pub mod method_fields;
pub mod method_mzml;
#[cfg(feature = "thermo")]
pub mod method_raw;
pub mod method_wiff;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod mzml;
pub mod mzml_estimate;
mod mzmlarray;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod mzmlhead;
pub(crate) mod trace_unit;
pub mod mzmlsrm;
pub mod manint;
#[cfg(feature = "mappa-dad")]
mod mzml_dad;
pub mod numarray;
pub mod numfmt;
pub mod numpress;
pub mod activations;
pub mod mzmlwrite;
#[cfg(any(not(target_family = "wasm"), feature = "valida"))]
pub mod mzml_validate;
pub mod pace;
pub mod peak1d;
pub mod baseline;
pub mod intwin;
pub mod peakq;
pub mod peakq_auto;
pub mod peakq_lr;
#[cfg(test)]
mod peakq_reach_test;
#[cfg(test)]
mod peakq_auto_test;
pub use mzlab_picchi::peaks;
pub mod peakext;
pub mod peak_labels;
pub use mzlab_picchi::signal;
pub use mzlab_picchi::sortc;
pub mod spare;
pub mod source_ramp;
pub mod profspec;
pub mod project;
pub mod pysum;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
// raw.rs belongs to the formats work: its one std sort stays until that lands (then sortc::slice_ord)
#[allow(clippy::disallowed_methods)]
#[cfg(feature = "thermo")]
pub mod raw;
#[cfg(feature = "thermo")]
pub mod rawfilter;
#[cfg(feature = "thermo")]
pub mod rawreader;
#[cfg(feature = "thermo")]
mod rawreader_reactions;
mod pressure_unit;
pub mod rng;
pub mod rois;
pub mod roi_cluster;
pub mod roles;
pub mod search;
pub mod jsonround;
pub mod channels;
pub mod channels_uv;
pub mod routes;
pub mod routes_xicbin;
pub mod scaninfo;
pub mod scanstats;
pub(crate) mod scanrow;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod scanparams;
pub mod mrmroutes;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod sniff;
pub mod source;
pub mod srm;
pub mod estimate;
pub mod flow;
pub mod mzrun;
pub mod step;
pub mod scantable;
pub mod spechead;
pub mod specdiff;
pub mod specpeaks;
pub mod spectrum;
pub mod subxic;
pub mod subpeaks;
pub mod upload;
pub mod window;
pub mod zip;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod xmltree;
pub mod similarity;
pub mod ratio;
pub mod pca;
pub mod qaqc;
pub mod statistics;
pub mod statmat;
pub mod statref;
pub mod statroute;
pub mod kendrick;
pub mod stats;
pub mod tracestat;
pub mod subformulas;
pub mod samples;
pub mod molecules;
pub mod drawing;
pub mod tpmine_irls;

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