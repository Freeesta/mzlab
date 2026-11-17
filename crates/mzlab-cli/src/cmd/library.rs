//! Libraries, conversion, validation and the native-only commands.

use super::Entry;
use crate::Outcome;

// In the wasm build these commands exist in the help but run nowhere.
#[cfg(not(target_family = "wasm"))]
fn validate(a: Vec<String>) -> Outcome {
    crate::validate::run(a.into_iter())
}
#[cfg(not(target_family = "wasm"))]
fn normative(a: Vec<String>) -> Outcome {
    crate::normative::run(a)
}
#[cfg(not(target_family = "wasm"))]
fn insilico(a: Vec<String>) -> Outcome {
    crate::insilico_pack::run(a)
}
#[cfg(target_family = "wasm")]
fn validate(_: Vec<String>) -> Outcome {
    Outcome::fail("unknown command validate")
}
#[cfg(target_family = "wasm")]
fn normative(_: Vec<String>) -> Outcome {
    Outcome::fail("unknown command normative")
}
#[cfg(target_family = "wasm")]
fn insilico(_: Vec<String>) -> Outcome {
    Outcome::fail("unknown command insilico")
}

pub(crate) static TABLE: &[Entry] = &[
    Entry { name: "msn-lib", aliases: &[], help: "mzlab-cli msn-lib add|build|export|search ...                the library of MSn trees (msn-lib --help)", run: crate::msn_lib::cmd },
    Entry { name: "msn-rules", aliases: &[], help: "mzlab-cli msn-rules RECORD... | sum LINES...   the fragmentation rules judged on library spectra (msn-rules --help)", run: crate::msn_rules::run },
    Entry { name: "convert", aliases: &[], help: "mzlab-cli convert INPUT... --out FOLDER [...]   vendor files and folders to indexed mzML (convert --help)", run: crate::convert::run },
    Entry { name: "validate", aliases: &[], help: "mzlab-cli validate FILE.mzML... [--json]   is the mzML valid: structure, PSI-MS vocabulary, index and SHA-1 (validate --help)", run: validate },
    Entry { name: "normative", aliases: &[], help: "mzlab-cli normative build --cache DIR --out DIR   build the EU and US regulatory lists from the official sources (native only; normative --help); insilico pack CSV OUT   the spectra.insilico candidate package", run: normative },
    Entry { name: "insilico", aliases: &[], help: "", run: insilico },
];
