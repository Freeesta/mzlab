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
    Entry::new("file list", "mzlab-cli file list DIR [--json] [--out PATH]   what «Open folder» lists", |a| crate::folder_cmd::run(pre("list", a))).schema("folder/1").exits(&[0, 2, 4]),
    Entry::new("file info", "mzlab-cli file info FILE... [--sample LIST] [--out PATH]   what a file holds in one call: container, samples, items, channels, proposed kind as JSON", crate::file_info_cmd::run)
        .schema("file_info/1")
        .exits(&[0, 2, 4, 5]),
    Entry::new("file inventory", "mzlab-cli file inventory FILE.wiff... [--full]   what a .wiff + .wiff.scan holds, sample by sample, with SHA-256 and a status as JSON", |a| crate::wiff_cmd::run(pre("list", a))),
    Entry::new("file verify", "mzlab-cli file verify OUT.wiff --against SRC.wiff...   whether an output equals its sources", |a| crate::wiff_cmd::run(pre("verify", a))),
    Entry::new("file items", "mzlab-cli file items FILE [--no-points] [--out PATH]   what the page learns about a file as JSON: counts, ranges, ms2_exps", crate::items_cmd::run).schema("items/1").exits(&[0, 2, 4, 6]),
    Entry::new("file channels", "mzlab-cli file channels FILE [--sample LIST] [--out PATH]   the channels of a file as JSON", crate::channels_cmd::run).schema("channels/1").examples(&["mzlab-cli file channels sample.mzML"]),
    Entry::new("file pair", "mzlab-cli file pair FILE... [--json]   the other half of a SCIEX pair (.wiff + .wiff.scan) by name", crate::pair_cmd::run).exits(&[0, 1, 2, 4]),
    Entry::new("file validate", "mzlab-cli file validate FILE.mzML... [--json]   is the mzML valid: structure, PSI-MS vocabulary, index and SHA-1", validate).schema("mzml-validation/1").exits(&[0, 1, 2, 4]),
    Entry::new("file convert", "mzlab-cli file convert INPUT... --out FOLDER [...]   vendor files and folders to indexed mzML; --dry-run, --json", crate::convert::run).schema("convert/1").exits(&[0, 1, 2, 3, 4]),
    Entry::new("sample vars", "mzlab-cli sample vars NAME...   how file names split into variables", |a| crate::samples_cmd::run(pre("vars", a))),
    Entry::new("sample group", "mzlab-cli sample group NAME...   the group name the file names share", |a| crate::samples_cmd::run(pre("group", a))),
    Entry::new("sample table", "mzlab-cli sample table NAME...   the sample table proposed for names", |a| crate::samples_cmd::run(pre("table", a))),
    Entry::new("sample list", "mzlab-cli sample list FILE...   what a file holds sample by sample, as JSON", |a| crate::samples_cmd::run(pre("list", a))).schema("sample-list/1").exits(&[0, 1, 2, 4]),
    Entry::new("sample select", "mzlab-cli sample select FILE --sample LIST   which samples LIST picks", |a| crate::samples_cmd::run(pre("select", a))).schema("sample-select/1").exits(&[0, 2, 4, 5]),
    Entry::new("sample design propose", "mzlab-cli sample design propose [FILE...] [--samples S.json] [--out PATH]   propose a design from names", |a| crate::design_cmd::run(pre("propose", a)))
        .schema("design_proposal/1")
        .exits(&[0, 2, 3, 4, 6])
        .examples(&["mzlab-cli sample design propose f.mzML"]),
    Entry::new("sample design validate", "mzlab-cli sample design validate DESIGN.json [--samples S.json]   check a design, name bad keys", |a| crate::design_cmd::run(pre("validate", a)))
        .schema("design_check/1")
        .exits(&[0, 2, 3, 4])
        .examples(&["mzlab-cli sample design validate d.json"]),
    Entry::new("sample design canonical", "mzlab-cli sample design canonical DESIGN.json [--out PATH]   canonical bytes and SHA-256", |a| crate::design_cmd::run(pre("canonical", a)))
        .schema("design_canonical/1")
        .exits(&[0, 2, 3, 4, 6])
        .examples(&["mzlab-cli sample design canonical d.json"]),
    Entry::new("trace chrom", "mzlab-cli trace chrom FILE --id ID [--sample ID] [--rt A,B] [--n N] [--format json|csv] [--out PATH]   the trace of a channel (TIC, BPC, SRM, UV, analog) as JSON or rt_min,intensity CSV", crate::trace_cmd::chrom)
        .schema("trace/1")
        .exits(&[0, 2, 4, 5, 6]),
    Entry::new("trace xic", "mzlab-cli trace xic FILE --mz M (--ppm P | --da D) [--rt A,B] [--ms-level N] [--polarity +|-] [--sample ID] [--format json|csv] [--out PATH]   the extracted ion chromatogram as JSON or CSV", crate::trace_cmd::xic)
        .schema("trace/1")
        .exits(&[0, 2, 4, 5, 6]),
    Entry::new("trace background", "mzlab-cli trace background FILE [--set NAME=VALUE...]   the background trace as JSON", crate::plain::background_cmd::run).exits(&[0, 2, 4]),
    Entry::new("dad subtract", "mzlab-cli dad subtract FILE ...   the DAD matrix A minus B as JSON", |a| crate::dad_cmd::run(pre("subtract", a))).exits(&[0, 2, 4, 5]),
    Entry::new("dad spectrum", "mzlab-cli dad spectrum FILE ...   the UV spectrum at a time as JSON", |a| crate::dad_cmd::run(pre("spectrum", a))).exits(&[0, 2, 4, 5]),
    Entry::new("dad trace", "mzlab-cli dad trace FILE ...   the UV trace at a wavelength as JSON", |a| crate::dad_cmd::run(pre("trace", a))).exits(&[0, 2, 4, 5]),
    Entry::new("dad ms-at", "mzlab-cli dad ms-at FILE ...   the MS at a DAD time as JSON", |a| crate::dad_cmd::run(pre("ms-at", a))).exits(&[0, 2, 4, 5]),
    Entry::new("dad delay", "mzlab-cli dad delay FILE ...   the DAD to MS delay as JSON", |a| crate::dad_cmd::run(pre("delay", a))).exits(&[0, 2, 4, 5]),
    Entry::new("spectrum get", "mzlab-cli spectrum get FILE (--rt MIN | --rt A,B | --scan NO) [--ms-level N] [--id CHANNEL] [--sample ID] [--format json|csv] [--out PATH]   one mass spectrum as JSON or mz,intensity CSV", crate::spectrum_cmd::get)
        .schema("spectrum/1")
        .exits(&[0, 2, 4, 5, 6]),
    Entry::new("spectrum list", "mzlab-cli spectrum list FILE [--ms-level N] [--id CHANNEL] [--sample ID] [--format json|csv] [--out PATH]   the scans of a file (number, level, rt, TIC, precursor, base peak)", crate::spectrum_cmd::list)
        .schema("scanlist/1")
        .exits(&[0, 2, 4, 5, 6]),
    Entry::new("spectrum dda-window", "mzlab-cli spectrum dda-window FILE --from MIN --to MIN   the MS2 of a window by precursor", crate::plain::dda_window_cmd::run).exits(&[0, 2, 4]),
];
