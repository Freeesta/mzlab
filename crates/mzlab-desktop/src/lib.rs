//! mzLab desktop: the native engine behind the `mzlab://` protocol of the window (`main.rs`, feature `app`).
//! Files are memory-mapped from the disk (`memmap2` lives only in this crate, never in the WASM graph); the answers are binary
//! (little-endian `f64` arrays with the layout of `mzlab-wasm`, no JSON) except errors and file summaries.

pub mod cartella;
pub mod engine;
mod feat;
mod feat_finder;
pub mod idxbuild;
pub mod librerie;
pub mod mapped;
pub mod protocol;
pub mod rpc;
mod rpc_lr;
pub mod scarica;
pub mod store;
pub mod update;

pub use engine::State;
pub use protocol::{handle, handle_body, Response};
