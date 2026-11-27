//! `modes::type_label`: the text the sample list shows for each scan-type code of the table.

use super::modes::{type_label, MODI};

#[test]
fn every_scan_type_code_has_the_label_the_sample_list_shows() {
    let want = [
        (0, "Full scan"),
        (1, "SIM"),
        (2, "Full scan"),
        (3, "SIM"),
        (4, "MRM"),
        (5, "Precursor ion"),
        (6, "Product ion"),
        (7, "Neutral loss"),
        (8, "Full scan"),
        (9, "Product ion"),
        (10, "Full scan"),
        (11, "Enhanced product ion"),
        (12, "Enhanced resolution"),
        (13, "MS3"),
        (14, "Time-delayed fragmentation"),
        (15, "Full scan"),
        (16, "Enhanced multiply charged"),
    ];
    for (code, label) in want {
        assert_eq!(type_label(Some(code)), label, "code {code}");
    }
    assert_eq!(MODI.len(), want.len());
    assert_eq!(type_label(Some(99)), "Unknown");
    assert_eq!(type_label(None), "Unknown");
}
