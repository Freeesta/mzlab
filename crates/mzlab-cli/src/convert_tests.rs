//! `mzlab-cli convert` on a folder of synthetic vendor files: valid indexed mzML, the failed file named with its cause and code 1, the bytes of `flow run`
//! with a `convert.mzml` plan, the same bytes with 1 and 3 workers, the cache of the second run, a sample per mzML, the usage errors.

use std::path::Path;
use std::process::Command;

use mzlab_core::json::{parse, Value};

use crate::convert_fixtures::{folder, makers, wiff};
use crate::flow_run_tests::{flow_run, p};
use crate::Outcome;

#[path = "convert_tests_wiff.rs"]
mod wiff_notes;

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
        o.stderr.contains("rotto.raw: err.file.notThisFormat"),
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

/// An injection in the sample table that acquired nothing (no `Idx`): an empty mzML under its own number, as the site lists it with 0 scans, and the samples after it keep theirs.
#[test]
fn an_empty_injection_keeps_its_number_and_gives_an_empty_mzml() {
    let dir = folder("vuote");
    let src = dir.join("in");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let made = Command::new("python3")
        .arg(root.join("tools/wiff_riferimento.py"))
        .arg(&src)
        .output();
    if !made.is_ok_and(|o| o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        return;
    }
    let out = dir.join("out");
    let o = convert(&[p(&src.join("buchi.wiff")), "--out", p(&out)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let files = mzmls(&out);
    let names: Vec<&str> = files.iter().map(|f| f.0.as_str()).collect();
    assert_eq!(names, ["buchi-1.mzML", "buchi-2.mzML", "buchi-3.mzML"]);
    let spectra = |b: &[u8]| String::from_utf8_lossy(b).matches("<spectrum ").count();
    assert_eq!(
        files.iter().map(|f| spectra(&f.1)).collect::<Vec<_>>(),
        [1, 0, 1]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A `.wiff` the readers refuse (a TOF experiment, a method of two periods) gives code 1 and a key whose catalogue text, the one the site shows, has its parameters filled in.
#[test]
fn a_refused_wiff_is_told_with_the_text_of_the_site() {
    let dir = folder("rifiuti");
    let src = dir.join("in");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let made = Command::new("python3")
        .arg(root.join("tools/wiff_riferimento.py"))
        .arg(&src)
        .output();
    if !made.is_ok_and(|o| o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        return;
    }
    for (name, text) in [
        (
            "tof",
            "Not supported yet: TOF MS scan (scan type code 8, experiment 1 of this .wiff)",
        ),
        (
            "periodi",
            "The method of this .wiff has 2 periods and this program reads one-period methods only",
        ),
    ] {
        let o = convert(&[
            p(&src.join(format!("{name}.wiff"))),
            "--out",
            p(&dir.join("out")),
        ]);
        assert_eq!(o.code, 1, "{name}: {}", o.stderr);
        // the summary keeps the key for scripts; the progress line shows it through `umano`
        let cause = o
            .stderr
            .lines()
            .next()
            .and_then(|l| l.split_once(".wiff: "))
            .map(|c| c.1)
            .unwrap_or_default();
        assert!(
            crate::error_text::human(cause).contains(text),
            "{name}: {}",
            o.stderr
        );
    }
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
    let run =
        |opt: &str, val: &str, out: &str| convert(&[p(&src), "--out", p(&dir.join(out)), opt, val]);
    assert_eq!(run("--peaks", "x", "o").code, 2);
    let (a, b) = (
        run("--peaks", "file", "s"),
        run("--set", "conv.peaks=file", "l"),
    );
    assert!(a.code == 0 && b.code == 0, "{} {}", a.stderr, b.stderr);
    assert_eq!(mzmls(&dir.join("s")), mzmls(&dir.join("l")));
    // the synthetic files hold centroids only: the profile and our centroids are asked in vain (never the maker's centroids written as ours), and
    // the cause is named, the same from the short form and from the parameter
    for (which, key) in [
        ("profile", "err.conv.noProfile"),
        ("mzlab", "err.conv.mzlabNoProfile"),
    ] {
        for (opt, val) in [
            ("--peaks", which.to_string()),
            ("--set", format!("conv.peaks={which}")),
        ] {
            let o = run(opt, &val, &format!("{which}{opt}"));
            assert!(
                o.code == 1 && o.stderr.contains(key),
                "{which} {opt}: {}",
                o.stderr
            );
        }
    }
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

#[test]
fn the_compression_kinds_are_the_short_forms_of_zlib_and_numpress() {
    let Some((dir, src)) = series("kinds", false) else {
        return;
    };
    let run = |name: &str, extra: &[&str]| {
        let out = dir.join(name);
        let mut a = vec![p(&src), "--out", p(&out)];
        a.extend_from_slice(extra);
        let o = convert(&a);
        assert_eq!(o.code, 0, "{}", o.stderr);
        mzmls(&out)
    };
    assert_eq!(run("std", &["--compression", "standard"]), run("def", &[]));
    assert_eq!(
        run("none", &["--compression", "none"]),
        run("z0", &["--zlib", "0"])
    );
    let small = run("small", &["--compression", "smallest"]);
    assert_eq!(small, run("np", &["--numpress"]));
    assert_ne!(small, run("std2", &[]));
    let o = convert(&[
        p(&src),
        "--out",
        p(&dir.join("bad")),
        "--compression",
        "tiny",
    ]);
    assert_eq!(o.code, 2, "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A method blob over the old 1 MiB head pushes `spectrumList` and `conv/1` far into the file: the report still reads both.
#[test]
fn the_report_reads_spectra_and_conv_past_a_large_method_blob() {
    let dir = std::env::temp_dir().join(format!("mzlab-head-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for blob in [0usize, 3 << 20] {
        let file = dir.join(format!("h{blob}.mzML"));
        let text = format!(
            "<?xml version=\"1.0\"?>\n<mzML><run id=\"r\"><userParam name=\"mzLab metodo/1\" value=\"{}\"/>\
             <userParam name=\"mzLab conv/1\" value=\"{{&quot;a&quot;:1}}\"/>\n<spectrumList count=\"7\" defaultDataProcessingRef=\"c\">\n\
             <spectrum index=\"0\"/></spectrumList></run></mzML>\n",
            "x".repeat(blob)
        );
        std::fs::write(&file, text).unwrap();
        let (n, conv, _) = crate::convert_report::head_of(&file);
        assert_eq!((n, conv.as_deref()), (7, Some("{\"a\":1}")), "blob {blob}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_files_are_converted_on_the_workers_of_the_computer_unless_workers_says_otherwise() {
    let args = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let auto = crate::convert::parse_args(args(&["a.raw", "--out", "x"]))
        .unwrap()
        .unwrap();
    assert_eq!(auto.workers, None);
    let wanted = mzlab_core::budget_auto::convert_wanted();
    assert_eq!(crate::convert::workers_for(8192.0, None), (wanted, None));
    let one = crate::convert::parse_args(args(&["a.raw", "--out", "x", "--workers", "1"]))
        .unwrap()
        .unwrap();
    assert_eq!(one.workers, Some(1));
    // --workers is kept even where the budget would lower the default, and says nothing
    assert_eq!(crate::convert::workers_for(100.0, Some(3)), (3, None));
}

#[test]
fn a_budget_too_small_for_the_default_converts_fewer_files_at_once_and_says_so() {
    let n = crate::convert::fewer_workers(256.0, 4).unwrap();
    assert_eq!(n.key, "warn.convert.fewerWorkers");
    assert_eq!(
        crate::error_text::warning(&n),
        "2 files are converted at once instead of 4, so that each conversion has its share of the memory budget of this computer (256 MB). The files written are the same; it only takes longer. (warn.convert.fewerWorkers)"
    );
    assert_eq!(crate::convert::fewer_workers(1024.0, 4), None);
    let (w, note) = crate::convert::workers_for(100.0, None);
    assert_eq!(w, 1);
    assert_eq!(
        note.is_some(),
        mzlab_core::budget_auto::convert_wanted() > 1
    );
}

/// Each entry of `metodo/1.avvisi` in the head of an mzML is a note with its catalogue text and place, and a key without a text keeps its place.
#[test]
fn the_method_warnings_are_notes_with_their_text() {
    let dir = folder("notes");
    let file = dir.join("n.mzML");
    let doc = "{&quot;schema&quot;:&quot;metodo/1&quot;,&quot;avvisi&quot;:[{&quot;chiave&quot;:&quot;meta.troncato&quot;,&quot;dove&quot;:&quot;scan 7&quot;},{&quot;chiave&quot;:&quot;meta.nuovo&quot;,&quot;dove&quot;:&quot;x&quot;}]}";
    let text = format!(
        "<?xml version=\"1.0\"?>\n<mzML><run id=\"r\"><userParam name=\"mzLab metodo/1\" value=\"{doc}\"/>\n<spectrumList count=\"0\" defaultDataProcessingRef=\"c\">\n</spectrumList></run></mzML>\n"
    );
    std::fs::write(&file, text).unwrap();
    let (_, _, notes) = crate::convert_report::head_of(&file);
    let lines: Vec<String> = notes
        .iter()
        .map(|(k, d)| crate::error_text::method_note(k, d))
        .collect();
    assert_eq!(
        lines,
        [
            "The file is cut short: scan 7. What comes before the cut is read. (meta.troncato)",
            "meta.nuovo: x"
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[path = "convert_partial_tests.rs"]
mod partial;

#[path = "convert_isolation_tests.rs"]
mod isolation;
