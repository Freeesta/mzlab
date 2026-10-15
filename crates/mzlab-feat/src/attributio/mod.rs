// Ported from Attributio campaign.py, paths.py, compat.py, sorbents.py, readers/*.py.
//! Attributio's campaign folder in Rust: `campaign.toml`, the output tables of mzMine, SIRIUS and GNPS2, the FBMN MGF, the reference standards and the chemist's
//! decisions, with the DETECTED rule per site and the sorbent lists. Everything reads through `Io` (a `read(path)` closure and a `list(dir)` closure), so the same
//! code runs on the files the browser holds, on the desktop and in the tests. Paths are text with «/», relative to the campaign folder. The result is a set of
//! typed rows (`Sources`) that the level cascade uses without opening a file again.

pub mod campaign;
mod campaign_read;
pub mod detection;
pub mod error;
pub mod frame;
pub mod io;
pub mod readers;
pub mod sorbents;
pub mod sources;
pub mod toml;

pub use campaign::{Campaign, Env, POLARITIES};
pub use error::{CampaignError, Result};
pub use frame::{Cell, Frame, Record};
pub use io::{Entry, Io, MemFiles};
pub use sources::{load_sources, load_sources_mem, Sources};
pub mod review;
