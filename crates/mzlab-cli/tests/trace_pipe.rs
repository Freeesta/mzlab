//! `trace chrom --format csv | peak auto-lr -` through the real binary: the CSV of a trace is what the peak commands read.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn a_srm_trace_pipes_into_peak_auto_lr() {
    let d = std::env::temp_dir().join(format!("mzlab_trace_pipe_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let script = "import sys; from pathlib import Path; import numpy as np; sys.path.insert(0, sys.argv[1]); import dati_sintetici as d; \
                  o = Path(sys.argv[2]); o.mkdir(parents=True, exist_ok=True); d.mrm(o / 'B_MRM.mzML', 1e5, np.random.default_rng(1), 1)";
    let made = Command::new("python3")
        .args(["-c", script])
        .arg(&tools)
        .arg(&d)
        .status();
    if !made.is_ok_and(|s| s.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return;
    }
    let bin = env!("CARGO_BIN_EXE_mzlab-cli");
    let file = d.join("B_MRM.mzML");
    let list = Command::new(bin)
        .args(["file", "channels"])
        .arg(&file)
        .output()
        .unwrap();
    let list = String::from_utf8(list.stdout).unwrap();
    let at = list.find("\"id\":\"srm:").expect("an SRM channel");
    let id = &list[at + 6..at + 6 + list[at + 6..].find('"').unwrap()];
    let trace = Command::new(bin)
        .args(["trace", "chrom"])
        .arg(&file)
        .args(["--id", id, "--format", "csv"])
        .output()
        .unwrap();
    assert_eq!(
        trace.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&trace.stderr)
    );
    assert!(trace.stdout.starts_with(b"rt_min,intensity\n"));
    let mut peak = Command::new(bin)
        .args(["peak", "auto-lr", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    peak.stdin.take().unwrap().write_all(&trace.stdout).unwrap();
    let out = peak.wait_with_output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
}
