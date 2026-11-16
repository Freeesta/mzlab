//! The notice and the reasons of a `.wiff` whose index lacks sections or lists a sample with nothing to show.

#[test]
fn sections_the_wiff_index_lacks_are_one_calm_notice_per_dataset() {
    assert_eq!(crate::convert::unindexed_sections(0, 0), None);
    let n = crate::convert::unindexed_sections(2, 1).unwrap();
    assert_eq!(n.key, "warn.wiff.unindexed");
    let text = crate::error_text::warning(&n);
    assert!(
        text.contains(": 3;")
            && text
                .contains("samples, with a name and method inferred from the previous sample: 2;")
            && text.contains("counted once): 1."),
        "{text}"
    );
}

/// The reasons of a sample with nothing to show are catalogue texts, the ones the site shows: present, non-empty, and never the bare
/// engine fallbacks of the progress line (a length is no proof of that, the catalogue sentences are meant to be short).
#[test]
fn the_reasons_of_an_empty_wiff_sample_are_catalogue_texts() {
    for key in ["warn.wiff.emptySample", "warn.wiff.inferredSample"] {
        let text = crate::error_text::catalog_text(key).unwrap_or_default();
        assert!(!text.trim().is_empty(), "{key}");
        assert!(
            !["no scans", "an empty injection"].contains(&text.trim()),
            "{key}: {text}"
        );
    }
}
