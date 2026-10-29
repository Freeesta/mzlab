//! `mzlab-cli convert` on a folder of synthetic vendor files: valid indexed mzML, the failed file named with its cause and code 1, the bytes of `flow run`
//! with a `convert.mzml` plan, the same bytes with 1 and 3 workers, the cache of the second run, a sample per mzML, the usage errors.

use std::path::Path;
use std::process::Command;

use mzlab_core::json::{parse, Value};

use crate::convert_fixtures::{folder, makers, wiff};
use crate::flow_run_tests::{flow_run, p};
use crate::Outcome;

fn convert(args: &[&str]) -> Outcome {
    let mut a = vec!["convert".to_string()];
    a.extend(args.iter().map(|x| x.to_string()));
    crate::execute(&a)
}

/// The mzML files of `dir`, by name.
fn mzmls(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|f| f.extension().is_some_and(|e| e == "mzML"))
        .map(|f| {
            (
                f.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(&f).unwrap(),
            )
        })
        .collect();
    v.sort();
    v
}

fn well_formed(file: &Path) {
    let text = std::fs::read_to_string(file).unwrap();
    assert!(
        text.starts_with("<?xml")
            && text.contains("<indexedmzML")
            && text.trim_end().ends_with("</indexedmzML>")
    );
    if let Ok(o) = Command::new("xmllint").arg("--noout").arg(file).output() {
        assert!(
            o.status.success(),
            "{}: {}",
            file.display(),
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

/// `src` with the two makers' files and a pair `.wiff`, or none without python3.
fn series(tag: &str, broken: bool) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let dir = folder(tag);
    let src = makers(&dir, broken);
    wiff(&src, "c", 1).then_some((dir, src))
}

#[test]
fn a_folder_gives_valid_mzml_and_the_broken_file_is_named_with_its_cause() {
    let Some((dir, src)) = series("folder", true) else {
        return;
    };
    let (out, rep) = (dir.join("out"), dir.join("rep.json"));
    let o = convert(&[p(&src), "--out", p(&out), "--report", p(&rep)]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    assert!(
        o.stderr.contains("rotto.raw: err.file.notAccepted"),
        "{}",
        o.stderr
    );
    let names: Vec<String> = mzmls(&out).into_iter().map(|f| f.0).collect();
    assert_eq!(names, ["a.mzML", "b.mzML", "c.mzML"]);
    for n in &names {
        well_formed(&out.join(n));
    }
    assert_eq!(String::from_utf8(o.stdout).unwrap().lines().count(), 3);

    let report = parse(&std::fs::read(&rep).unwrap()).unwrap();
    let list = report.arr();
    assert_eq!(list.len(), 4);
    let failed: Vec<&Value> = list.iter().filter(|r| r.get("errore").is_some()).collect();
    assert_eq!(failed.len(), 1);
    assert!(failed[0]
        .get("ingresso")
        .and_then(Value::str)
        .unwrap()
        .ends_with("rotto.raw"));
    assert!(failed[0]
        .get("causa")
        .and_then(Value::str)
        .is_some_and(|c| !c.is_empty()));
    for r in list.iter().filter(|r| r.get("errore").is_none()) {
        assert!(r.get("spettri").and_then(Value::f64).unwrap() > 0.0);
        assert_eq!(r.get("sha256").and_then(Value::str).unwrap().len(), 64);
        assert_eq!(
            r.get("conv/1")
                .and_then(|c| c.get("schema"))
                .and_then(Value::str),
            Some("conv/1")
        );
        assert_eq!(r.get("saltato").and_then(Value::bool), Some(false));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_bytes_are_those_of_flow_run_and_do_not_depend_on_the_workers() {
    let Some((dir, src)) = series("parity", false) else {
        return;
    };
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"a.mzXML","type":"raw"},{"name":"b.cdf","type":"raw"},{"name":"c.wiff","type":"raw"}],
      "steps":[{"id":"conv","step":"convert.mzml@1","in":{"raw":["file:a.mzXML","file:b.cdf","file:c.wiff"]}}]}"#;
    let (plan_file, flow_out) = (dir.join("plan.json"), dir.join("flow"));
    std::fs::write(&plan_file, plan).unwrap();
    let f = flow_run(&[p(&plan_file), "--in", p(&src), "--out", p(&flow_out)]);
    assert_eq!(f.code, 0, "{}", f.stderr);

    let (one, three) = (dir.join("one"), dir.join("three"));
    for (out, workers) in [(&one, "1"), (&three, "3")] {
        let o = convert(&[p(&src), "--out", p(out), "--workers", workers]);
        assert_eq!(o.code, 0, "{}", o.stderr);
    }
    let by_flow =
        ["a", "b", "c"].map(|n| std::fs::read(flow_out.join(format!("conv-{n}.mzML"))).unwrap());
    let got = mzmls(&one);
    for (k, (_, bytes)) in got.iter().enumerate() {
        assert_eq!(*bytes, by_flow[k], "file {k}");
    }
    assert_eq!(got, mzmls(&three));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_second_run_skips_every_file_and_puts_back_a_removed_one() {
    let dir = folder("cache");
    let src = makers(&dir, false);
    let (out, rep) = (dir.join("out"), dir.join("rep.json"));
    let args = [p(&src), "--out", p(&out), "--report", p(&rep)];
    assert_eq!(convert(&args).code, 0);
    let first = mzmls(&out);
    assert_eq!(first.len(), 2);

    std::fs::remove_file(out.join("a.mzML")).unwrap();
    let o = convert(&args);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(
        o.stderr.starts_with("done: 0 converted, 2 skipped"),
        "{}",
        o.stderr
    );
    assert_eq!(mzmls(&out), first);
    let report = std::fs::read_to_string(&rep).unwrap();
    assert_eq!(report.matches("\"saltato\":true").count(), 2);

    let o = convert(&[p(&src), "--out", p(&out), "--fresh"]);
    assert!(
        o.stderr.starts_with("done: 2 converted, 0 skipped"),
        "{}",
        o.stderr
    );
    assert_eq!(mzmls(&out), first);

    // another setting is another conversion
    let o = convert(&[p(&src), "--out", p(&out), "--zlib", "6", "--numpress"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stderr.starts_with("done: 2 converted"), "{}", o.stderr);
    assert_ne!(mzmls(&out), first);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_wiff_with_several_samples_gives_one_mzml_per_sample() {
    let dir = folder("samples");
    let src = dir.join("in");
    std::fs::create_dir_all(&src).unwrap();
    if !wiff(&src, "c", 2) {
        return;
    }
    let out = dir.join("out");
    let o = convert(&[p(&src.join("c.wiff")), "--out", p(&out)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let files = mzmls(&out);
    assert_eq!(
        files.len(),
        2,
        "{:?}",
        files.iter().map(|f| &f.0).collect::<Vec<_>>()
    );
    assert!(files
        .iter()
        .all(|f| f.0.starts_with("c-") && f.1.starts_with(b"<?xml")));
    assert_ne!(files[0].1, files[1].1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn usage_errors_and_a_missing_input_have_their_codes() {
    let dir = folder("usage");
    let src = makers(&dir, false);
    let out = dir.join("out");
    assert_eq!(convert(&[p(&src)]).code, 2);
    assert_eq!(convert(&["--out", p(&out)]).code, 2);
    assert_eq!(convert(&[p(&src), "--out", p(&out), "--bogus"]).code, 2);
    assert_eq!(
        convert(&[p(&src), "--out", p(&out), "--workers", "0"]).code,
        2
    );
    let o = convert(&[p(&src), "--out", p(&out), "--zlib", "12"]);
    assert_eq!(o.code, 2, "{}", o.stderr);
    assert_eq!(
        convert(&[p(&dir.join("missing.raw")), "--out", p(&out)]).code,
        4
    );
    let empty = dir.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    assert_eq!(convert(&[p(&empty), "--out", p(&out)]).code, 4);

    // an output over its own input
    let o = convert(&[p(&src), "--out", p(&src)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let o = convert(&[p(&src.join("a.mzML")), "--out", p(&src)]);
    assert_eq!(o.code, 2, "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn peaks_take_three_values_and_the_short_form_is_the_parameter() {
    let dir = folder("peaks");
    let src = makers(&dir, false);
    assert_eq!(
        convert(&[p(&src), "--out", p(&dir.join("o")), "--peaks", "x"]).code,
        2
    );
    for which in ["file", "mzlab"] {
        let (short, long) = (
            dir.join(format!("s-{which}")),
            dir.join(format!("l-{which}")),
        );
        let o = convert(&[p(&src), "--out", p(&short), "--peaks", which]);
        assert_eq!(o.code, 0, "{which}: {}", o.stderr);
        let set = format!("conv.peaks={which}");
        let o = convert(&[p(&src), "--out", p(&long), "--set", &set]);
        assert_eq!(o.code, 0, "{which}: {}", o.stderr);
        assert_eq!(mzmls(&short), mzmls(&long), "{which}");
    }
    // the synthetic files hold centroids only: the profile is asked in vain, and the cause is named
    let o = convert(&[p(&src), "--out", p(&dir.join("pr")), "--peaks", "profile"]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    assert!(o.stderr.contains("err.conv.noProfile"), "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn help_lists_every_option() {
    let o = convert(&["--help"]);
    assert_eq!(o.code, 0);
    let text = String::from_utf8(o.stdout).unwrap();
    for opt in [
        "--out",
        "--zlib",
        "--numpress",
        "--peaks",
        "--workers",
        "--report",
        "--fresh",
        "--set",
    ] {
        assert!(text.contains(opt), "{opt}");
    }
    assert!(
        String::from_utf8(crate::execute(&["--help".to_string()]).stdout)
            .unwrap()
            .contains("mzlab-cli convert")
    );
}

/// The same file name in two folders gives two mzML files (the position in the name), not one replaced by the other; names without a clash stay as they are.
#[test]
fn the_same_name_in_two_folders_gives_two_files() {
    let dir = folder("same_name");
    let src = makers(&dir, false);
    let other = dir.join("in2");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::copy(src.join("a.mzXML"), other.join("a.mzXML")).unwrap();
    let out = dir.join("out");
    let o = convert(&[p(&src), p(&other), "--out", p(&out)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let names: Vec<String> = mzmls(&out).into_iter().map(|f| f.0).collect();
    assert_eq!(names, ["0-a.mzXML.mzML", "2-a.mzXML.mzML", "b.mzML"]);
    assert_eq!(String::from_utf8(o.stdout).unwrap().lines().count(), 3);
    let _ = std::fs::remove_dir_all(&dir);
}
