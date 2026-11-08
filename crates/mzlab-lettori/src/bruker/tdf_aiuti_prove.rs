//! Unit tests of `tdf_aiuti`: folder names, the sums of scan ranges and the frames of a cut `*_bin`.
use super::*;
use crate::bruker::spettri::summed;

#[test]
fn the_folder_of_a_member_name_splits_on_both_separators() {
    assert_eq!(cartella("campione.d/analysis.tdf"), Some("campione.d"));
    assert_eq!(cartella("campione.d\\analysis.tdf"), Some("campione.d"));
    assert_eq!(cartella("a\\b.d/analysis.tdf"), Some("a\\b.d"));
    assert_eq!(cartella("analysis.tdf"), None);
}

#[test]
fn the_sum_of_the_points_is_the_sum_of_the_merged_spectrum() {
    let p = Picchi {
        // the points of scans 0, 1, 2, 2, 3, 5, 9
        start: vec![0, 1, 2, 4, 5, 5, 6, 6, 6, 6, 7],
        tof: vec![10, 10, 11, 12, 10, 13, 14],
        int: vec![4, 5, 6, 7, 8, 9, u32::MAX],
    };
    for scan in [
        vec![(0, 3)],
        vec![(1, 2), (3, 6)],
        vec![(0, 100)],
        vec![],
        vec![(2, 6), (0, 3), (5, 5)],
    ] {
        let merged: f64 = summed(&p, &scan).iter().map(|x| x.1 as f64).sum();
        assert_eq!(sum_scan(&p, &scan), merged);
    }
}

#[test]
fn overlapping_ranges_count_a_point_once_and_many_points_sort_as_few() {
    let p = Picchi {
        start: vec![0, 1, 2, 4, 5, 5, 6, 6, 6, 6, 7],
        tof: vec![10, 10, 11, 12, 10, 13, 14],
        int: vec![4, 5, 6, 7, 8, 9, 1],
    };
    assert_eq!(summed(&p, &[(2, 6), (0, 3)]), summed(&p, &[(0, 6)]));
    assert_eq!(sum_scan(&p, &[(2, 6), (0, 3)]), 39.0);
    // a frame of many points over many scans: the radix path, against a plain sort of the same points
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut c = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let mut q = Picchi::default();
    q.start.clear();
    for _ in 0..300 {
        q.start.push(q.tof.len());
        for _ in 0..(c() % 40) {
            q.tof
                .push((c() % 3_000_000) as u32 | if c() % 50 == 0 { 1 << 31 } else { 0 });
            q.int.push((c() % 1000) as u32);
        }
    }
    q.start.push(q.tof.len());
    assert!(q.len() > 2048);
    let mut expected: Vec<(u32, u64)> = Vec::new();
    let mut v: Vec<(u32, u32)> = q.tof.iter().copied().zip(q.int.iter().copied()).collect();
    v.sort_unstable();
    for (t, y) in v {
        match expected.last_mut() {
            Some(l) if l.0 == t => l.1 += u64::from(y),
            _ => expected.push((t, u64::from(y))),
        }
    }
    assert_eq!(summed(&q, &[(0, 300)]), expected);
}

#[test]
fn a_block_cut_by_the_end_of_the_file_is_told_apart() {
    // TDF: blocks of 20 bytes (the length counts the header); TSF: 8 bytes of header and 12 of payload
    let mut tdf = Vec::new();
    let mut tsf = Vec::new();
    for _ in 0..3 {
        tdf.extend(20u32.to_le_bytes());
        tdf.extend([0u8; 16]);
        tsf.extend(4u32.to_le_bytes());
        tsf.extend(8u32.to_le_bytes());
        tsf.extend([0u8; 12]);
    }
    let pos = [0, 20, 40, -1];
    for (b, t) in [(tdf, false), (tsf, true)] {
        for (dim, attesi) in [
            (60, [false, false, false, true]),
            (59, [false, false, true, true]),
            (44, [false, false, true, true]),
            (40, [false, false, true, true]),
            (39, [false, true, true, true]),
        ] {
            let mut c = b[..dim].to_vec();
            assert_eq!(cut_ones(&pos, &mut c, dim as u64, t), attesi, "{t} {dim}");
        }
    }
}
