//! `flow record` and `flow verify` on a real run of the synthetic base chain: the record verifies, the same run gives the same bytes, a changed byte is named.
//! The chains need a build that names its commit (a git checkout, or `MZLAB_COMMIT` at build time): `flow run` writes it, `flow record` needs it.

use crate::flow_run_tests::{fixture, flow_run, p};
use mzlab_core::budget::sha256_hex;

/// When `MZLAB_JSON_SAMPLES` names a folder the document is left there for `tests/test_cli_json.py`.
fn sample(name: &str, bytes: &[u8]) {
    if let Some(dir) = std::env::var_os("MZLAB_JSON_SAMPLES") {
        std::fs::write(std::path::Path::new(&dir).join(name), bytes).unwrap();
    }
}

fn cli(args: &[&str]) -> crate::Outcome {
    crate::execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

/// A run folder as `flow run` leaves it (the commit is the one of this build, never patched in) and the `samples/1` that gives every file its role;
/// `hashed` false leaves each SHA-256 empty, as `sample table --names` does.
fn run_folder(tag: &str, hashed: bool) -> (std::path::PathBuf, std::path::PathBuf) {
    let (dir, files) = fixture(tag);
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let o = flow_run(&[p(&plan), "--in", p(&data), "--out", p(&out)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let list: Vec<mzlab_core::samples::Sample> = files
        .iter()
        .enumerate()
        .map(|(k, (n, b))| {
            let sha = if hashed { sha256_hex(b) } else { String::new() };
            mzlab_core::samples::Sample::new(n, &sha, k as u32 + 1)
        })
        .collect();
    let roles = dir.join("roles.json");
    std::fs::write(&roles, mzlab_core::samples::to_json(&list)).unwrap();
    (out, roles)
}

#[test]
fn a_record_of_a_run_verifies_and_is_the_same_bytes_twice() {
    let (out, roles) = run_folder("mzrun_a", true);
    let rec = |extra: &[&str]| {
        let mut a = vec!["flow", "record", p(&out), "--roles", p(&roles)];
        a.extend(extra);
        cli(&a)
    };
    let (a, b) = (rec(&[]), rec(&[]));
    assert_eq!(a.code, 0, "{}", a.stderr);
    assert_eq!(a.stdout, b.stdout);
    let file = out.join("run.mzrun.json");
    assert_eq!(rec(&["--out", p(&file)]).code, 0);
    assert_eq!(std::fs::read(&file).unwrap(), a.stdout);
    let v = cli(&["flow", "verify", p(&file)]);
    assert_eq!((v.code, v.stdout.len()), (0, 0), "{}", v.stderr);
    let text = String::from_utf8(a.stdout).unwrap();
    std::fs::write(
        &file,
        text.replacen("\"revisions\":0", "\"revisions\":1", 1),
    )
    .unwrap();
    let v = cli(&["flow", "verify", p(&file)]);
    assert_eq!(
        (v.code, v.stdout.len(), v.stderr.as_str()),
        (3, 0, "self_sha256\n")
    );
    let j = cli(&["flow", "verify", p(&file), "--json"]);
    let j_out = String::from_utf8(j.stdout).unwrap();
    assert_eq!(j.code, 3);
    // one error/1 (code 3) that carries the verify/1 report
    assert!(
        j_out.starts_with("{\"schema\":\"error/1\",\"code\":3,"),
        "{j_out}"
    );
    let report = j_out
        .split_once("\"report\":")
        .unwrap()
        .1
        .strip_suffix("}\n")
        .unwrap();
    sample("verify.json", report.as_bytes());
    assert!(
        report.starts_with("{\"schema\":\"verify/1\",\"ok\":false,"),
        "{report}"
    );
    assert!(
        report.ends_with("\"problems\":[\"self_sha256\"]}"),
        "{report}"
    );
    let _ = std::fs::remove_dir_all(out.parent().unwrap());
}

#[test]
fn usage_mistakes_exit_2() {
    for a in [
        &["flow", "record"][..],
        &["flow", "record", "x"],
        &["flow", "verify"],
        &["flow", "verify", "/nope.json"],
    ] {
        assert_eq!(cli(a).code, crate::exit::usage_or_missing(a), "{a:?}");
    }
}

#[test]
fn roles_without_sha256_take_it_from_the_run_and_the_record_verifies() {
    let (out, roles) = run_folder("mzrun_b", false);
    let file = out.join("run.mzrun.json");
    let r = cli(&[
        "flow",
        "record",
        p(&out),
        "--roles",
        p(&roles),
        "--out",
        p(&file),
    ]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert!(r.stderr.contains("(warn.mzrun.roleSha)"), "{}", r.stderr);
    let v = cli(&["flow", "verify", p(&file), "--json"]);
    assert_eq!(v.code, 0, "{}", v.stderr);
    assert!(String::from_utf8(v.stdout)
        .unwrap()
        .starts_with("{\"schema\":\"verify/1\",\"ok\":true,"));
    let _ = std::fs::remove_dir_all(out.parent().unwrap());
}

#[test]
fn samples_table_on_real_files_gives_roles_that_record_accepts() {
    let (dir, _) = fixture("mzrun_c");
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    assert_eq!(
        flow_run(&[p(&plan), "--in", p(&data), "--out", p(&out)]).code,
        0
    );
    let mut paths: Vec<String> = std::fs::read_dir(&data)
        .unwrap()
        .map(|e| e.unwrap().path().to_string_lossy().into_owned())
        .collect();
    paths.sort();
    let mut args = vec!["sample", "table"];
    args.extend(paths.iter().map(String::as_str));
    let t = cli(&args);
    assert_eq!(t.code, 0, "{}", t.stderr);
    let table = String::from_utf8(t.stdout).unwrap();
    assert!(
        !table.contains("\"sha256\":\"\""),
        "every file is hashed: {table}"
    );
    let roles = dir.join("table.json");
    std::fs::write(&roles, table).unwrap();
    let r = cli(&["flow", "record", p(&out), "--roles", p(&roles)]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_role_that_matches_no_input_is_refused_with_the_names_the_run_has() {
    let (out, roles) = run_folder("mzrun_d", false);
    let text = std::fs::read_to_string(&roles).unwrap();
    let first = mzlab_core::samples::from_json(text.as_bytes()).unwrap()[0]
        .file
        .clone();
    std::fs::write(&roles, text.replacen(&first, "nothing.mzML", 1)).unwrap();
    let r = cli(&["flow", "record", p(&out), "--roles", p(&roles)]);
    assert_eq!(r.code, 3);
    assert!(
        r.stderr.contains("nothing.mzML") && r.stderr.contains(&first),
        "{}",
        r.stderr
    );
    let _ = std::fs::remove_dir_all(out.parent().unwrap());
}

#[test]
fn a_run_made_without_a_commit_is_refused_with_the_key_and_the_hint() {
    let (out, roles) = run_folder("mzrun_e", true);
    let prov = std::fs::read_to_string(out.join("prov.json")).unwrap();
    // a build with a commit has the key to take out; one without (no git, no MZLAB_COMMIT) already has none
    if let Some(start) = prov.find("\"commit\":\"") {
        let end = start + prov[start..].find(',').unwrap() + 1;
        let text = format!("{}{}", &prov[..start], &prov[end..]);
        std::fs::write(out.join("prov.json"), text).unwrap();
    }
    let r = cli(&["flow", "record", p(&out), "--roles", p(&roles)]);
    assert_eq!(r.code, 3);
    assert!(
        r.stderr.contains("(err.mzrun.commit)") && r.stderr.contains("MZLAB_COMMIT"),
        "{}",
        r.stderr
    );
    let _ = std::fs::remove_dir_all(out.parent().unwrap());
}
