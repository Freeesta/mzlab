//! The `file`, `raw`, `sample`, `trace`, `dad` and `spectrum` groups: what a file holds, how its samples are named and selected, its traces and spectra.

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

#[cfg(not(target_family = "wasm"))]
use crate::pack_cmd::pack_run;
#[cfg(not(target_family = "wasm"))]
use crate::pack_read_cmd::{unpack_run, verify_run};
#[cfg(target_family = "wasm")]
fn pack_run(_: Vec<String>) -> Outcome {
    Outcome::fail("unknown command file pack")
}
#[cfg(target_family = "wasm")]
fn unpack_run(_: Vec<String>) -> Outcome {
    Outcome::fail("unknown command file unpack")
}
#[cfg(target_family = "wasm")]
fn verify_run(_: Vec<String>) -> Outcome {
    Outcome::fail("unknown command file check-pack")
}

pub(crate) static TABLE: &[Entry] = &[
    Entry::new("file list", "mzlab-cli file list DIR [--json] [--out PATH]   what «Open folder» lists, .D / .raw folders included", |a| crate::folder_cmd::run(pre("list", a))).schema("folder/1"),
    Entry::new("file info", "mzlab-cli file info FILE... [--sample LIST] [--out PATH]   what a file holds: container, samples, items, channels, proposed kind", crate::file_info_cmd::run)
        .schema("file_info/1")
        .exits(&[0, 2, 4, 5]),
    Entry::new("file index", "mzlab-cli file index FILE... [--sample ID] [--out PATH]   pre-build the index cache as JSON", crate::file_index_cmd::run)
        .schema("index-build/1")
        .exits(&[0, 2, 4, 5, 6]),
    Entry::new("file inventory", "mzlab-cli file inventory FILE.wiff... [--full]   what a .wiff + .wiff.scan holds per sample, SHA-256 and status", |a| crate::wiff_cmd::run(pre("list", a))),
    Entry::new("file verify", "mzlab-cli file verify OUT.wiff --against SRC.wiff...   whether an output equals its sources", |a| crate::wiff_cmd::run(pre("verify", a))),
    Entry::new("file items", "mzlab-cli file items FILE [--no-points] [--out PATH]   what the page learns about a file: counts, ranges, ms2_exps with CE", crate::items_cmd::run).schema("items/1").exits(&[0, 2, 4, 6]),
    Entry::new("file method", "mzlab-cli file method FILE [--sample LIST] [--out PATH]   segment precursor, CE, CES, DP, fill time, Q1; null + reason", crate::method_cmd::run)
        .schema("file_method/1")
        .exits(&[0, 2, 4, 5, 6]),
    Entry::new("file channels", "mzlab-cli file channels FILE [--sample LIST] [--out PATH]   the channels of a file as JSON", crate::channels_cmd::run).schema("channels/1").examples(&["mzlab-cli file channels sample.mzML"]),
    Entry::new("file pair", "mzlab-cli file pair FILE... [--json]   the other half of a SCIEX pair found by name", crate::pair_cmd::run).exits(&[0, 1, 2]),
    Entry::new("file validate", "mzlab-cli file validate FILE.mzML... [--json]   is the mzML valid: structure, vocabulary, index and SHA-1", validate).schema("mzml-validation/1").exits(&[0, 1, 2, 4]),
    Entry::new("file pack", "mzlab-cli file pack PATH... --out DIR [--part-size SIZE] [--name NAME] [--dry-run] [--force]   split into parts", pack_run).schema("pack-run/1").exits(&[0, 2, 3, 4, 6, 8]),
    Entry::new("file unpack", "mzlab-cli file unpack MANIFEST --out DIR [--parts DIR] [--dry-run] [--force]   rebuild files", unpack_run).schema("unpack-run/1").exits(&[0, 1, 2, 3, 4, 6]),
    Entry::new("file check-pack", "mzlab-cli file check-pack MANIFEST [--parts DIR]   parts to resend", verify_run).schema("verify-pack/1").exits(&[0, 1, 2, 3, 4]),
    Entry::new("file convert", "mzlab-cli file convert INPUT... --out FOLDER [--name file|sample] [...]   vendor files to indexed mzML", crate::convert::run).schema("convert/1").exits(&[0, 1, 2, 3, 4, 5]),
    Entry::new("raw info", "mzlab-cli raw info FILE... [--sample ID] [--probe N] [--offset-ion MZ[@RT]] [--dry-run] [--out PATH]   data kind, ion quantum, grid, label check", crate::raw_cmd::info_run)
        .schema("raw-info/1")
        .exits(&[0, 1, 2, 4, 5, 6]),
    Entry::new("raw cache", "mzlab-cli raw cache FILE... [--sample ID] [--check] [--out PATH]   build or check the index cache entries", crate::raw_cache_cmd::cache_run)
        .schema("raw-cache/1")
        .exits(&[0, 1, 2, 4, 5, 6]),
    Entry::new("raw xic", "mzlab-cli raw xic FILE... --mz LIST --rt A,B [--da D] [--ms-level N] [--polarity +|-] [--mz-offset DA|auto:MZ[@RT]] [--set NAME=VALUE,...] [--sample ID] [--limit N] [--dry-run] [--out PATH]   peak, trace or nothing at a mass and time", crate::raw_xic_cmd::xic_run)
        .schema("raw-xic/1")
        .exits(&[0, 1, 2, 4, 5, 6]),
    Entry::new("raw scan", "mzlab-cli raw scan FILE... --rt A,B [--side MIN] [--gap MIN] [--bin nominal|0.1] [--mz-range LO,HI] [--top N] [--channel ID] [--ms-level N] [--polarity +|-] [--mz-offset DA|auto:MZ[@RT]] [--set NAME=VALUE,...] [--sample ID] [--limit N] [--full DIR] [--dry-run] [--out PATH]   what is in the window's spectrum that is not in the background", crate::raw_scan_cmd::scan_run)
        .schema("raw-scan/1")
        .exits(&[0, 1, 2, 4, 5, 6]),
    Entry::new("sample vars", "mzlab-cli sample vars NAME...   how file names split into variables", |a| crate::samples_cmd::run(pre("vars", a))),
    Entry::new("sample group", "mzlab-cli sample group NAME...   the file names' group name", |a| crate::samples_cmd::run(pre("group", a))),
    Entry::new("sample table", "mzlab-cli sample table NAME... [--text]   the sample table for the file names, roles logged on stderr", |a| crate::samples_cmd::run(pre("table", a))),
    Entry::new("sample import", "mzlab-cli sample import META.csv FILE|DIR... [--dry-run]   file -> sample name, role, group from a CSV", |a| crate::samples_cmd::run(pre("import", a))).schema("sample-import/1").exits(&[0, 1, 2, 3, 4, 6]),
    Entry::new("sample pair", "mzlab-cli sample pair SAMPLES.json [FILE...] [--table T.mzfeat]   how the samples pair with the files, roles proposed and samples left out", |a| crate::samples_cmd::run(pre("pair", a))).schema("sample-pair/1").exits(&[0, 1, 2, 3, 4]),
    Entry::new("sample list", "mzlab-cli sample list FILE...   what a file holds per sample", |a| crate::samples_cmd::run(pre("list", a))).schema("sample-list/1").exits(&[0, 1, 2, 4, 8]),
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
    Entry::new("trace chrom", "mzlab-cli trace chrom FILE --id ID [--sample ID] [--rt A,B] [--n N] [--format json|csv] [--out PATH]   a channel trace as JSON or rt_min,intensity CSV", crate::trace_cmd::chrom)
        .schema("trace/1")
        .exits(&[0, 2, 4, 5, 6]),
    Entry::new("trace xic", "mzlab-cli trace xic FILE (--mz M[,M..] | --sweep LO:HI:STEP) [--ppm P | --da D | --tol T] [--smooth 3|5] [--rt A,B] [--ms-level N] [--polarity +|-] [--sample ID] [--format json|csv] [--out PATH] [--dry-run]   XICs of m/z, lists in one call: trace/1, trace-set/1, CSV; --mz needs a tolerance; --dry-run xicplan/1", crate::trace_xic::xic)
        .schema("trace/1")
        .exits(&[0, 1, 2, 4, 5, 6]),
    Entry::new("trace background", "mzlab-cli trace background FILE [--set NAME=VALUE...]   the background trace of a file as JSON", crate::plain::background_cmd::run),
    Entry::new("dad subtract", "mzlab-cli dad subtract FILE ...   the DAD matrix A minus B", |a| crate::dad_cmd::run(pre("subtract", a))),
    Entry::new("dad spectrum", "mzlab-cli dad spectrum FILE ...   the UV spectrum at a time", |a| crate::dad_cmd::run(pre("spectrum", a))),
    Entry::new("dad trace", "mzlab-cli dad trace FILE ...   the UV trace at a wavelength", |a| crate::dad_cmd::run(pre("trace", a))),
    Entry::new("dad ms-at", "mzlab-cli dad ms-at FILE ...   the MS linked to a DAD time", |a| crate::dad_cmd::run(pre("ms-at", a))),
    Entry::new("dad delay", "mzlab-cli dad delay FILE ...   the delay between the DAD and the MS", |a| crate::dad_cmd::run(pre("delay", a))),
    Entry::new("dad peaks", "mzlab-cli dad peaks FILE ...   UV peaks, lambda-max, MS ions", |a| crate::dad_cmd::run(pre("peaks", a))),
    Entry::new("spectrum get", "mzlab-cli spectrum get FILE (--rt MIN | --rt A,B | --scan NO) [--ms-level N] [--id CHANNEL] [--sample ID] [--sum] [--fill-ms T] [--minus FILE] [--minus-sample ID] [--minus-rt A,B] [--bin nominal|0.1 [--mz-offset DA|auto:MZ[@RT]]] [--format json|csv] [--out PATH]   one spectrum as JSON or CSV; --sum adds scans; --minus a blank", crate::spectrum_cmd::get)
        .schema("spectrum/1")
        .exits(&[0, 1, 2, 4, 5, 6]),
    Entry::new("spectrum list", "mzlab-cli spectrum list FILE [--ms-level N] [--id CHANNEL] [--sample ID] [--format json|csv] [--out PATH]   the scans of a file (number, level, rt, TIC, precursor, base peak)", crate::spectrum_cmd::list)
        .schema("scanlist/1")
        .exits(&[0, 2, 4, 5, 6]),
    Entry::new("spectrum dda-window", "mzlab-cli spectrum dda-window FILE --from MIN --to MIN   the MS2 of a window by precursor", crate::plain::dda_window_cmd::run),
];
