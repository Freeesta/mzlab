//! The workflow of mzLab: read an `mzflow/2` file (`parse`), check the whole plan against the step descriptors before anything starts (`check`), resolve every
//! instance to its canonical parameters (`canon`), order it (`order`), run it through a `Runner` and write the `prov/1` record (`run`, `prov`).
//! Pure Rust with no I/O and no clock: the caller owns files and time (`crates/mzlab-cli/examples/flow.rs`), the same code runs in the engine and natively.

mod canon;
mod check;
mod corsa;
mod order;
mod parse;
mod prov;
mod run;

pub use canon::{cache_key, canonical, Resolved};
pub use check::{check, conversion_path, Platform, Problem};
pub use corsa::{build as corsa_build, Corsa, Uscita};
pub use order::{levels, plan_order};
pub use parse::parse;
pub use prov::{prov_json, Activity, Agent, ErrorRecord};
pub use run::{execute, Artifact, Job, Outcome, Run, Runner};

use crate::json::Value;

/// `program` of the file: the program that wrote it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Program {
    pub name: String,
    pub version: String,
    pub commit: Option<String>,
}

/// What is known about the data before it is read: empty = unknown (a requirement on an unknown facet is checked again when the step starts).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Observed {
    pub resolution: Vec<String>,
    pub polarity: Vec<i8>,
    pub ms_levels: Vec<u32>,
}

/// An entry of `inputs`: a file by name, identified by its SHA-256 (never by the name).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileInput {
    pub name: String,
    pub sha256: String,
    pub bytes: Option<u64>,
    /// A `dataType` name; absent = not declared, the type is not checked.
    pub ty: Option<String>,
    pub observed: Observed,
}

/// A reference of an `in` port: an input file or an output of another instance.
#[derive(Debug, Clone, PartialEq)]
pub enum Ref {
    File(String),
    Out { instance: String, port: String },
}

/// One step instance of the file.
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub id: String,
    /// The step id without the version (`features.detect`).
    pub step: String,
    pub version: u32,
    pub algorithm: Option<String>,
    pub preset: Option<String>,
    pub label: Option<String>,
    /// Port name and its references, in the order written.
    pub ins: Vec<(String, Vec<Ref>)>,
    /// Only the values that differ from the defaults, in the order written.
    pub params: Vec<(String, Value)>,
}

impl Instance {
    /// `id@version` as the file writes it.
    pub fn step_ref(&self) -> String {
        format!("{}@{}", self.step, self.version)
    }
    /// The instances this one reads from, once each, in the order of its references.
    pub fn sources(&self) -> Vec<&str> {
        let mut v: Vec<&str> = Vec::new();
        for (_, refs) in &self.ins {
            for r in refs {
                if let Ref::Out { instance, .. } = r {
                    if !v.contains(&instance.as_str()) {
                        v.push(instance);
                    }
                }
            }
        }
        v
    }
}

/// A workflow file, `mzflow/2` or the old `mzflow/1` (no steps, only its log of operations).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    pub format: String,
    pub program: Program,
    pub inputs: Vec<FileInput>,
    pub steps: Vec<Instance>,
    /// The log of `mzflow/1`, passed through untouched.
    pub operations: Option<Value>,
}

impl Plan {
    pub fn instance(&self, id: &str) -> Option<&Instance> {
        self.steps.iter().find(|s| s.id == id)
    }
    pub fn input(&self, name: &str) -> Option<&FileInput> {
        self.inputs.iter().find(|f| f.name == name)
    }
}
