//! A hypotheses registry of another format or version is refused with a stable key, never converted.

use mzlab_core::json::parse;
use mzlab_feat::hypo::check_registry_format;

#[test]
fn the_old_registry_format_is_refused() {
    for text in [
        r#"{"format":"ipotesi/1","hypotheses":[]}"#,
        r#"{"hypotheses":[]}"#,
    ] {
        let v = parse(text.as_bytes()).unwrap();
        assert_eq!(
            check_registry_format(&v).unwrap_err(),
            "error.hypo.registryFormat"
        );
    }
}

#[test]
fn the_current_registry_format_is_accepted() {
    let v = parse(br#"{"format":"hypotheses/2","hypotheses":[]}"#).unwrap();
    assert!(check_registry_format(&v).is_ok());
}

/// `run_to` is the one call behind the page (`.wasm`) and the CLI (`flow_link`): the refusal is the same error key on both routes.
#[cfg(feature = "interp")]
#[test]
fn the_interpretation_refuses_an_old_registry_with_the_new_format_key() {
    use mzlab_core::error::Result;
    use mzlab_core::idxread::Index;
    use mzlab_feat::interp::run::run_to;
    use mzlab_feat::store::Slice;
    let parent = r#"{"neutral":"C6H12O6","adduct":"[M+H]+","rt":1}"#;
    let old = r#"{"format":"ipotesi/1","hypotheses":[]}"#;
    let mut table = Slice(&[][..]);
    let mut sink = Vec::new();
    let mut open = |_k: usize| -> Result<Index<std::fs::File>> { unreachable!() };
    let err = run_to(&mut table, 0, parent, old, 1, &mut open, None, &mut sink).unwrap_err();
    assert_eq!(err.key, "error.hypo.registryFormat");
}
