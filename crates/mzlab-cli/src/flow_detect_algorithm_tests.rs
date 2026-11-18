//! The `algorithm` slot of `features.detect` on the synthetic fixture of `flow_run_tests`: the parameters of the trace stage are common to every algorithm; `branches`: the recipe
//! text of the browser pool over a plan with two branches.

use mzlab_core::budget::sha256_hex;

use crate::flow_run_tests::{fixture, flow_run, folder, p};

fn run(tag: &str, set: &[&str]) -> Vec<(String, Vec<u8>)> {
    let (dir, _) = fixture(tag);
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let mut args = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
    for s in set {
        args.extend(["--set", s]);
    }
    let o = flow_run(&args);
    assert_eq!(o.code, 0, "{set:?}: {}", o.stderr);
    let files = folder(&out);
    let _ = std::fs::remove_dir_all(&dir);
    files
}

fn sha(files: &[(String, Vec<u8>)], name: &str) -> String {
    let f = files
        .iter()
        .find(|f| f.0 == name)
        .unwrap_or_else(|| panic!("{name} is not in the run folder"));
    sha256_hex(&f.1)
}

/// The default run (the algorithm `masstrace`, every parameter at its default) writes these bytes; moving a parameter between the descriptor of the
/// step and that of an algorithm must not change them, nor the name of the file. The fixture is a unit-resolution file, so its recipe text carries the
/// noise gate of the unit recipe (`min_snr`).
#[test]
fn the_default_detection_is_byte_identical_to_the_one_before_the_scope_moved() {
    let files = run("default", &[]);
    for (name, expected) in [
        (
            "det-s_t000.mzfeatf",
            "835fe8d0a0c04b7a5251b4f140fe0d3a4ecb6c9b9ccabd1c0ef6d20751b0149c",
        ),
        (
            "det-s_t010.mzfeatf",
            "a6f28bfc78ab2cd5b47bed199fda2d504b77701fb8f3b146d5520cb3cc1793e7",
        ),
        (
            "det-s_t030.mzfeatf",
            "e15dfb28d57b7b55389e2ea8225fc08cb5d34cc0a9bc378b708ffd534cffd55b",
        ),
        (
            "gap.mzfeat",
            "f5cdab64bf4416020a115a9340bd9b51de15110ab12cfc5e6790af9d90935326",
        ),
    ] {
        assert_eq!(sha(&files, name), expected, "{name}");
    }
}

/// With the two noise gates set to 0 the unit recipe writes the bytes it wrote before them: the gates drop no row of this fixture and the recipe text
/// has no field for a gate that is off.
#[test]
fn the_noise_gates_off_write_the_bytes_of_the_recipe_without_them() {
    let files = run("gates_off", &["det.min_width=0", "det.min_snr=0"]);
    for (name, expected) in [
        (
            "det-s_t000.mzfeatf",
            "e527bf64bf631c5b378dd602d10f54dc844756d8eab433bb7048128e6d2c77c1",
        ),
        (
            "det-s_t010.mzfeatf",
            "b744dbf66874c57292be47b23d3288500e65f9101f18a773ee90aba19d11f524",
        ),
        (
            "det-s_t030.mzfeatf",
            "4d1d5ef70327dbf68e5a06f198d170a61dbb9c466631b98c2259f5356021b67b",
        ),
        (
            "gap.mzfeat",
            "8f282dd1d7ed562aa9a652594f5e3764f0e3199b942a964c8dc4ff230219ba52",
        ),
    ] {
        assert_eq!(sha(&files, name), expected, "{name}");
    }
}

/// `min_points`, `max_gap`, `long_trace` and `merge_overlap` act on the traces whatever peak model follows, so every algorithm can set them.
#[test]
fn the_trace_parameters_can_be_set_with_every_algorithm() {
    let plain = run("cw_default", &["det.algorithm=centwave"]);
    let strict = run(
        "cw_strict",
        &["det.algorithm=centwave", "det.min_points=40"],
    );
    assert_ne!(
        sha(&strict, "gap.mzfeat"),
        sha(&plain, "gap.mzfeat"),
        "min_points reaches the result with centwave"
    );
    let as_run = String::from_utf8(
        strict
            .iter()
            .find(|f| f.0 == "plan.json")
            .unwrap()
            .1
            .clone(),
    )
    .unwrap();
    assert!(as_run.contains("\"min_points\":40"), "{as_run}");
    for set in [
        "det.max_gap=1",
        "det.long_trace=200",
        "det.merge_overlap=0.5",
    ] {
        run("cw_other", &["det.algorithm=centwave", set]);
    }
    run("mt_strict", &["det.min_points=40"]);
}

mod branches;
mod lineage;
