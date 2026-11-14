//! Files opened from outside (`src/opening.rs`): which arguments count, the queue, and the alternate notice «make mzLab the default app?».

use mzlab_desktop::libraries::{read_config, write_config, Config};
use mzlab_desktop::opening::{
    answer_notice, ask_again, paths_from_args, record_opening, NoticeAnswer, Pending,
};

fn s(x: &std::path::Path) -> String {
    x.to_string_lossy().into_owned()
}

#[test]
fn only_single_mzml_and_raw_files_are_taken() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    for n in ["a.mzML", "b.mzml", "c.RAW", "d.txt", "e.dam"] {
        std::fs::write(d.join(n), b"x").unwrap();
    }
    // a Waters/Agilent-like folder named like a file cannot be associated
    std::fs::create_dir(d.join("w.raw")).unwrap();
    let args: Vec<String> = [
        "--flag",
        "a.mzML",
        "b.mzml",
        "c.RAW",
        "d.txt",
        "e.dam",
        "w.raw",
        "missing.mzML",
    ]
    .iter()
    .map(|x| x.to_string())
    .collect();
    let got = paths_from_args(&args, Some(d));
    assert_eq!(
        got,
        vec![d.join("a.mzML"), d.join("b.mzml"), d.join("c.RAW")]
    );
    // an absolute path wins over the working directory
    let abs = vec![s(&d.join("a.mzML"))];
    assert_eq!(
        paths_from_args(&abs, Some(std::path::Path::new("/nowhere"))),
        vec![d.join("a.mzML")]
    );
    assert!(paths_from_args(&[], None).is_empty());
}

#[test]
fn second_launch_argv_skips_the_program_and_resolves_against_its_cwd() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    std::fs::write(d.join("x.mzML"), b"x").unwrap();
    // what the single-instance plugin hands over: the program name first, then the relative file
    let argv: Vec<String> = ["mzlab", "x.mzML"].iter().map(|x| x.to_string()).collect();
    assert_eq!(
        paths_from_args(argv.get(1..).unwrap_or_default(), Some(d)),
        vec![d.join("x.mzML")]
    );
    assert!(paths_from_args([].as_slice(), Some(d)).is_empty());
}

#[test]
fn pending_queue_keeps_order_and_is_emptied_once() {
    let q = Pending::default();
    q.push(["/x/a.mzML".into(), "/x/b.mzML".into()]);
    q.push(["/x/a.mzML".into()]);
    assert_eq!(q.take().len(), 2);
    assert!(q.take().is_empty());
}

#[test]
fn notice_comes_every_other_opening_until_a_final_answer() {
    let mut c = Config::default();
    let seq: Vec<bool> = (0..6).map(|_| record_opening(&mut c)).collect();
    assert_eq!(seq, [true, false, true, false, true, false]);
    // «No (not now)»: nothing changes
    answer_notice(&mut c, None);
    assert!(record_opening(&mut c));
    // «Don't ask again» stops for good, and so does «Yes»
    answer_notice(&mut c, Some(NoticeAnswer::Never));
    assert!((0..5).all(|_| !record_opening(&mut c)));
    ask_again(&mut c);
    assert_eq!((c.aperture, c.risposta), (0, None));
    assert!(record_opening(&mut c));
    answer_notice(&mut c, Some(NoticeAnswer::Yes));
    assert!((0..5).all(|_| !record_opening(&mut c)));
}

#[test]
fn counter_and_answer_survive_in_the_config_file() {
    let t = tempfile::tempdir().unwrap();
    let mut c = read_config(t.path());
    record_opening(&mut c);
    answer_notice(&mut c, Some(NoticeAnswer::Yes));
    write_config(t.path(), &c).unwrap();
    assert_eq!(read_config(t.path()), c);
    assert_eq!(read_config(t.path()).risposta, Some(NoticeAnswer::Yes));
    // a config.json written before this existed still reads
    std::fs::write(t.path().join("config.json"), r#"{"esterne":true}"#).unwrap();
    let old = read_config(t.path());
    assert!(old.esterne && old.aperture == 0 && old.risposta.is_none());
}

#[test]
fn accepted_extensions_come_from_the_readers_registry() {
    let all = mzlab_desktop::opening::file_extensions();
    // every single-file reader extension is there, folder-only ones are not
    for e in [
        ".mzml", ".mzxml", ".raw", ".wiff", ".qgd", ".lcd", ".cdf", ".nc",
    ] {
        assert!(all.iter().any(|x| x == e), "{e}");
    }
    assert!(!all.iter().any(|x| x == ".d"));
    let dlg = mzlab_desktop::opening::dialog_extensions();
    assert!(dlg.contains(&"MZXML".to_string()) && dlg.contains(&"mzxml".to_string()));
    assert!(dlg.iter().all(|x| !x.starts_with('.') && !x.contains('.')));
    assert!(dlg.contains(&"gz".to_string()));
}

/// `tauri.conf.json` (static) must list exactly the extensions derived from the registry.
#[test]
fn file_associations_in_tauri_conf_match_the_registry() {
    let conf =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tauri.conf.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&conf).unwrap();
    let mut got: Vec<String> = v["bundle"]["fileAssociations"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["ext"].as_array().unwrap().iter())
        .map(|e| e.as_str().unwrap().to_ascii_lowercase())
        .collect();
    got.sort();
    assert_eq!(got, mzlab_desktop::opening::associated_extensions());
}

#[test]
fn mzxml_wiff_and_mzml_gz_files_are_taken_but_not_other_gz() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    for n in ["a.mzXML", "b.wiff", "c.mzML.gz", "d.txt.gz"] {
        std::fs::write(d.join(n), b"x").unwrap();
    }
    let args: Vec<String> = ["a.mzXML", "b.wiff", "c.mzML.gz"]
        .iter()
        .map(|x| x.to_string())
        .collect();
    assert_eq!(
        paths_from_args(&args, Some(d)),
        vec![d.join("a.mzXML"), d.join("b.wiff"), d.join("c.mzML.gz")]
    );
}

fn touch(d: &std::path::Path, names: &[&str]) {
    for n in names {
        std::fs::write(d.join(n), b"x").unwrap();
    }
}

#[test]
fn wiff_scan_opens_its_wiff_ignoring_case_with_spaces_and_non_ascii() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    touch(d, &["Campióne 1.WIFF", "Campióne 1.WIFF.scan"]);
    let plan = mzlab_desktop::opening::plan_from_paths([d.join("Campióne 1.WIFF.scan")]);
    assert_eq!(
        plan.files,
        vec![d.join("Campióne 1.WIFF"), d.join("Campióne 1.WIFF.scan")]
    );
    assert!(plan.problems.is_empty());
    // the .wiff scan name in another case than its .wiff
    touch(d, &["b.wiff", "b.WIFF.SCAN"]);
    let plan = mzlab_desktop::opening::plan_from_paths([d.join("b.WIFF.SCAN")]);
    assert_eq!(plan.files, vec![d.join("b.wiff"), d.join("b.WIFF.SCAN")]);
}

#[test]
fn wiff_alone_takes_its_scan_companion_along_once() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    touch(d, &["a.wiff", "a.wiff.scan", "n.wiff"]);
    let plan = mzlab_desktop::opening::plan_from_paths([d.join("a.wiff"), d.join("a.wiff.scan")]);
    assert_eq!(plan.files, vec![d.join("a.wiff"), d.join("a.wiff.scan")]);
    // no companion: the .wiff alone, no message
    let plan = mzlab_desktop::opening::plan_from_paths([d.join("n.wiff")]);
    assert_eq!(plan.files, vec![d.join("n.wiff")]);
    assert!(plan.problems.is_empty());
}

#[test]
fn scan_without_wiff_or_foreign_scan_is_refused_with_a_message() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    touch(d, &["lost.wiff.scan", "other.scan", "ok.mzML"]);
    let plan = mzlab_desktop::opening::plan_from_paths([
        d.join("lost.wiff.scan"),
        d.join("other.scan"),
        d.join("ok.mzML"),
    ]);
    assert_eq!(plan.files, vec![d.join("ok.mzML")]);
    assert_eq!(plan.problems.len(), 2);
    assert!(plan.problems[0].contains("«lost.wiff» is missing"));
    assert!(plan.problems[1].contains("not a SCIEX scan companion"));
}

#[test]
fn scan_is_associated_but_other_companions_and_nc_are_not() {
    let a = mzlab_desktop::opening::associated_extensions();
    assert!(a.contains(&"scan".to_string()) && a.contains(&"wiff".to_string()));
    assert!(!a.contains(&"nc".to_string()) && !a.contains(&"gz".to_string()));
}
