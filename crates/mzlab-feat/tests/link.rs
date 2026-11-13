//! `link/1` against the independent reference `tools/link_reference.py` (Python `urllib.parse.quote`): the same bytes for every case of
//! `tests/dati/link_golden.json` (charges, brackets, stereo, isotopes, `#`, `+` and `/`).
use mzlab_feat::chem::link::link_json;
use serde_json::Value;

#[test]
fn every_golden_case_is_the_reference_byte_for_byte() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../../tests/dati/link_golden.json")).unwrap();
    assert!(cases.len() >= 8);
    for c in &cases {
        let want = c["link"].as_str().unwrap();
        assert_eq!(
            link_json(&c["request"].to_string()).unwrap(),
            want,
            "{}",
            c["request"]
        );
    }
}
