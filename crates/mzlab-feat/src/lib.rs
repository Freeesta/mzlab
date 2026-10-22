//! Feature detection of mzLab: ion traces of the MS1 peaks of one file, in slabs of m/z read from the `.mzidx`.
//! `trace::detect` is the detection of mzFinder; `table::FileFeatures` is its result.
//! `family::build_families` joins isotopologues, adducts and in-source fragments of the same ion into families.
//! `pipeline::analyse` adds the quality of every peak (`quality`), the isotopologues (`isotopes`) and the MS2 with purity (`ms2`).

#![forbid(unsafe_code)]
pub mod align;
mod aligned;
mod aligned_meta;
mod aligned_ops;
#[cfg(feature = "attributio")]
pub mod attributio;
pub mod axis;
pub mod centwave;
mod channels;
#[cfg(feature = "chem")]
pub mod chem;
pub mod classi;
pub mod deconvolve;
pub mod evid;
pub mod export;
pub mod export_inclusione;
pub mod export_ms;
pub mod export_mztab;
pub mod facts;
pub mod famalign;
pub mod family;
pub mod famrecord;
pub mod drift;
pub mod filters;
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
pub mod mrm;
pub mod ms2;
pub mod ms2spec;
pub mod norm;
pub mod params;
pub mod params_drift;
pub mod params_height;
pub mod params_unit;
pub mod pipeline;
pub mod polpair;
mod post;
pub mod prof;
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
pub mod stat;
pub mod steps;
pub mod store;
mod store_read;
pub mod suspects;
mod sweep;
pub mod table;
pub mod tabcheck;
pub mod target;
pub mod tp;
pub mod trace;
#[cfg(feature = "untargeted")]
pub mod untargeted;
pub mod view;
pub mod weakfile;

/// Version of the feature table and of the rules that produce it (changes when the output of `detect` changes).
pub const FEAT_VERSION: u32 = 3;

/// Version of this crate (the one the gate of quality `cancello.json` names): it changes when the output of the pipeline may change.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
