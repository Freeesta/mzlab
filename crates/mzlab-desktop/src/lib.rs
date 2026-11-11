//! mzLab desktop: the native engine behind the `mzlab://` protocol of the window (`main.rs`, feature `app`).
//! Files are memory-mapped from the disk (`memmap2` lives only in this crate, never in the WASM graph); the answers are binary
//! (little-endian `f64` arrays with the layout of `mzlab-wasm`, no JSON) except errors and file summaries.

mod conv_queue;
#[cfg(test)]
mod conv_queue_tests;
mod conv_stamp;
#[cfg(feature = "app")]
pub mod dialog;
pub mod download;
pub mod download_sources;
pub mod engine;
mod feat;
mod feat_attributio;
mod feat_finder;
mod feat_graph;
mod feat_import;
mod feat_msn;
mod feat_progress;
mod feat_untargeted;
mod flow_live;
mod flow_progress;
mod flow_reuse;
mod flow_run;
mod flow_stage;
pub mod folder;
pub mod idxbuild;
mod idxbuild_nat;
pub mod insilico;
pub mod libraries;
mod llm;
pub mod mapped;
mod natives;
mod natives_slots;
#[cfg(test)]
mod natives_tests;
pub mod normative;
pub mod opening;
pub mod protocol;
mod ram;
pub mod release;
pub mod rpc;
mod rpc_lr;
mod rpc_masscal;
mod rpc_msn;
mod rpc_nat;
mod rpc_warm;
pub mod store;
pub mod update;

pub use engine::State;
pub use protocol::{handle, handle_body, Response};
