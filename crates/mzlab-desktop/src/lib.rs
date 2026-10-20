//! mzLab desktop: the native engine behind the `mzlab://` protocol of the window (`main.rs`, feature `app`).
//! Files are memory-mapped from the disk (`memmap2` lives only in this crate, never in the WASM graph); the answers are binary
//! (little-endian `f64` arrays with the layout of `mzlab-wasm`, no JSON) except errors and file summaries.

pub mod cartella;
#[cfg(feature = "app")]
pub mod dialogo;
pub mod engine;
mod feat;
mod feat_attributio;
mod feat_finder;
mod feat_grafo;
mod feat_import;
mod feat_msn;
mod feat_untargeted;
mod flow_run;
pub mod idxbuild;
mod idxbuild_nat;
pub mod librerie;
pub mod mapped;
mod nativi;
mod nativi_slots;
pub mod protocol;
mod ram;
pub mod release;
pub mod rpc;
mod rpc_lr;
mod rpc_nat;
pub mod scarica;
pub mod store;
pub mod update;

pub use engine::State;
pub use protocol::{handle, handle_body, Response};
