//! `count_keys` (the counting of `ms2_exps`) finds its keys through a map: the same keys, counts and order as a scan of the list for each key.
use super::*;

/// The counting `ms2_exps` did before the map (`==` on the values), kept to compare.
fn by_scan(ms2: &[&Scan]) -> Vec<KeyCount> {
    let mut cnt: Vec<KeyCount> = vec![];
    for s in ms2 {
        let k = (
            s.precursor.filter(|p| *p != 0.0).map(|p| round(p, 1)),
            s.collision_energy,
        );
        match cnt.iter_mut().find(|(q, _)| *q == k) {
            Some(c) => c.1 += 1,
            None => cnt.push((k, 1)),
        }
    }
    cnt
}

#[test]
fn the_map_counts_the_keys_as_the_scan_of_the_list_did() {
    // -0.0 equals 0.0, a NaN equals nothing, None equals None
    let ce = [None, Some(0.0), Some(-0.0), Some(f64::NAN), Some(30.0)];
    let v: Vec<Scan> = (0..400u32)
        .map(|k| Scan {
            level: 2,
            precursor: match k % 7 {
                0 => None,
                1 => Some(-0.01),
                2 => Some(f64::NAN),
                _ => Some(150.0 + f64::from(k % 37) * 0.37),
            },
            collision_energy: ce[(k % 5) as usize],
            ..Scan::default()
        })
        .collect();
    let r: Vec<&Scan> = v.iter().collect();
    let (got, want) = (count_keys(&r), by_scan(&r));
    assert_eq!(format!("{got:?}"), format!("{want:?}"));
    assert!(got.len() > 100);
}
