//! Feature detection of mzLab: ion traces of the MS1 peaks of one file, in slabs of m/z read from the `.mzidx`.
//! `trace::detect` is the port of `TP_Mine/py/tpmine/hr/features.py::detect`; `table::FileFeatures` is its result.
//! `family::build_families` joins isotopologues, adducts and in-source fragments of the same ion into families.
//! `pipeline::analyse` adds the quality of every peak (`quality`), the isotopologues (`isotopes`) and the MS2 with purity (`ms2`).

pub mod align;
mod aligned;
mod aligned_ops;
mod channels;
pub mod evid;
pub mod famalign;
pub mod family;
pub mod gapfill;
pub mod hypo;
pub mod isotopes;
pub mod kinetic;
pub mod ms2;
pub mod pipeline;
mod post;
pub mod quality;
pub mod recipe;
pub mod resolve;
pub mod rtmap;
pub mod series;
pub mod sortc;
pub mod store;
pub mod suspects;
mod sweep;
pub mod table;
pub mod target;
pub mod trace;

/// Version of the feature table and of the rules that produce it (changes when the output of `detect` changes).
pub const FEAT_VERSION: u32 = 3;
