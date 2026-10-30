//! What a native executor hands back to the `FileRunner` (`flow.rs`): its output file, the values it measured, and a second output for the steps that have two.

use mzlab_core::flow::{Artifact, Mem};
use mzlab_core::json::Value;

/// What an executor made: the output port, its file and the values it measured (name, value, proof).
pub(crate) struct Made {
    pub port: &'static str,
    pub artifact: Artifact,
    pub estimates: Vec<(String, Value, Value)>,
    pub warnings: Vec<String>,
    /// What a per-file step says of its file (`{"via": "centroided" | "passthrough", ...}`).
    pub file: Option<Value>,
    pub mem: Option<Mem>, // the memory of an alignment, when its step already worked it out
    /// A second output of a step that has two (port, file), made and kept like the first.
    pub extra: Option<(&'static str, Artifact)>,
}

impl Made {
    pub fn new(port: &'static str, artifact: Artifact) -> Made {
        Made {
            port,
            artifact,
            estimates: Vec::new(),
            warnings: Vec::new(),
            file: None,
            mem: None,
            extra: None,
        }
    }
}
