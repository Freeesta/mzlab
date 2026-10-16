//! mzLab spectral libraries (pure Rust, no `unsafe`): parsers, the `.mzlib` format, Flash-style indexes and scores.
#![forbid(unsafe_code)]

pub mod all_pairs;
pub mod build;
pub mod clean;
pub mod codec;
pub mod decoy;
pub mod error;
pub mod format;
pub mod io;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod parse;
#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]
pub mod read;
pub mod scores;
pub mod search;
pub mod sortutil;
pub mod spec;
pub mod util;

pub use error::{Error, Result};
