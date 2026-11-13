//! A `data.ms` with a damaged header count or record (synthetic, written by `agilent_synth::datams`, no laboratory data): every whole record is read,
//! a record that does not frame ends the read and the scans after it are counted, a file that ends inside a record is a cut copy.

use mzlab_core::agilent::{AgilentD, Files};

mod agilent_synth;
use agilent_synth::datams::encode_data_ms;

#[test]
fn a_data_ms_count_below_its_records_reads_them_all() {
    let bytes = encode_data_ms(&[(60000, vec![(1019, 10)]), (60300, vec![(1680, 20)])]);
    // a header that declares fewer scans than the whole records the file holds: every whole record is read, and the header count is said
    let mut fewer = bytes.clone();
    fewer[0x116..0x11A].copy_from_slice(&1u32.to_be_bytes());
    let d = AgilentD::open(
        "old.D",
        &Files {
            data_ms: Some(&fewer),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!((d.records.len(), d.missing, d.damaged), (2, 0, 0));
    assert!(d
        .metadata
        .iter()
        .any(|m| m.name == "scans declared by the header" && m.value == "1"));
}

#[test]
fn a_data_ms_record_that_does_not_frame_ends_the_read() {
    let bytes = encode_data_ms(&[
        (60000, vec![(1019, 10)]),
        (60300, vec![(1680, 20)]),
        (60600, vec![(1700, 30)]),
    ]);
    // the second record's length is broken: the next one cannot be found, the first is read and the other two are counted
    let words = (24 + 4 * 2) / 2;
    let mut broken = bytes.clone();
    let second = 0x140 + 2 * words;
    broken[second..second + 2].copy_from_slice(&999u16.to_be_bytes());
    let d = AgilentD::open(
        "old.D",
        &Files {
            data_ms: Some(&broken),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!((d.records.len(), d.missing, d.damaged), (1, 0, 2));
    // the file ends inside the third record (the encoder adds 20 bytes after the records): a cut copy
    let d = AgilentD::open(
        "old.D",
        &Files {
            data_ms: Some(&bytes[..bytes.len() - 20 - 3]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!((d.records.len(), d.missing, d.damaged), (2, 1, 0));
}
