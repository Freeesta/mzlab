//! mzLab core: pure Rust, readable from `Read` or `&[u8]`, no native dependencies in the WASM graph.

#![forbid(unsafe_code)]
// the unit tests compare against the standard sorts
#![cfg_attr(test, allow(clippy::disallowed_methods))]
pub mod activations;
pub mod adduct_table;
#[cfg(not(feature = "estesi"))]
pub mod agilent;
#[cfg(feature = "estesi")]
#[path = "agilent_ext/mod.rs"]
pub mod agilent;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod analog;
pub mod analysis;
pub mod area_unit;
pub mod axisroutes;
pub mod baseline;
pub mod bench_stats;
pub mod blocks;
pub mod budget;
pub mod budget_auto;
pub mod calib;
pub mod centroids;
mod cespread;
pub mod chrom_background;
#[cfg(test)]
mod chrom_background_test;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod chromeleon;
pub mod coalesce;
pub mod components;
pub mod composition;
pub mod comproute;
pub mod conv;
pub mod ddafinestra;
pub mod difference;
pub mod diffroute;
pub mod elements;
pub mod entropy;
pub mod error;
pub mod fileroutes;
pub mod gradient;
pub mod gunzip;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod hashio;
pub mod hrroutes;
pub mod hrscanbin;
pub mod hrscans;
pub mod idx_newer;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod idxcache;
mod idxcheck;
pub mod idxread;
pub mod idxsearch;
pub mod idxsource;
pub mod idxwalk;
pub mod infusion;
pub mod intwin;
pub mod ionpattern;
pub mod isofine;
pub mod isoscore;
#[cfg(feature = "isotopi-tutti")]
pub mod isotable;
pub mod iteminfo;
pub mod itemroutes;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod json;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod json_members;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod jsonb;
pub mod kinetics;
pub mod labmethods;
pub mod layout;
pub mod lr;
pub mod manint;
pub mod mapcells;
pub mod mapgrid;
pub mod maproute;
pub mod mapscan;
pub mod mapscan_read;
pub mod maptiles;
pub mod mapzoom;
pub mod method;
pub mod method_fields;
pub mod method_fit;
pub mod method_mzml;
#[cfg(feature = "thermo")]
pub mod method_raw;
pub mod method_wiff;
pub mod methodcheck;
pub mod methodinfo;
pub mod methodjson;
pub mod ms2sim;
pub mod msngraph;
pub mod msnroute;
pub mod msntree;
pub mod msntree_build;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod mzidx;
mod mzidx_order;
pub mod mzidx_reader;
pub mod mzidx_spill;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod mzml;
#[cfg(feature = "mappa-dad")]
mod mzml_dad;
pub mod mzml_estimate;
#[cfg(any(not(target_family = "wasm"), feature = "valida"))]
pub mod mzml_validate;
mod mzmlarray;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod mzmlhead;
pub mod mzmlsrm;
pub mod mzmlwrite;
pub mod nominal;
pub mod nuclides;
pub mod numarray;
pub mod numfmt;
pub mod numpress;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod ole;
pub mod origin;
pub mod originroute;
pub mod pace;
pub mod pairing;
pub mod peak1d;
pub mod peakq;
pub mod peakq_auto;
#[cfg(test)]
mod peakq_auto_test;
pub mod peakq_lr;
pub mod peakq_lrauto;
#[cfg(test)]
mod peakq_reach_test;
pub mod periodic;
pub mod provenance;
mod readat;
pub mod readers;
pub mod readers_centroids;
pub mod readers_finite;
pub mod readers_mzml;
pub mod readers_session;
pub mod score_terms;
pub mod ticbg;
pub mod ticbg_route;
pub mod ticbg_synth;
#[cfg(test)]
mod ticbg_test;
pub(crate) mod trace_unit;
pub mod transport;
pub mod valley;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod wiff;
pub use mzlab_picchi::peaks;
pub mod peak_labels;
pub mod peakext;
pub use mzlab_picchi::signal;
pub use mzlab_picchi::sortc;
pub mod channels;
pub mod channels_uv;
pub mod design;
pub mod estimate;
pub mod flow;
pub mod jsonround;
pub mod kendrick;
pub mod molecules;
pub mod mrmroutes;
pub mod mzrun;
pub mod pca;
mod pressure_unit;
pub mod profspec;
pub mod project;
pub mod pysum;
pub mod qaqc;
pub mod ratio;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
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
pub mod reflist;
pub mod rng;
pub mod roi_cluster;
pub mod rois;
pub mod roles;
pub mod routes;
pub mod routes_open;
pub mod routes_xicbin;
pub mod samples;
pub mod scaninfo;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod scanparams;
pub(crate) mod scanrow;
pub mod scanstats;
pub mod scantable;
pub mod search;
pub mod similarity;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod sniff;
pub mod source;
pub mod source_ramp;
pub mod spare;
pub mod specdiff;
pub mod spechead;
pub mod specpeaks;
pub mod spectrum;
pub mod srm;
pub mod statistics;
pub mod statmat;
pub mod statref;
pub mod statroute;
pub mod stats;
pub mod step;
pub mod subformulas;
pub mod subpeaks;
pub mod subxic;
pub mod tpmine_irls;
pub mod tracestat;
pub mod upload;
pub mod window;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod xmltree;
pub mod zip;

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
