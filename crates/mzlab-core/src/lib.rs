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
pub mod abbina;
pub mod bench_stats;
pub mod budget;
pub mod budget_auto;
pub mod centroidi;
pub mod adduct_table;
pub mod analysis;
pub mod axisroutes;
pub mod components;
pub mod comproute;
pub mod composition;
pub mod differenza;
pub mod diffroute;
pub mod elements;
pub mod entropy;
pub mod score_terms;
pub mod fileroutes;
pub mod hrroutes;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod chromeleon;
pub mod chrom_fondo;
#[cfg(test)]
mod chrom_fondo_test;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod hashio;
pub mod hrscanbin;
pub mod hrscans;
pub mod error;
pub mod provenance;
pub mod gradient;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod idxcache;
pub mod idxnuovo;
pub mod blocchi;
mod idxcheck;
pub mod idxread;
pub mod idxsearch;
pub mod idxwalk;
pub mod ionpattern;
mod isotab;
pub mod isoscore;
pub mod itemroutes;
pub mod infusione;
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
pub mod mapscan;
pub mod mapscan_read;
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
pub mod mzidx_lettore;
mod mzidx_ordine;
pub mod mzidx_spill;
pub mod msnalbero;
pub mod msngrafo;
pub mod msnroute;
pub mod msntree;
pub mod methodcheck;
pub mod methodinfo;
pub mod methodjson;
pub mod lettori;
pub mod trasporto;
pub mod lettori_centroidi;
pub mod lettori_mzml;
pub mod lettori_sessione;
pub mod conv;
pub mod metodo;
pub mod metodo_campi;
pub mod metodo_mzml;
#[cfg(feature = "thermo")]
pub mod metodo_raw;
pub mod metodo_wiff;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod mzml;
mod mzmlarray;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod mzmlhead;
pub(crate) mod unita_traccia;
pub mod mzmlsrm;
#[cfg(feature = "mappa-dad")]
mod mzml_dad;
pub mod numarray;
pub mod numpress;
pub mod attivazioni;
pub mod mzmlwrite;
#[cfg(any(not(target_family = "wasm"), feature = "valida"))]
pub mod mzmlvalida;
pub mod pace;
pub mod peakq;
pub mod peakq_auto;
pub mod peakq_lr;
#[cfg(test)]
mod peakq_reach_test;
#[cfg(test)]
mod peakq_auto_test;
pub use mzlab_picchi::peaks;
pub mod peakext;
pub mod etichette_picchi;
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
pub mod rawlettore;
#[cfg(feature = "thermo")]
mod rawlettore_reazioni;
pub mod rng;
pub mod rois;
pub mod roi_cluster;
pub mod roles;
pub mod ricerca;
pub mod jsonround;
pub mod canali;
pub mod canali_uv;
pub mod routes;
pub mod routes_xicbin;
pub mod scaninfo;
pub(crate) mod scanrow;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod scanparams;
pub mod mrmroutes;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod sniff;
pub mod sorgente;
pub mod srm;
pub mod stima;
pub mod flow;
pub mod step;
pub mod scantable;
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
pub mod statistica;
pub mod statmat;
pub mod statref;
pub mod statroute;
pub mod kendrick;
pub mod stats;
pub mod tracestat;
pub mod subformulas;
pub mod campioni;
pub mod molecules;
pub mod disegno;
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