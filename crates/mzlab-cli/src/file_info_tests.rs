//! Tests of `file info`: its `items` and `channels` members are the documents of `file items` and `file channels`, on the synthetic files and on a `.wiff`.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::super::extract::tests::{mzml_file, run, synthetic};

fn text(o: &crate::Outcome) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

/// The channel ids of a document, in order.
fn ids(doc: &str) -> Vec<String> {
    let mut v = Vec::new();
    let mut rest = doc;
    while let Some(i) = rest.find("\"id\":\"") {
        rest = &rest[i + 6..];
        v.push(rest[..rest.find('"').unwrap()].to_string());
    }
    v
}

#[test]
fn info_holds_the_documents_of_items_and_channels() {
    let (dir, f) = mzml_file("info");
    let o = run(&["file", "info", &f]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let doc = text(&o);
    assert!(
        doc.starts_with("{\"schema\":\"file_info/1\",\"files\":["),
        "{doc}"
    );
    assert!(
        doc.ends_with("\n")
            && doc.contains("\"container\":\"mzml\"")
            && doc.contains("\"kind\":{\"proposed\":")
    );
    let items = text(&run(&["file", "items", &f]));
    let chans = text(&run(&["file", "channels", &f]));
    assert!(
        doc.contains(&format!("\"items\":{}", items.trim_end())),
        "items"
    );
    assert!(
        doc.contains(&format!("\"channels\":{}", chans.trim_end())),
        "channels"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn info_lists_the_channels_of_the_synthetic_files() {
    let Some(d) = synthetic() else { return };
    let mut files: Vec<PathBuf> = vec![
        d.join("B_MRM.mzML"),
        d.join("B_MS2.mzML"),
        d.join("C_IDA.mzML"),
        d.join("C_MRM-EPI.mzML"),
    ];
    if let Ok(rd) = std::fs::read_dir(d.join("canali")) {
        files.extend(
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "mzML")),
        );
    }
    for f in files {
        let f = f.to_string_lossy().into_owned();
        let info = run(&["file", "info", &f]);
        let chans = run(&["file", "channels", &f]);
        assert_eq!((info.code, chans.code), (0, 0), "{f}: {}", info.stderr);
        assert!(ids(&text(&info)).len() >= ids(&text(&chans)).len());
        assert!(text(&info).contains(text(&chans).trim_end()), "{f}");
    }
    // several files give one entry each, in order
    let (a, b) = (d.join("B_MRM.mzML"), d.join("B_MS2.mzML"));
    let two = text(&run(&[
        "file",
        "info",
        &a.to_string_lossy(),
        &b.to_string_lossy(),
    ]));
    assert_eq!(two.matches("\"container\":\"mzml\"").count(), 2);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn info_errors_have_their_codes() {
    let (dir, f) = mzml_file("info_err");
    assert_eq!(run(&["file", "info"]).code, 2);
    assert_eq!(run(&["file", "info", &f, "--sample", "1"]).code, 2);
    assert_eq!(run(&["file", "info", &f, "--bogus"]).code, 2);
    assert_eq!(run(&["file", "info", &format!("{f}.missing")]).code, 4);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn info_names_every_sample_of_a_wiff_and_reads_the_chosen_one() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_info_wiff_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = Command::new("python3")
        .arg(tools.join("wiff_riferimento.py"))
        .arg(dir.join("in"))
        .output();
    if !made.is_ok_and(|o| o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("skipped: python3 not available");
        return;
    }
    let wiff = dir.join("in/two.wiff").to_string_lossy().into_owned();
    let first = run(&["file", "info", &wiff]);
    assert_eq!(first.code, 0, "{}", first.stderr);
    let d = text(&first);
    assert_eq!(d.matches("\"name\":\"two_s").count(), 2, "{d}");
    assert_eq!(
        d.matches("\"channels\":{").count(),
        1,
        "only the first sample is read: {d}"
    );
    let two = run(&["file", "info", &wiff, "--sample", "2"]);
    assert_eq!(two.code, 0, "{}", two.stderr);
    let (d2, want) = (
        text(&two),
        text(&run(&["file", "channels", &wiff, "--sample", "2"])),
    );
    assert!(
        d2.contains(want.trim_end()) && d2.contains("\"proposed\":\"lc-mrm\""),
        "{d2}"
    );
    let bad = run(&["file", "info", &wiff, "--sample", "9"]);
    assert_eq!(bad.code, 5, "{}", bad.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}
