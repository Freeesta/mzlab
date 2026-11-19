//! `mzlab-cli flow report` on a run of the synthetic identification fixture: same bytes twice, and a byte changed in an input changes the SHA-256 reported.

use crate::flow::run_plan;
use mzlab_core::json::parse;

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"q.mgf","type":"spectra"},{"name":"lib.msp","type":"library"}],"steps":[
{"id":"m","step":"spectra.match@1","in":{"query":"file:q.mgf","library":"file:lib.msp"}}]}"#;

fn report(dir: &std::path::Path) -> String {
    let work = dir.join("work");
    let (_, prov) = run_plan(PLAN.as_bytes(), dir, &work, 1, None).unwrap();
    std::fs::write(work.join("prov.json"), prov).unwrap();
    crate::flow_report::report(&work).unwrap()
}

fn sha_of(text: &str, name: &str) -> String {
    let v = parse(text.as_bytes()).unwrap();
    let all = [
        v.get("inputs").unwrap().arr(),
        v.get("lists").unwrap().arr(),
    ]
    .concat();
    let f = all
        .iter()
        .find(|f| f.get("name").unwrap().str() == Some(name))
        .unwrap();
    f.get("sha256").unwrap().str().unwrap().to_string()
}

#[test]
fn the_report_is_stable_and_follows_the_inputs() {
    let dir = crate::flow_ident_tests::fixture("corsa");
    let a = report(&dir);
    // reading the same folder again gives the same bytes
    assert_eq!(a, crate::flow_report::report(&dir.join("work")).unwrap());
    assert_eq!(sha_of(&a, "q.mgf"), sha_of(&report(&dir), "q.mgf"));
    let v = parse(a.as_bytes()).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("run-report/1"));
    assert_eq!(
        v.get("steps").unwrap().arr()[0]
            .get("status")
            .unwrap()
            .str(),
        Some("ok")
    );
    let mut q = std::fs::read(dir.join("q.mgf")).unwrap();
    q[0] ^= 1;
    std::fs::write(dir.join("q.mgf"), q).unwrap();
    // the library keeps its hash, the changed query does not
    let b = report(&dir);
    assert_ne!(sha_of(&a, "q.mgf"), sha_of(&b, "q.mgf"));
    assert_eq!(sha_of(&a, "lib.msp"), sha_of(&b, "lib.msp"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_report_and_command_follow_the_contract() {
    let run = |a: &[&str]| crate::execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    for a in [
        &["flow", "report", "/nonexistent/run"][..],
        &["flow", "report", "/nonexistent/run", "--json"],
        &[
            "explain-missing",
            "/nonexistent/t.mzfeat",
            "/nonexistent/e.csv",
            "--json",
        ],
    ] {
        let o = run(a);
        assert_eq!(o.code, 4, "{a:?}: {}", o.stderr);
        let t = String::from_utf8(o.stdout).unwrap();
        assert!(
            t.starts_with("{\"schema\":\"error/1\"") && t.ends_with('\n'),
            "{t}"
        );
    }
    assert_eq!(run(&["flow", "report", "a", "--bogus"]).code, 2);
    let dir = std::env::temp_dir().join(format!("mzlab-cmdjson-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let plan = dir.join("plan.json");
    std::fs::write(&plan, PLAN).unwrap();
    let p = plan.to_str().unwrap();
    let text = String::from_utf8(run(&["flow", "command", p]).stdout).unwrap();
    let o = run(&["flow", "command", p, "--json"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["schema"], "flowcommand/1");
    assert_eq!(v["script"], text.as_str());
    std::fs::remove_dir_all(dir).ok();
}
