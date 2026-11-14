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
            "555539449d112175355ad13c83304452be4c65d7f4ea20077596d17ec0739fb3",
        ),
        (
            "det-s_t010.mzfeatf",
            "c4ebc91136c8c7b9aac1663afec40438df895728e4e9874593b60e41f49934aa",
        ),
        (
            "det-s_t030.mzfeatf",
            "533914b83f5d6e86ce3a90894bd9b1203e24546053c0b81934b2fee539a7eb70",
        ),
        (
            "gap.mzfeat",
            "9544bf77393017468d1f5ea12be47c0952d3749e85c334eeb0d2ed6198944a92",
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
            "e6d68fb2e6678f30e661c3b1ac54aa49395f784b7a67452e2e4c04c7129baf5b",
        ),
        (
            "det-s_t010.mzfeatf",
            "b3c2c23e210aaeb40eaf9fda548c68434cf5c0b974ed3056c1d2c10caa5bb406",
        ),
        (
            "det-s_t030.mzfeatf",
            "45b170fcd8078a4b73d172f0723d6da45798e3b166e29447e6c5274ae6467906",
        ),
        (
            "gap.mzfeat",
            "ba49762eba0330e41e58d6308aa15420ac40095dcf91559c5181d2285d927575",
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
