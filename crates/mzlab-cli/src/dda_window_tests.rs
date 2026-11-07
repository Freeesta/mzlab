//! `mzlab-cli dda-window`: on a synthetic Orbitrap DDA file of `tools/dati_sintetici.py` (skipped without python3 and numpy, `MZLAB_GOLDEN_STRICT=1`
//! fails instead) the CLI gives the bytes of the engine of the page (`Routes::handle("/api/dda_window")` on the index built in memory, as the site builds it).

use std::path::Path;
use std::process::Command;

use crate::execute;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn cli_gives_the_bytes_of_the_engine() {
    let d = std::env::temp_dir().join(format!("mzlab_cli_ddaw_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("dda.mzML");
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let script = "import sys; import numpy as np; sys.path.insert(0, sys.argv[1]); import dati_sintetici as d; d.hr_dda(sys.argv[2], np.random.default_rng(31), 'exploris')";
    let ok = Command::new("python3")
        .args(["-c", script])
        .arg(&tools)
        .arg(&f)
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return;
    }
    let b = std::fs::read(&f).unwrap();
    let mut src = b.clone();
    let p = mzlab_core::upload::prepare(&mut src, b.len() as u64, "dda.mzML").unwrap();
    let idx = mzlab_core::mzidx::build_mzml_to_vec(&p.table, &b, 64 << 20)
        .unwrap()
        .0;
    let n = idx.len() as u64;
    let mut r = mzlab_core::routes::Routes::new(mzlab_core::idxread::Index::open(idx, n).unwrap());
    let fp = f.to_string_lossy().into_owned();
    let mut groups = 0;
    for (a, b, hr) in [
        ("13", "16", true),
        ("16", "13", false),
        ("0", "1000", true),
        ("500", "600", true),
    ] {
        let mut args = vec!["dda-window", &fp, "--from", a, "--to", b];
        if !hr {
            args.extend(["--set", "hr=0"]);
        }
        let o = run(&args);
        assert_eq!(o.code, 0, "{}", o.stderr);
        let want = r
            .handle(
                "/api/dda_window",
                &super::query(a.parse().unwrap(), b.parse().unwrap(), hr),
            )
            .unwrap()
            .unwrap();
        let got = String::from_utf8(o.stdout).unwrap();
        assert_eq!(got, want + "\n", "{a}-{b}");
        groups += got.matches("\"prec\"").count();
    }
    assert!(groups > 10, "only {groups} groups");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn usage_errors_exit_with_2() {
    for a in [
        vec!["dda-window"],
        vec!["dda-window", "a.mzML", "--from", "1"],
        vec!["dda-window", "a", "b", "--from", "1", "--to", "2"],
        vec!["dda-window", "a", "--from", "x", "--to", "2"],
        vec![
            "dda-window",
            "a",
            "--from",
            "1",
            "--to",
            "2",
            "--set",
            "hr=2",
        ],
        vec![
            "dda-window",
            "/nonexistent/x.mzML",
            "--from",
            "1",
            "--to",
            "2",
        ],
    ] {
        assert_eq!(run(&a).code, 2, "{a:?}");
    }
    assert_eq!(run(&["dda-window", "--help"]).code, 0);
}
