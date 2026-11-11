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
        ..Default::default()
    });
    assert_eq!(w.len(), 2);
    assert!(w[0].contains(r#""meta.ioneScartata""#) && w[0].contains("in 1 of 4 scans"));
    assert!(w[1].contains(r#""meta.daTabella""#) && w[1].contains("of 4 of 4 scans"));
}

/// What the reader passes on unverified or refuses is named with its count: an MS order not decoded, events found by number in a cut table, profiles
/// whose calibration no known conversion uses.
#[test]
fn unverified_and_refused_scans_are_named_with_their_counts() {
    let w: Vec<String> = scan_warnings(&crate::raw::ScanNotes {
        scans: 9,
        with_event: 9,
        order_unknown: 2,
        event_by_class: 5,
        calibration_unknown: 3,
        ..Default::default()
    })
    .into_iter()
    .flatten()
    .map(|v| v.json())
    .collect();
    assert_eq!(w.len(), 3);
    assert!(w.iter().all(|t| t.contains(r#""meta.nonDecodificato""#)));
    assert!(w[0].contains("MS order of 2 of 9 scans") && w[0].contains("level is unknown"));
    assert!(w[1].contains("5 of 9 scans") && w[1].contains("unchecked"));
    assert!(w[2].contains("3 of 9 scans") && w[2].contains("refused"));
}
