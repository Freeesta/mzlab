use super::*;

/// Features at `mz` with the same apex and the given window areas, sorted by m/z.
fn feats(mz: &[f64], area: &[f64]) -> FileFeatures {
    FileFeatures {
        mz: mz.to_vec(),
        rt: vec![5.0; mz.len()],
        area_win: area.to_vec(),
        flags: vec![0; mz.len()],
        iso_group: vec![0; mz.len()],
        charge: vec![0; mz.len()],
        ..FileFeatures::default()
    }
}

/// `iso_group` of a halogenated ion of neutral mass `m`: members at M, M+2, M+4... (81Br steps) with binomial areas.
fn groups_of(m: f64, area: &[f64], legacy: bool) -> Vec<u32> {
    let mz: Vec<f64> = (0..area.len())
        .map(|i| m + ELECTRON_PROTON + i as f64 * D_PLUS2[3])
        .collect();
    let mut f = feats(&mz, area);
    let p = IsoParams {
        legacy_plus2_cap: legacy,
        ..IsoParams::default()
    };
    group(&mut f, &p).unwrap();
    f.iso_group
}

#[test]
fn polybrominated_clusters_group_with_the_binomial_ratios() {
    // Br2 M+2/M = 1.95, Br3 2.92 (Meija 2016 abundances): one group each, not [0, g, g] with M+2 as the head
    for area in [vec![100.0, 195.0, 95.0], vec![100.0, 292.0, 284.0, 92.0]] {
        let g = groups_of(400.0, &area, false);
        assert!(g[0] != 0 && g.iter().all(|&x| x == g[0]), "{area:?}: {g:?}");
    }
}

#[test]
fn the_old_cap_of_1_2_stays_selectable() {
    let g = groups_of(400.0, &[100.0, 195.0, 95.0], true);
    assert_eq!(g[0], 0, "{g:?}");
    assert_eq!(g[1], g[2], "{g:?}");
}

#[test]
fn the_halogen_bound_follows_the_mass() {
    // a 150 Da ion holds one Br at most (0.97 + averagine): a ratio of 1.95 is still refused
    let g = groups_of(150.0, &[100.0, 195.0], false);
    assert_eq!(g, vec![0, 0]);
}

#[test]
fn a_cursor_finds_the_partition_point_wherever_it_starts() {
    let mut seed = 9u64;
    let mut r = |n: u64| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) % n
    };
    for _ in 0..300 {
        let mut mz: Vec<f64> = (0..r(60)).map(|_| 100.0 + r(40) as f64 * 0.25).collect();
        mz.sort_by(f64::total_cmp);
        let lo = 99.0 + r(45) as f64 * 0.25;
        for start in 0..=mz.len() {
            assert_eq!(
                near(&mz, start, |m| m < lo),
                mz.partition_point(|&m| m < lo)
            );
            assert_eq!(
                near(&mz, start, |m| m <= lo),
                mz.partition_point(|&m| m <= lo)
            );
        }
    }
}
