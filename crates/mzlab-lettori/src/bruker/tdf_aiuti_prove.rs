//! Unit tests of `tdf_aiuti`: folder names and the sums of scan ranges.
use super::*;
use crate::bruker::spettri::somma;

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
        inizio: vec![0, 1, 2, 4, 5, 5, 6, 6, 6, 6, 7],
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
        let merged: f64 = somma(&p, &scan).iter().map(|x| x.1 as f64).sum();
        assert_eq!(somma_scan(&p, &scan), merged);
    }
}

#[test]
fn overlapping_ranges_count_a_point_once_and_many_points_sort_as_few() {
    let p = Picchi {
        inizio: vec![0, 1, 2, 4, 5, 5, 6, 6, 6, 6, 7],
        tof: vec![10, 10, 11, 12, 10, 13, 14],
        int: vec![4, 5, 6, 7, 8, 9, 1],
    };
    assert_eq!(somma(&p, &[(2, 6), (0, 3)]), somma(&p, &[(0, 6)]));
    assert_eq!(somma_scan(&p, &[(2, 6), (0, 3)]), 39.0);
    // a frame of many points over many scans: the radix path, against a plain sort of the same points
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut c = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let mut q = Picchi::default();
    q.inizio.clear();
    for _ in 0..300 {
        q.inizio.push(q.tof.len());
        for _ in 0..(c() % 40) {
            q.tof
                .push((c() % 3_000_000) as u32 | if c() % 50 == 0 { 1 << 31 } else { 0 });
            q.int.push((c() % 1000) as u32);
        }
    }
    q.inizio.push(q.tof.len());
    assert!(q.len() > 2048);
    let mut atteso: Vec<(u32, u64)> = Vec::new();
    let mut v: Vec<(u32, u32)> = q.tof.iter().copied().zip(q.int.iter().copied()).collect();
    v.sort_unstable();
    for (t, y) in v {
        match atteso.last_mut() {
            Some(l) if l.0 == t => l.1 += u64::from(y),
            _ => atteso.push((t, u64::from(y))),
        }
    }
    assert_eq!(somma(&q, &[(0, 300)]), atteso);
}
