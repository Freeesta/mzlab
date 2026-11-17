//! Native orchestration of the engines for a transport (browser bridge, CLI, desktop app): no wasm-bindgen, js-sys or web-sys here.
//! `run` holds the plain modules moved out of the bridges, `op` the operations and `vfs` the virtual file system (filled by other chats).
#![forbid(unsafe_code)]

pub mod op;
pub mod run;
pub mod vfs;
