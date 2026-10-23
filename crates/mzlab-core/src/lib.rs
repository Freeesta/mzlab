//! mzLab core: pure Rust, readable from `Read` or `&[u8]`, no native dependencies in the WASM graph.

#![forbid(unsafe_code)]
// the unit tests compare against the standard sorts
#![cfg_attr(test, allow(clippy::disallowed_methods))]
pub mod abbina;
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
pub mod axisroutes;
pub mod bench_stats;
pub mod budget;
pub mod budget_auto;
pub mod campioni;
pub mod centroidi;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod chromeleon;
pub mod components;
pub mod composition;
pub mod comproute;
pub mod conv;
pub mod differenza;
pub mod diffroute;
pub mod disegno;
pub mod elements;
pub mod entropy;
pub mod error;
pub mod fileroutes;
pub mod flow;
pub mod gradient;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod hashio;
pub mod hrroutes;
pub mod hrscanbin;
pub mod hrscans;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod idxcache;
pub mod idxnuovo;
pub mod idxread;
pub mod infusione;
pub mod isoscore;
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
pub mod jsonb;
pub mod jsonround;
pub mod kinetics;
pub mod labmethods;
pub mod lettori;
pub mod lettori_centroidi;
pub mod lettori_mzml;
pub mod lr;
pub mod mapcells;
pub mod mapgrid;
pub mod maproute;
pub mod maptiles;
pub mod methodcheck;
pub mod methodinfo;
pub mod methodjson;
pub mod metodo;
pub mod metodo_campi;
pub mod metodo_mzml;
#[cfg(feature = "thermo")]
pub mod metodo_raw;
pub mod metodo_wiff;
pub mod molecules;
pub mod mrmroutes;
pub mod ms2sim;
pub mod msnalbero;
pub mod msngrafo;
pub mod msnroute;
pub mod msntree;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod mzidx;
pub mod mzidx_lettore;
mod mzidx_ordine;
pub mod mzidx_spill;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod mzml;
mod mzmlarray;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod mzmlhead;
#[cfg(not(target_family = "wasm"))]
pub mod mzmlvalida;
pub mod mzmlwrite;
pub mod numarray;
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
pub mod pca;
pub mod peakext;
pub mod peakq;
pub mod peaks;
pub mod profspec;
pub mod project;
pub mod provenance;
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
pub mod rawlettore;
#[cfg(feature = "thermo")]
mod rawlettore_reazioni;
pub mod ricerca;
pub mod rng;
pub mod roi_cluster;
pub mod rois;
pub mod roles;
pub mod routes;
pub mod routes_xicbin;
pub mod scaninfo;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod scanparams;
pub(crate) mod scanrow;
pub mod scantable;
pub mod score_terms;
pub mod signal;
pub mod similarity;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod sniff;
pub mod sorgente;
pub mod sortc;
pub mod source_ramp;
pub mod specdiff;
pub mod specpeaks;
pub mod spectrum;
pub mod srm;
pub mod statistica;
pub mod statmat;
pub mod statref;
pub mod statroute;
pub mod stats;
pub mod step;
pub mod stima;
pub mod subformulas;
pub mod subpeaks;
pub mod subxic;
pub mod tpmine_irls;
pub mod tracestat;
pub mod trasporto;
pub mod upload;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod wiff;
pub mod window;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod xmltree;

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
