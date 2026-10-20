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
    assert_eq!(v.get("schema").unwrap().str(), Some("corsa/1"));
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
