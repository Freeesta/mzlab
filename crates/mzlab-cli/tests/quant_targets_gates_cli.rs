//! `mzlab-cli quant targets` shape gates through the real binary on synthetic mzML files (a real peak and one-scan noise blips in a matrix
//! control and a blank): the old behaviour, the gate, the evidence in the log, the hint on stderr and the exit codes of a bad gate value.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../mzlab-feat/tests/common_quant/mod.rs"]
#[allow(dead_code)]
mod common_quant;
use common_quant::{mzml, Ion};

fn cli(args: &[&str], at: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mzlab-cli"))
        .args(["quant", "targets"])
        .args(args)
        .current_dir(at)
        .output()
        .unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// The keys of the log of a run are exactly those `docs/schemi/quant-targets-1.json` requires (the rows are checked in quant_targets_cli.rs).
fn schema_lists_the_log_keys(json: &str) {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../docs/schemi/quant-targets-1.json")).unwrap();
    let doc: serde_json::Value = serde_json::from_str(json).unwrap();
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v
    };
    let keys = sorted(doc["log"].as_object().unwrap().keys().cloned().collect());
    let required = sorted(
        schema["properties"]["log"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap().to_string())
            .collect(),
    );
    assert_eq!(keys, required);
}

/// A matrix control (a real peak and one-scan noise blips) and a blank (blips only) as synthetic mzML, with their target table.
fn blip_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_quant_targets_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("runs")).unwrap();
    let ions = |real: f64| -> Vec<Ion> {
        let mut v = vec![Ion(200.0, 5.0, real, 0.1)];
        v.extend((0..8).map(|i| {
            Ion(
                210.0 + 10.0 * f64::from(i),
                5.0,
                30.0 + 8.0 * f64::from(i),
                0.02,
            )
        }));
        v
    };
    let mut t = String::from("id,mz,rt_min,rt_max\nreal,200.0,4.0,6.0\n");
    for i in 0..8 {
        t += &format!("blip{i},{},4.0,6.0\n", 210 + 10 * i);
    }
    std::fs::write(d.join("targets.csv"), t).unwrap();
    std::fs::write(d.join("runs/Matrix.mzML"), mzml(&ions(5000.0), 7)).unwrap();
    std::fs::write(d.join("runs/Blank_01.mzML"), mzml(&ions(0.0), 9)).unwrap();
    d
}

#[test]
fn the_shape_gate_turns_blips_into_forced_rows_with_evidence_a_hint_and_exit_codes() {
    let d = blip_dir("gates");
    let base = [
        "--targets",
        "targets.csv",
        "runs",
        "--blank-pattern",
        "blank*",
    ];
    let run = |extra: &[&str]| {
        let mut a = base.to_vec();
        a.extend_from_slice(extra);
        cli(&a, &d)
    };
    let count = |o: &Output, word: &str| {
        text(&o.stdout)
            .lines()
            .filter(|l| l.contains(&format!(",{word},")))
            .count()
    };
    let old = run(&[]);
    assert_eq!(old.status.code(), Some(0), "{}", text(&old.stderr));
    assert!(
        count(&old, "detected") > 2,
        "the blips pass S/N 3 with the old defaults: {}",
        text(&old.stdout)
    );
    // the hint is on stderr (data stays on stdout) and names the parameters
    assert!(
        text(&old.stderr).contains("warn.quant.targets.forced"),
        "{}",
        text(&old.stderr)
    );
    assert!(
        text(&old.stderr).contains("min_half"),
        "{}",
        text(&old.stderr)
    );
    let new = run(&["--set", "min_half=3", "--log", "log.json"]);
    assert_eq!(new.status.code(), Some(0), "{}", text(&new.stderr));
    assert_eq!(count(&new, "detected"), 1, "{}", text(&new.stdout));
    // 18 rows: the real peak detected, one blank target without any peak (absent), the rest forced
    assert_eq!(count(&new, "forced"), 16, "{}", text(&new.stdout));
    let log: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(d.join("log.json")).unwrap()).unwrap();
    assert_eq!(log["params"]["min_half"], 3);
    let dec = log["decisions"].as_array().unwrap();
    assert_eq!(dec.len(), count(&new, "forced"));
    let half = dec[0]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "half_pts")
        .unwrap();
    assert_eq!(
        (half["limit"].as_f64(), half["op"].as_str()),
        (Some(3.0), Some(">="))
    );
    // the log keys are the schema's, and every state is a word of the vocabulary
    schema_lists_the_log_keys(&text(&run(&["--set", "min_half=3", "--json"]).stdout));
    // exit codes: a gate out of its range is a usage error (2), never clamped; the data is untouched on stdout
    for bad in [
        "max_asym=0.5",
        "min_half=-1",
        "min_half=1.5",
        "max_fwhm=-3",
        "min_half=x",
        "max_widht=3",
    ] {
        let o = run(&["--set", bad]);
        assert_eq!(o.status.code(), Some(2), "{bad}: {}", text(&o.stderr));
        assert!(o.stdout.is_empty(), "{bad}");
    }
    let _ = std::fs::remove_dir_all(&d);
}
