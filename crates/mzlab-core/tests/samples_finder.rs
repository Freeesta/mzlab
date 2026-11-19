//! `route_finder` (linked into the `table` .wasm) against `route` (the app).
use mzlab_core::samples;

/// `route_finder` (what the `table` .wasm links) gives the same bytes as `route` for its operations and refuses the others, which `rotte` serves.
#[test]
fn route_finder_same_bytes_as_route() {
    let files = r#"{"files":[{"name":"blank_01.mzML","sha256":"","kind":""},{"name":"qc_pool_02.mzML","sha256":"","kind":""}]}"#;
    let doc = samples::route("propose", files).unwrap();
    for (op, input) in [
        ("propose", files),
        ("validate", doc.as_str()),
        ("per_tp", doc.as_str()),
        ("canonical", doc.as_str()),
        ("digest", doc.as_str()),
    ] {
        assert_eq!(
            samples::route_finder(op, input).unwrap(),
            samples::route(op, input).unwrap(),
            "{op}"
        );
    }
    for op in ["propose_explained", "relink"] {
        assert_eq!(
            samples::route_finder(op, files).unwrap_err().key,
            "error.badSamples",
            "{op}"
        );
    }
}
