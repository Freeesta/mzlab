//! What a native executor hands back to the `FileRunner` (`flow.rs`): its output file, the values it measured, and a second output for the steps that have two.

use mzlab_core::flow::{Artifact, Job, Mem};
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
    /// The preset the plan named, else the one the files decided (`lr` for a detection on unit-resolution files); the record names it.
    pub preset: Option<&'static str>,
}

/// The preset ids a step can decide from its files, as the static text `Made` holds.
pub(crate) fn preset_id(id: &str) -> Option<&'static str> {
    ["lr", "nominal"].into_iter().find(|p| *p == id)
}

/// The preset the plan names for the step of `job`, else the one the files decided (`auto`): the record of a run names the preset in force.
pub(crate) fn named_preset(job: &Job, auto: Option<&'static str>) -> Option<&'static str> {
    let named = job
        .step
        .presets
        .iter()
        .find(|p| Some(p.id) == job.instance.preset.as_deref());
    named.map(|p| p.id).or(auto)
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
            preset: None,
        }
    }
}
