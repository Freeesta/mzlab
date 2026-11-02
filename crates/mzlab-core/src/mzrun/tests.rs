use super::*;
use crate::campioni::Campione;
use crate::json::parse;

const EXAMPLE: &str = include_str!("../../../../tests/data/mzrun/record.mzrun.json");

fn sha(c: char) -> String {
    c.to_string().repeat(64)
}

fn sample(file: &str, sha: &str, order: u32) -> Campione {
    Campione::new(file, sha, order)
}

fn parts_text() -> (String, String, Vec<Campione>) {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"S01","sha256":"{}","bytes":10,"type":"mzml"}}],"steps":[{{"id":"detect","step":"features.detect@1","in":{{"source":["file:S01"]}}}}]}}"#,
        sha('a')
    );
    let prov = format!(
        r#"{{"schema":"prov/1","agent":{{"name":"mzLab","version":"1.0.0","commit":"{}","platform":"native"}},"entity":{{"file:S01":{{"sha256":"{}","bytes":10,"type":"mzml"}},"detect.peaks[0]":{{"sha256":"{}","bytes":5,"type":"peaks"}}}},"activity":{{"detect":{{"step":"features.detect@1","algorithm":"masstrace@1","params":{{"ppm":5,"min_scans":3,"seed":7}},"estimates":{{"min_scans":{{"basis":"x"}}}},"params_sha256":"{}"}}}}}}"#,
        "1".repeat(40),
        sha('a'),
        sha('b'),
        sha('c')
    );
    (plan, prov, vec![sample("S01.mzML", &sha('a'), 1)])
}

fn built() -> String {
    let (plan, prov, roles) = parts_text();
    let (plan, prov) = (
        parse(plan.as_bytes()).unwrap(),
        parse(prov.as_bytes()).unwrap(),
    );
    build(&Parts {
        plan: &plan,
        prov: &prov,
        roles: &roles,
        resources: vec![],
        decisions: vec![],
        limits: vec![],
    })
    .unwrap()
}

#[test]
fn the_example_of_the_schema_verifies() {
    let v = verify(EXAMPLE.as_bytes()).unwrap();
    assert_eq!(v.problems, Vec::<String>::new());
}

#[test]
fn the_digest_is_over_sorted_name_tab_hash_lines() {
    let e = [("b".to_string(), sha('2')), ("a".to_string(), sha('1'))];
    let text = format!("a\t{}\nb\t{}\n", sha('1'), sha('2'));
    assert_eq!(
        results_digest(&e),
        crate::budget::sha256_hex(text.as_bytes())
    );
}

#[test]
fn a_built_record_verifies_and_is_the_same_bytes_twice() {
    let a = built();
    assert_eq!(a, built());
    assert!(a.ends_with("}\n"));
    assert_eq!(verify(a.as_bytes()).unwrap().problems, Vec::<String>::new());
    let v = parse(a.as_bytes()).unwrap();
    assert_eq!(
        v.get("seeds").unwrap().get("detect.seed").unwrap().f64(),
        Some(7.0)
    );
    let pin = v
        .get("resolved")
        .unwrap()
        .get("detect")
        .unwrap()
        .get("pin")
        .unwrap();
    assert_eq!(pin.get("min_scans").unwrap().str(), Some("estimate"));
    assert_eq!(pin.get("ppm").unwrap().str(), Some("fixed"));
    assert_eq!(
        v.get("results")
            .unwrap()
            .get("entities")
            .unwrap()
            .entries()
            .len(),
        1
    );
}

#[test]
fn a_changed_byte_names_what_no_longer_matches() {
    let a = built();
    // the first one is the hash of the entity inside the prov/1
    let t = a.replacen(&sha('b'), &sha('d'), 1);
    let p = verify(t.as_bytes()).unwrap().problems;
    assert_eq!(
        p,
        ["self_sha256", "entity:detect.peaks[0]"].map(String::from)
    );
    let t = a.replace(&sha('b'), &sha('d'));
    assert_eq!(
        verify(t.as_bytes()).unwrap().problems,
        ["self_sha256", "results_digest"].map(String::from)
    );
    let t = a.replace("\"ppm\":5", "\"ppm\":6");
    assert_eq!(
        verify(t.as_bytes()).unwrap().problems,
        ["self_sha256".to_string()]
    );
}

#[test]
fn a_newer_or_foreign_file_is_refused() {
    let t = built().replace("\"min_reader\":1", "\"min_reader\":2");
    assert_eq!(verify(t.as_bytes()).unwrap_err().key, "error.newerFormat");
    assert_eq!(
        verify(b"{\"schema\":\"x\"}").unwrap_err().key,
        "err.mzrun.read"
    );
}

#[test]
fn an_input_without_a_role_or_a_run_without_a_commit_does_not_build() {
    let (plan, prov, _) = parts_text();
    let (p, v) = (
        parse(plan.as_bytes()).unwrap(),
        parse(prov.as_bytes()).unwrap(),
    );
    let mk = |v: &Value, roles: &[Campione]| {
        build(&Parts {
            plan: &p,
            prov: v,
            roles,
            resources: vec![],
            decisions: vec![],
            limits: vec![],
        })
    };
    assert_eq!(mk(&v, &[]).unwrap_err().key, "err.mzrun.build");
    let nc = parse(prov.replace(&"1".repeat(40), "").as_bytes()).unwrap();
    assert_eq!(
        mk(&nc, &[sample("S01.mzML", &sha('a'), 1)])
            .unwrap_err()
            .key,
        "err.mzrun.build"
    );
}
