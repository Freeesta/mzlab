//! Feature detection of mzLab: ion traces of the MS1 peaks of one file, in slabs of m/z read from the `.mzidx`.
//! `trace::detect` is the port of `TP_Mine/py/tpmine/hr/features.py::detect`; `table::FileFeatures` is its result.
//! `family::build_families` joins isotopologues, adducts and in-source fragments of the same ion into families.
//! `pipeline::analyse` adds the quality of every peak (`quality`), the isotopologues (`isotopes`) and the MS2 with purity (`ms2`).

#![forbid(unsafe_code)]
pub mod align;
mod aligned;
mod aligned_meta;
mod aligned_ops;
mod channels;
pub mod evid;
pub mod famalign;
pub mod facts;
pub mod family;
pub mod filters;
pub mod gapfill;
pub mod hypo;
pub mod isotopes;
pub mod kinetic;
pub mod ms2;
pub mod norm;
pub mod params;
pub mod pipeline;
mod post;
pub mod qc;
pub mod quality;
pub mod recipe;
pub mod resolve;
pub mod report;
pub mod rowident;
pub mod rowmeta;
pub mod rtmap;
pub mod samples;
pub mod series;
pub mod slabs;
pub mod sortc;
pub mod store;
mod store_read;
pub mod suspects;
mod sweep;
pub mod table;
pub mod target;
pub mod trace;
pub mod view;

/// Version of the feature table and of the rules that produce it (changes when the output of `detect` changes).
pub const FEAT_VERSION: u32 = 3;

/// Version of this crate (the one the gate of quality `cancello.json` names): it changes when the output of the pipeline may change.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
