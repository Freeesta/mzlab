//! `mzlab-cli convert` on a `.wiff` pair with one broken unit (`tools/wiff_isolamento.py`): the healthy samples are written byte for byte as from the
//! whole pair (the SHA-1 digests of the `.wiff.scan` and of the document aside), the damaged one fails alone with its cause; a NaN cps is never
//! written and its cycle is counted; `manual-table` lists a sample at position 0 of the table. Skipped without python3.
use std::path::{Path, PathBuf};
use std::process::Command;

use super::super::convert;
use crate::convert_fixtures::folder;
use crate::flow_run_tests::p;

/// `x.wiff` + `x.wiff.scan` copied from pair `name` of the isolation script into `dir/in`, or none without python3.
fn pair(dir: &Path, name: &str) -> Option<PathBuf> {
    let src = dir.join("src");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let made = Command::new("python3")
        .arg(root.join("tools/wiff_isolamento.py"))
        .arg(&src)
        .output();
    if !made.is_ok_and(|o| o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP convert isolation: python3 not available");
        return None;
    }
    let inp = dir.join("in");
    std::fs::create_dir_all(&inp).unwrap();
    for ext in ["wiff", "wiff.scan"] {
        std::fs::copy(
            src.join(format!("{name}.{ext}")),
            inp.join(format!("x.{ext}")),
        )
        .unwrap();
    }
    Some(inp.join("x.wiff"))
}

/// The bytes with every run of exactly 40 hex digits (a SHA-1) blanked.
fn without_digests(b: &[u8]) -> Vec<u8> {
    let mut out = b.to_vec();
    let mut i = 0;
    while i < b.len() {
        let n = b[i..].iter().take_while(|c| c.is_ascii_hexdigit()).count();
        if n == 40 {
            out[i..i + n].fill(b'#');
        }
        i += n.max(1);
    }
    out
}

#[test]
fn the_healthy_samples_of_a_pair_with_a_damaged_one_are_written_as_from_the_whole_pair() {
    let (good, bad) = (folder("iso_sano"), folder("iso_guasto"));
    let (Some(g), Some(b)) = (pair(&good, "sano"), pair(&bad, "guasto")) else {
        return;
    };
    let o = convert(&[p(&g), "--out", p(&good.join("out"))]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let o = convert(&[p(&b), "--out", p(&bad.join("out"))]);
    assert_ne!(o.code, 0, "{}", o.stderr);
    assert!(
        o.stderr.contains("1 of 3 failed (2 converted"),
        "{}",
        o.stderr
    );
    assert!(
        o.stderr.contains("err.file.wiff detail=TOF block"),
        "{}",
        o.stderr
    );
    assert!(!bad.join("out/x-1.mzML").exists());
    for s in [2, 3] {
        let read = |d: &Path| std::fs::read(d.join(format!("out/x-{s}.mzML"))).unwrap();
        let (want, got) = (read(&good), read(&bad));
        assert_eq!(without_digests(&got), without_digests(&want), "sample {s}");
    }
    let _ = (
        std::fs::remove_dir_all(&good),
        std::fs::remove_dir_all(&bad),
    );
}

#[test]
fn a_nan_cps_is_never_written_and_its_cycle_is_counted() {
    let dir = folder("iso_nan");
    let Some(w) = pair(&dir, "mrm_nan") else {
        return;
    };
    let report = dir.join("r.json");
    let o = convert(&[p(&w), "--out", p(&dir.join("out")), "--report", p(&report)]);
    assert_eq!(o.code, 3, "{}", o.stderr);
    assert!(
        o.stderr
            .contains("partial read, 9 of 10 MRM cycles (warn.wiff.damaged)"),
        "{}",
        o.stderr
    );
    let r = std::fs::read_to_string(&report).unwrap();
    assert!(r.contains("\"unita\":\"cicli\""), "{r}");
    let f = std::fs::File::open(dir.join("out/x-1.mzML")).unwrap();
    let size = f.metadata().unwrap().len();
    let mut m = mzlab_core::mzmlhead::Mzml::new(f, size);
    let chroms = m.chromatograms(0).unwrap();
    assert_eq!(chroms.len(), 4, "TIC and three transitions");
    for c in &chroms {
        let (t, y) = m.chromatogram(c).unwrap();
        assert_eq!((t.len(), y.len()), (9, 9), "9 of 10 cycles");
        assert!(
            t.iter().chain(&y).all(|x| x.is_finite()),
            "a NaN or infinity in the mzML"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn manual_table_lists_a_sample_at_position_zero_of_the_table() {
    let dir = folder("iso_vuoto0");
    let Some(w) = pair(&dir, "vuoto0") else {
        return;
    };
    let mut warn = String::new();
    let files = crate::manual_table_cmd::files_json(&[w], &mut warn).unwrap();
    assert_eq!(files.matches("{\"name\":").count(), 2, "{files}");
    let _ = std::fs::remove_dir_all(&dir);
}
