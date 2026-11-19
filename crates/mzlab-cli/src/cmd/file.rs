//! The `file`, `sample`, `trace`, `dad` and `spectrum` groups: what a file holds, how its samples are named and selected, its traces and spectra.

use super::{pre, Entry};
use crate::Outcome;

#[cfg(not(target_family = "wasm"))]
fn validate(a: Vec<String>) -> Outcome {
    crate::validate::run(a.into_iter())
}
// In the wasm build the command exists in the help but runs nowhere.
#[cfg(target_family = "wasm")]
fn validate(_: Vec<String>) -> Outcome {
    Outcome::fail("unknown command file validate")
}

pub(crate) static TABLE: &[Entry] = &[
    Entry::new("file list", "mzlab-cli file list DIR [--json] [--out PATH]   what «Open folder» lists, .D / .raw folders included", |a| crate::folder_cmd::run(pre("list", a))).schema("folder/1"),
    Entry::new("file info", "mzlab-cli file info FILE.wiff... [--inventory]   what a .wiff + .wiff.scan holds, sample by sample, with SHA-256 and a status as JSON", |a| crate::wiff_cmd::run(pre("list", a))),
    Entry::new("file verify", "mzlab-cli file verify OUT.wiff --against SRC.wiff...   whether an output equals its sources", |a| crate::wiff_cmd::run(pre("verify", a))),
    Entry::new("file items", "mzlab-cli file items FILE [--out PATH]   what the page learns about a file as JSON: counts, ranges, ms2_exps with CE and its spread", crate::items_cmd::run),
    Entry::new("file channels", "mzlab-cli file channels FILE [--out PATH]   the channels of a file as JSON", crate::channels_cmd::run).schema("channels/1"),
    Entry::new("file pair", "mzlab-cli file pair FILE... [--json]   the other half of a SCIEX pair (.wiff + .wiff.scan) found by name in the folder of the file; exit 1 when one is missing", crate::pair_cmd::run),
    Entry::new("file validate", "mzlab-cli file validate FILE.mzML... [--json]   is the mzML valid: structure, PSI-MS vocabulary, index and SHA-1", validate),
    Entry::new("file convert", "mzlab-cli file convert INPUT... --out FOLDER [...]   vendor files and folders to indexed mzML", crate::convert::run),
    Entry::new("sample vars", "mzlab-cli sample vars NAME...   how the file names split into variables, as JSON", |a| crate::samples_cmd::run(pre("vars", a))),
    Entry::new("sample group", "mzlab-cli sample group NAME...   the group name the file names share, as JSON", |a| crate::samples_cmd::run(pre("group", a))),
    Entry::new("sample table", "mzlab-cli sample table NAME...   the sample table proposed for the file names", |a| crate::samples_cmd::run(pre("table", a))),
    Entry::new("sample list", "mzlab-cli sample list FILE...   what a file holds sample by sample, as JSON; exit 1 or 3 when a sample is refused", |a| crate::samples_cmd::run(pre("list", a))),
    Entry::new("sample select", "mzlab-cli sample select FILE --sample LIST   which samples --sample LIST picks", |a| crate::samples_cmd::run(pre("select", a))),
    Entry::new("trace background", "mzlab-cli trace background FILE [--set NAME=VALUE...]   the background trace of a file as JSON", crate::plain::background_cmd::run),
    Entry::new("dad subtract", "mzlab-cli dad subtract FILE ...   the DAD / PDA matrix A minus B as JSON", |a| crate::dad_cmd::run(pre("subtract", a))),
    Entry::new("dad spectrum", "mzlab-cli dad spectrum FILE ...   the UV spectrum at a time as JSON", |a| crate::dad_cmd::run(pre("spectrum", a))),
    Entry::new("dad trace", "mzlab-cli dad trace FILE ...   the UV trace at a wavelength as JSON", |a| crate::dad_cmd::run(pre("trace", a))),
    Entry::new("dad ms-at", "mzlab-cli dad ms-at FILE ...   the MS linked to a DAD time as JSON", |a| crate::dad_cmd::run(pre("ms-at", a))),
    Entry::new("dad delay", "mzlab-cli dad delay FILE ...   the delay between the DAD and the MS as JSON", |a| crate::dad_cmd::run(pre("delay", a))),
    Entry::new("spectrum dda-window", "mzlab-cli spectrum dda-window FILE --from MIN --to MIN   the MS2 of a window by precursor", crate::plain::dda_window_cmd::run),
];
