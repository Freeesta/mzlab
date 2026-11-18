//! The keys of `mzlab-cli channels` are exactly those `docs/schemi/channels-1.json` documents (each with its unit).

use mzlab_core::json::{parse, Value};

use crate::execute;

fn keys(o: &Value) -> Vec<String> {
    o.entries().iter().map(|e| e.0.clone()).collect()
}

fn props(v: &Value) -> Vec<String> {
    keys(v.get("properties").unwrap())
}

#[test]
fn every_channel_key_is_in_the_schema() {
    let schema = parse(include_bytes!("../../../docs/schemi/channels-1.json")).unwrap();
    let dir = std::env::temp_dir().join(format!("mzlab_cli_chkeys_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("a.mzML");
    std::fs::write(&f, crate::tests::mzml(10.0)).unwrap();
    let o = execute(&["channels".into(), f.to_string_lossy().into_owned()]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = parse(&o.stdout).unwrap();
    assert_eq!(keys(&v), props(&schema));
    let top = schema.get("properties").unwrap();
    assert_eq!(
        keys(v.get("kind").unwrap()),
        props(top.get("kind").unwrap())
    );
    let item = top.get("channels").unwrap().get("items").unwrap();
    for c in v.get("channels").unwrap().arr() {
        assert_eq!(keys(c), props(item));
    }
    let _ = std::fs::remove_dir_all(&dir);
}
