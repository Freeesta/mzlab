//! The folder of the data pointed at by the user (desktop app): the list is the ONE scan of the CLI (`mzlab_cli::folder_cmd`, `mzlab-cli folder list`): the compatible
//! files and the folder datasets (.D, .d, Waters .raw) as one row each, like `CartPP.collect` of the page. Only paths relative to the folder travel to the page;
//! `resolve` turns one back into a real path and refuses `..` and absolute paths.

use std::path::Path;

pub use mzlab_cli::folder_cmd::{resolve, Item};

/// The rows of the list as the page reads them (`folder/1` items).
pub fn list(root: &Path) -> Vec<serde_json::Value> {
    mzlab_cli::folder_cmd::list(root)
        .iter()
        .filter_map(|i| serde_json::from_str(&i.to_json()).ok())
        .collect()
}
