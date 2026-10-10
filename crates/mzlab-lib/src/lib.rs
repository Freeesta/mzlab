//! mzLab spectral libraries (pure Rust, no `unsafe`): parsers, the `.mzlib` format, Flash-style indexes and scores.
#![forbid(unsafe_code)]

pub mod build;
pub mod clean;
pub mod codec;
pub mod error;
pub mod format;
pub mod io;
pub mod parse;
pub mod read;
pub mod spec;
pub mod util;

pub use error::{Error, Result};
