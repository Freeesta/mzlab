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
            "113cbf914b837298a24d40fc274a94fe9199fd57b5d91f1c489892c327851eb0",
        ),
        (
            "det-s_t010.mzfeatf",
            "8e2a96674a9c348076a3fa8afeb2ab6cde0d2275ccaa97ad522bfc3a1642ef07",
        ),
        (
            "det-s_t030.mzfeatf",
            "af6b6fedff65a05c57087a3377c503e26046fe18a1b0090b3621240649ffedc7",
        ),
        (
            "gap.mzfeat",
            "978757668d78445e7a117770103258a019e0ee6ab4dd1ca3d30873d779aa7262",
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
            "f8b92e711618604b9ecde90518ea74c2aa67cdba56a7e1a151fbba1196a873eb",
        ),
        (
            "det-s_t010.mzfeatf",
            "2c1a2c9bdc04b546559164990192404b66f266c15261f9b70033dfb25ba3b1d5",
        ),
        (
            "det-s_t030.mzfeatf",
            "103b80c19f503dc86979fe59ccb4d9f9baf371afda2f85270ddac8264da4150a",
        ),
        (
            "gap.mzfeat",
            "d471b2cc2e5ebbb310e3402355d2c8956745a24608fe01df53876d176daccbc3",
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
