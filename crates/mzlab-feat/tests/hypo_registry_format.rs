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
