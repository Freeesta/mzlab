//! Feature detection of mzLab: ion traces of the MS1 peaks of one file, in slabs of m/z read from the `.mzidx`.
//! `trace::detect` is the detection of mzFinder; `table::FileFeatures` is its result.
//! `family::build_families` joins isotopologues, adducts and in-source fragments of the same ion into families.
//! `pipeline::analyse` adds the quality of every peak (`quality`), the isotopologues (`isotopes`) and the MS2 with purity (`ms2`).

#![forbid(unsafe_code)]
pub mod adducts;
pub mod align;
mod aligned;
mod aligned_meta;
mod aligned_ops;
#[cfg(feature = "attributio")]
pub mod attributio;
pub mod axis;
pub mod broad;
pub mod broad_blank;
mod broad_front;
pub mod broad_gt;
pub mod centwave;
mod channels;
#[cfg(feature = "chem")]
pub mod chem;
pub mod classi;
pub mod deconvolve;
pub mod dia;
pub mod drift;
pub mod evid;
pub mod export;
pub mod export_inclusione;
pub mod export_ms;
pub mod export_mztab;
pub mod facts;
pub mod famalign;
pub mod family;
pub mod famrecord;
pub mod filters;
pub mod filters_blank;
pub mod filters_nts;
#[cfg(feature = "untargeted")]
pub mod flow_formula_ms1;
#[cfg(feature = "flow_formula")]
pub mod flow_formula_ms2;
#[cfg(feature = "flow_formula")]
pub mod flow_insilico;
pub mod flow_level;
pub mod flow_ms2;
pub mod flow_screen_suspects;
pub mod gapfill;
pub mod gruppi;
pub mod hypo;
pub mod import;
#[cfg(feature = "interp")]
pub mod interp;
pub mod isotopes;
pub mod kinetic;
pub mod legacy;
pub mod liste;
mod lrnoise;
pub mod massacc;
pub mod mrm;
pub mod ms2;
pub mod ms2spec;
pub mod nature;
pub mod norm;
pub mod params;
pub mod params_drift;
pub mod params_height;
pub mod params_unit;
pub mod pipeline;
pub mod polpair;
mod post;
pub mod prof;
mod prog;
pub mod qc;
pub mod quality;
pub mod recipe;
mod recipe_lr;
pub mod report;
pub mod resolve;
pub mod rete;
pub mod riga;
pub mod rowident;
pub mod rowmeta;
pub mod rtmap;
pub mod rtwarp;
pub mod samples;
pub mod series;
pub mod slabs;
pub mod spill;
pub use mzlab_core::sortc;
mod chunked;
pub mod plausible;
pub mod stat;
pub mod steps;
pub mod store;
mod store_read;
pub mod suspects;
pub mod suspects_listings;
mod sweep;
pub mod tabcheck;
pub mod table;
pub mod target;
pub mod tp;
pub mod trace;
mod trace_text;
#[cfg(feature = "untargeted")]
pub mod untargeted;
pub mod uv_bands;
pub mod view;
pub mod weakfile;

/// Version of the feature table and of the rules that produce it (changes when the output of `detect` changes).
pub const FEAT_VERSION: u32 = 3;

/// Version of this crate (the one the gate of quality `cancello.json` names): it changes when the output of the pipeline may change.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
