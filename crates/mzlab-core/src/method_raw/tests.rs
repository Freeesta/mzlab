//! The tests of method_raw.rs that need no file.
use super::*;

/// The notes of the scans become one warning each, with their counts; none when nothing was put aside or taken from the method.
#[test]
fn the_scan_notes_are_warnings_with_their_counts() {
    let json = |n: crate::raw::ScanNotes| {
        scan_warnings(&n)
            .into_iter()
            .flatten()
            .map(|v| v.json())
            .collect::<Vec<_>>()
    };
    let none = crate::raw::ScanNotes {
        scans: 4,
        with_event: 4,
        ..Default::default()
    };
    assert!(json(none).is_empty());
    let w = json(crate::raw::ScanNotes {
        scans: 4,
        with_event: 0,
        ion_rejected: 1,
        srm_from_table: 4,
    });
    assert_eq!(w.len(), 2);
    assert!(w[0].contains(r#""meta.ioneScartata""#) && w[0].contains("in 1 of 4 scans"));
    assert!(w[1].contains(r#""meta.daTabella""#) && w[1].contains("of 4 of 4 scans"));
}
