//! `flow record` and `flow verify` on a real run of the synthetic base chain: the record verifies, the same run gives the same bytes, a changed byte is named.

use crate::flow_run_tests::{fixture, flow_run, p};
use mzlab_core::budget::sha256_hex;

fn cli(args: &[&str]) -> crate::Outcome {
    crate::execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

/// A run folder whose `prov.json` names a commit (a test build has none) and the `campioni/2` that gives every file its role.
fn run_folder(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let (dir, files) = fixture(tag);
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let o = flow_run(&[p(&plan), "--in", p(&data), "--out", p(&out)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let prov = std::fs::read_to_string(out.join("prov.json")).unwrap();
    let with = prov.replacen(
        "\"platform\":",
        &format!("\"commit\":\"{}\",\"platform\":", "1".repeat(40)),
        1,
    );
    assert_ne!(with, prov, "the agent has a platform key");
    std::fs::write(out.join("prov.json"), with).unwrap();
    let list: Vec<String> = files
        .iter()
        .enumerate()
        .map(|(k, (n, b))| {
            format!(
                "{{\"file\":\"{n}\",\"sha256\":\"{}\",\"tipo\":\"sample\",\"ordine_iniezione\":{},\"replica\":1,\"serie\":\"\",\"tempo\":{},\"unita\":\"min\",\"tempo_confermato\":true}}",
                sha256_hex(b),
                k + 1,
                [0, 10, 30][k]
            )
        })
        .collect();
    let roles = dir.join("roles.json");
    std::fs::write(
        &roles,
        format!(
            "{{\"schema\":\"campioni/2\",\"campioni\":[{}]}}",
            list.join(",")
        ),
    )
    .unwrap();
    (out, roles)
}

#[test]
fn a_record_of_a_run_verifies_and_is_the_same_bytes_twice() {
    let (out, roles) = run_folder("mzrun_a");
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
        (v.code, String::from_utf8(v.stdout).unwrap()),
        (3, "self_sha256\n".to_string())
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
        assert_eq!(cli(a).code, 2, "{a:?}");
    }
}
