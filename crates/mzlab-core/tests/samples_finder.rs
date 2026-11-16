//! `route_finder` (linked into the `table` .wasm) against `route` (the app).
use mzlab_core::samples;

/// `route_finder` (what the `table` .wasm links) gives the same bytes as `route` for its operations and refuses the others, which `rotte` serves.
#[test]
fn route_finder_same_bytes_as_route() {
    let files = r#"{"files":[{"name":"blank_01.mzML","sha256":"","kind":""},{"name":"qc_pool_02.mzML","sha256":"","kind":""}]}"#;
    let doc = samples::route("proponi", files).unwrap();
    let doc3 = samples::route("proponi3", files).unwrap();
    for (op, input) in [
        ("proponi", files),
        ("valida", doc.as_str()),
        ("per_tp", doc.as_str()),
        ("canonico", doc.as_str()),
        ("digest", doc.as_str()),
        ("canonico", doc3.as_str()),
        ("digest", doc3.as_str()),
    ] {
        assert_eq!(
            samples::route_finder(op, input).unwrap(),
            samples::route(op, input).unwrap(),
            "{op}"
        );
    }
    assert!(samples::route("migra", &doc).is_ok());
    for op in ["proponi_spiegato", "proponi3", "ricollega", "migra"] {
        assert_eq!(
            samples::route_finder(op, files).unwrap_err().key,
            "error.badCampioni",
            "{op}"
        );
    }
}
