//! Accent folding and the raw-HTML safety scan keep their results (they were sped up without changing them).
use mzlab_libro::{convert, norm, Options};

fn raw(body: &str) -> Result<String, String> {
    let md = format!("# T\n\n```html\n{body}\n```\n");
    let opt = Options {
        teoria: true,
        raw_ok: true,
        ..Options::default()
    };
    convert(&md, "01-p.md", &opt).map_err(|e| e.to_string())
}

#[test]
fn norm_folds_accents_and_case() {
    assert_eq!(norm("  Èà ÀÉ\tñÑ żŽ ǅ Perché  "), "ea ae nn zz ǆ perche");
    assert_eq!(norm("m/z [M+H]+ 13C"), "m/z [m+h]+ 13c");
}

#[test]
fn raw_html_scan_still_refuses_and_accepts() {
    assert!(raw("<p>perché è così</p>").is_ok());
    for bad in [
        "<p>è</p>\n<script>x</script>",
        "<a href=\"JavaScript:x\">è</a>",
        "<p onclick=\"x\">è</p>",
        "< iframe src=x>",
    ] {
        assert!(raw(bad).is_err(), "{bad} must be refused");
    }
    assert!(raw("<p>il javascript non c'è</p>").is_ok());
}
