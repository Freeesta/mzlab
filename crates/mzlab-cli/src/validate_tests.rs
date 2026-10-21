//! `mzlab-cli validate` in this process: exit codes, the lines, the JSON.
use std::path::PathBuf;

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Opzioni};

use crate::execute;

fn written(indexed: bool) -> Vec<u8> {
    let h = Header {
        run: "x".into(),
        ..Default::default()
    };
    let mut w = if indexed {
        MzmlWriter::indicizzato(Vec::new(), &h, Opzioni::default())
    } else {
        MzmlWriter::new(Vec::new(), &h)
    }
    .unwrap();
    w.open_spectra(2).unwrap();
    for i in 0..2 {
        let sp = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: i as f64,
            polarity: 1,
            mz: vec![100.5, 200.25],
            intensity: vec![5.0, 6.0],
            ..Default::default()
        };
        w.spectrum(&sp).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// A folder of the test with `good.mzML` (indexed) and `bad.mzML` (plain, with an accession that does not exist).
fn folder(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mzlab-validate-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("good.mzML"), written(true)).unwrap();
    let mut bad = written(false);
    let at = bad.windows(10).position(|w| w == b"MS:1000285").unwrap();
    bad[at..at + 10].copy_from_slice(b"MS:1999285");
    std::fs::write(dir.join("bad.mzML"), bad).unwrap();
    dir
}

fn run(args: &[&str]) -> crate::Outcome {
    execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn a_valid_file_exits_0_with_a_summary_and_no_line() {
    let dir = folder("ok");
    let o = run(&["validate", dir.join("good.mzML").to_str().unwrap()]);
    assert_eq!((o.code, o.stdout.len()), (0, 0), "{}", o.stderr);
    assert!(
        o.stderr.contains(
            "good.mzML: valid (indexed, 2 spectra, 0 chromatograms, 0 errors, 0 warnings)"
        ),
        "{}",
        o.stderr
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_broken_file_exits_1_with_one_line_per_problem() {
    let dir = folder("bad");
    let o = run(&["validate", dir.join("bad.mzML").to_str().unwrap()]);
    assert_eq!(o.code, 1);
    let out = String::from_utf8(o.stdout).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 1, "{out}");
    assert!(
        lines[0].starts_with("error /mzML/run/spectrumList/spectrum[0]/cvParam[MS:1999285]"),
        "{out}"
    );
    assert!(
        lines[0].contains("accession: MS:1999285 is not a term"),
        "{out}"
    );
    assert!(o.stderr.contains("bad.mzML: INVALID"), "{}", o.stderr);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn several_files_prefix_each_line_and_json_is_an_array() {
    let dir = folder("many");
    let (g, b) = (dir.join("good.mzML"), dir.join("bad.mzML"));
    let (g, b) = (g.to_str().unwrap(), b.to_str().unwrap());
    let o = run(&["validate", g, b]);
    assert_eq!(o.code, 1);
    assert!(String::from_utf8(o.stdout)
        .unwrap()
        .lines()
        .all(|l| l.starts_with("bad.mzML: error ")));
    let o = run(&["validate", "--json", g, b]);
    let v = mzlab_core::json::parse(&o.stdout).unwrap();
    assert_eq!(v.arr().len(), 2);
    assert_eq!(
        v.at(0).and_then(|f| f.get("valid")).and_then(|x| x.bool()),
        Some(true)
    );
    let bad = v.at(1).unwrap();
    assert_eq!(bad.get("valid").and_then(|x| x.bool()), Some(false));
    assert_eq!(
        bad.get("problems")
            .unwrap()
            .at(0)
            .and_then(|p| p.get("code"))
            .and_then(|c| c.str()),
        Some("accession")
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn usage_errors_and_unreadable_files_exit_2() {
    assert_eq!(run(&["validate"]).code, 2);
    assert_eq!(run(&["validate", "--nope", "x.mzML"]).code, 2);
    let o = run(&["validate", "/no/such/file.mzML"]);
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("cannot be read"), "{}", o.stderr);
    let h = run(&["validate", "--help"]);
    assert_eq!(h.code, 0);
    assert!(String::from_utf8(h.stdout)
        .unwrap()
        .contains("usage: mzlab-cli validate"));
}
