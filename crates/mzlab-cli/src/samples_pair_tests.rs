//! `sample pair`: the pairing check, its exit codes and its document.

use mzlab_core::json::parse;

/// The call of the contract test of missing inputs: a samples file that is not there.
pub(crate) const MISSING: &[&[&str]] = &[&["sample", "pair", "/nonexistent/s.json", "x.mzML"]];

fn run(args: &[&str]) -> (u8, String, String) {
    let o = crate::execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    (
        o.code,
        String::from_utf8(o.stdout).unwrap(),
        o.stderr,
    )
}

fn samples_file(dir: &std::path::Path, names: &[&str]) -> String {
    let files: Vec<_> = names
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    let path = dir.join("samples.json");
    std::fs::write(&path, mzlab_core::samples::to_json(&mzlab_core::samples::propose(&files))).unwrap();
    path.to_string_lossy().into_owned()
}

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_pair_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Exit 0 for files that pair as written, 1 with the choices (a left-out sample, a role from a name) as `pairing` and on stderr, both with the samples/1 on stdout.
#[test]
fn the_check_says_every_choice_and_ends_with_its_own_code() {
    let d = dir("a");
    let s = samples_file(&d, &["S_t0.mzML", "S_t10.mzML", "S_std.mzML"]);
    let (code, out, err) = run(&["sample", "pair", &s, "S_t0.mzML", "S_t10.mzML", "S_std.mzML", "--json"]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    let v = parse(out.as_bytes()).unwrap();
    assert_eq!(v.get("status").and_then(|x| x.str()), Some("complete"));
    assert!(v.get("pairing").unwrap().arr().is_empty());
    // a standard the series lacks and a dark control it has but the samples do not
    let (code, out, err) = run(&["sample", "pair", &s, "d/S_t0.mzML", "d/S_t10.mzML", "d/S_darkcontrol.mzML", "--json"]);
    assert_eq!(code, 1, "{out}");
    let v = parse(out.as_bytes()).unwrap();
    assert_eq!(v.get("status").and_then(|x| x.str()), Some("adjusted"));
    let got: Vec<_> = v
        .get("pairing")
        .unwrap()
        .arr()
        .iter()
        .map(|n| {
            let t = |k: &str| n.get(k).and_then(|x| x.str()).unwrap().to_string();
            (t("action"), t("file"), t("role"), t("evidence"))
        })
        .collect();
    assert_eq!(
        got,
        [
            ("left_out".into(), "S_std.mzML".into(), "std".into(), "".into()),
            ("proposed".into(), "S_darkcontrol.mzML".into(), "dark".into(), "darkcontrol".into()),
        ]
    );
    assert!(err.contains("S_std.mzML") && err.contains("S_darkcontrol.mzML") && err.contains("darkcontrol"), "{err}");
    // without --json the paired samples/1 alone
    let (_, bare, _) = run(&["sample", "pair", &s, "S_t0.mzML", "S_t10.mzML", "S_darkcontrol.mzML"]);
    assert_eq!(parse(bare.as_bytes()).unwrap().get("samples").unwrap().arr().len(), 3);
}

/// The failure modes keep their codes: usage 2, a samples file that is not a samples/1 (a `sample-import/1` here) 3 with the hint naming the commands, a missing file 4.
#[test]
fn the_failure_modes_have_distinct_pinned_codes() {
    let d = dir("b");
    let s = samples_file(&d, &["S_t0.mzML"]);
    assert_eq!(run(&["sample", "pair", &s]).0, 2);
    assert_eq!(run(&["sample", "pair", &s, "x.mzML", "--table", "t.mzfeat"]).0, 2);
    let wrong = d.join("import.json");
    std::fs::write(&wrong, "{\"schema\":\"sample-import/1\",\"samples\":{},\"report\":{}}").unwrap();
    let (code, out, err) = run(&["sample", "pair", wrong.to_str().unwrap(), "S_t0.mzML", "--json"]);
    assert_eq!(code, 3, "{out}");
    assert!(err.contains("sample table") && err.contains("sample import"), "{err}");
    assert_eq!(parse(out.lines().next().unwrap().as_bytes()).unwrap().get("error_key").and_then(|x| x.str()), Some("error.badSamples"));
    assert_eq!(run(&["sample", "pair", "/no/such/samples.json", "S_t0.mzML"]).0, 4);
    assert_eq!(run(&["sample", "pair", &s, "--table", "/no/such/t.mzfeat"]).0, 4);
}
