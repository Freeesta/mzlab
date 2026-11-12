//! The checks of `DDERealTimeData` rows: a stream that does not fit the scans and a row that is not a plausible precursor give none, never a guess.

use super::*;

fn record(cycle: usize, experiment: usize) -> Record {
    Record {
        at: 0,
        end: 0,
        size: 10,
        time_min: 0.0,
        tic: 0.0,
        cycle,
        experiment,
        points: f64::NAN,
        precursor: None,
        charge: None,
    }
}

fn entry(mz: f64, z: u16) -> [u8; ROW] {
    let mut r = [0u8; ROW];
    r[..8].copy_from_slice(&mz.to_le_bytes());
    r[16..24].copy_from_slice(&1e5f64.to_le_bytes());
    r[24..26].copy_from_slice(&z.to_le_bytes());
    r
}

fn stream(rows: &[[u8; ROW]]) -> Vec<u8> {
    let mut b = vec![0u8; HEAD];
    rows.iter().for_each(|r| b.extend_from_slice(r));
    b
}

/// A stream one byte short of its last row.
fn cut(mut b: Vec<u8>) -> Vec<u8> {
    b.pop();
    b
}

/// Three cycles of survey + one EPI.
fn records() -> Vec<Record> {
    (0..3).flat_map(|c| [record(c, 0), record(c, 1)]).collect()
}

#[test]
fn a_stream_with_one_row_per_dependent_slot_is_read() {
    let b = stream(&[entry(493.27, 1), entry(547.30, 2), entry(687.85, 0)]);
    let rs = records();
    let t = rows(&b, 2, &rs);
    let at = |k: usize| t.and_then(|t| row(t, k));
    assert_eq!(t.map(<[u8]>::len), Some(3 * ROW));
    assert_eq!(slot(&record(2, 1), 2), 2);
    assert_eq!(
        at(1).and_then(|r| plausible(r, None)),
        Some((547.30, Some(2)))
    );
    assert_eq!(at(2).and_then(|r| plausible(r, None)), Some((687.85, None)));
    assert!(at(3).is_none());
}

#[test]
fn a_stream_that_does_not_fit_the_scans_gives_no_rows() {
    let rs = records();
    // one row short, one row too many, a cut row, no rows, a head alone
    for b in [
        stream(&[entry(493.27, 1), entry(547.30, 1)]),
        stream(&[
            entry(493.27, 1),
            entry(547.30, 1),
            entry(687.85, 1),
            entry(700.0, 1),
        ]),
        cut(stream(&[
            entry(493.27, 1),
            entry(547.30, 1),
            entry(687.85, 1),
        ])),
        vec![0u8; 8],
        vec![0u8; HEAD],
    ] {
        assert!(rows(&b, 2, &rs).is_none(), "{} bytes", b.len());
    }
    // a method with no experiment after the survey has no dependent slot
    assert!(rows(&stream(&[entry(493.27, 1)]), 1, &rs).is_none());
}

#[test]
fn a_row_that_is_not_a_precursor_is_refused() {
    let range = Some([100.0, 1000.0]);
    assert_eq!(plausible(&entry(493.27, 1), range), Some((493.27, Some(1))));
    for (mz, z) in [
        (0.0, 0),
        (-5.0, 1),
        (f64::NAN, 1),
        (f64::INFINITY, 1),
        (1500.0, 1),
        (50.0, 1),
        (493.27, 1000),
    ] {
        assert_eq!(plausible(&entry(mz, z), range), None, "{mz} {z}");
    }
    assert_eq!(plausible(&entry(39_000.0, 0), None), Some((39_000.0, None)));
    assert_eq!(plausible(&entry(41_000.0, 0), None), None);
}
