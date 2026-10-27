//! mzLab spectral libraries (pure Rust, no `unsafe`): parsers, the `.mzlib` format, Flash-style indexes and scores.
#![forbid(unsafe_code)]

pub mod all_pairs;
pub mod build;
pub mod clean;
pub mod codec;
pub mod contrib;
pub mod decoy;
pub mod error;
mod fast_ln;
pub mod flash;
mod flash_resolve;
pub mod format;
pub mod frag_tol;
pub mod io;
pub mod libinfo;
pub mod libset;
pub mod matchres;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod parse;
pub mod prep;
pub mod preset;
#[warn(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
pub mod read;
pub mod scores;
pub mod search;
pub mod sha256;
pub mod sortutil;
pub mod spec;
pub mod stein;
pub mod util;

pub use error::{Error, Result};
