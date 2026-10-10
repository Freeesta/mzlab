//! mzLab desktop: the native engine behind the `mzlab://` protocol of the window (`main.rs`, feature `app`).
//! Files are memory-mapped from the disk (`memmap2` lives only in this crate, never in the WASM graph); the answers are binary
//! (little-endian `f64` arrays with the layout of `mzlab-wasm`, no JSON) except errors and file summaries.

pub mod engine;
pub mod mapped;
pub mod protocol;
pub mod update;

pub use engine::State;
pub use protocol::{handle, Response};
