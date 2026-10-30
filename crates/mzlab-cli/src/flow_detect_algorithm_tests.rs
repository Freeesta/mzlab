//! The `algorithm` slot of `features.detect` on the synthetic fixture of `flow_run_tests`: the parameters of the trace stage are common to every algorithm.

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
            "ff058e59e3f2c26aafc5814762964760818dfaf4feb01ff7cb26dc2ae27ef1b3",
        ),
        (
            "det-s_t010.mzfeatf",
            "3d297c50185165352e3931b5ee583600301babb5136249c835e04334b5f33ff6",
        ),
        (
            "det-s_t030.mzfeatf",
            "497f7699cedf24bd0e67991d8c7b4dd62849b36ddd3768bfe162f0af1479861a",
        ),
        (
            "gap.mzfeat",
            "1f728c9100c3281b0b8b58547537722543cc663bca1d6c58c45d6e7432f6511f",
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
            "a14770cb661f08f2a983c9246d84d72b476fc2a835bf5ccf2eba226399a5191e",
        ),
        (
            "det-s_t010.mzfeatf",
            "a06927336c5933f417782d6df365115bc52d1150dbe3429d22486dcc2519955e",
        ),
        (
            "det-s_t030.mzfeatf",
            "2a5364a75e2c7605d856e20fe3a04cba9d1da6529fac7343dcbbfa99752680b5",
        ),
        (
            "gap.mzfeat",
            "7a5e8acbdf439d7231c8b95a32b718740560963714686ead0c9740bdb45ff311",
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
