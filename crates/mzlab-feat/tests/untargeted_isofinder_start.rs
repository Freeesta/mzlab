//! `find_candidates` starts its walk down just below the target only where that gives the same candidates as starting at the top of the spectrum.
//! The reference is the walk from the top as it was written before (verbatim), run on deterministic pseudo-random spectra over narrow and wide
//! tolerances, charges 1-5 and 20, non-finite m/z, duplicates, f32-rounded values and targets outside the spectrum; every value is compared by its bits.

use mzlab_feat::untargeted::isofinder::{find_candidates, ladder::closest_index, Pt, Tol};

fn possible_isotope_ref(mz: f64, known: &[Pt], diffs: &[f64], tol: &Tol, up: bool) -> bool {
    for d in diffs {
        let theo = if up { mz - d } else { mz + d };
        let t = tol.for_mass(theo);
        for k in known.iter().rev() {
            if up && theo - t > k.0 || !up && k.0 > theo + t {
                break;
            }
            if k.0 >= theo - t && k.0 <= theo + t {
                return true;
            }
        }
    }
    false
}

fn insert_if_new_ref(list: &mut Vec<Pt>, p: Pt) {
    for c in (0..list.len()).rev() {
        if list[c].0 == p.0 {
            return;
        }
        if list[c].0 < p.0 {
            list.insert(c + 1, p);
            return;
        }
    }
}

/// The candidate search with the walk down started at the top (`from_top`, the reference) or always at `t - 1` (the unguarded shortcut, kept only
/// to show that the comparison can fail).
fn walk(
    diffs: &[f64],
    max_diff: f64,
    tol: &Tol,
    mz: &[f64],
    y: &[f64],
    target_mz: f64,
    from_top: bool,
) -> Vec<Pt> {
    let Some(t) = closest_index(mz, target_mz) else {
        return Vec::new();
    };
    let n = mz.len() as i64;
    let mut cands = vec![(mz[t], y[t])];
    let (mut m, mut last) = (mz[t], mz[t]);
    let mut dp = if from_top { n - 1 } else { t as i64 - 1 };
    while dp >= 0 && m >= last - max_diff {
        m = mz[dp as usize];
        if possible_isotope_ref(m, &cands, diffs, tol, false) {
            cands.push((m, y[dp as usize]));
            last = m;
        }
        dp -= 1;
    }
    cands.sort_by(|a, b| a.0.total_cmp(&b.0));
    m = mz[t];
    let mut max_mz = m;
    dp += 1;
    while dp < n && m <= max_mz + max_diff {
        m = mz[dp as usize];
        if possible_isotope_ref(m, &cands, diffs, tol, true) {
            let p = (m, y[dp as usize]);
            if m > max_mz {
                cands.push(p);
                max_mz = m;
            } else {
                insert_if_new_ref(&mut cands, p);
            }
        }
        dp += 1;
    }
    cands.sort_by(|a, b| a.0.total_cmp(&b.0));
    cands
}

const CHARGES: [i32; 6] = [1, 2, 3, 4, 5, 20];
const ELEMENTS: [&str; 8] = ["H", "C", "N", "O", "S", "Cl", "Br", "Si"];

/// The differences of `Engine::new` for charge `z`: every pair of isotopes of every element, divided by the charge, ascending.
fn diffs_for(z: i32) -> Vec<f64> {
    let mut out = Vec::new();
    for el in ELEMENTS {
        let iso = mzlab_core::isoscore::isotopes_of(el).unwrap();
        for i in 0..iso.len() {
            for j in i + 1..iso.len() {
                out.push((iso[j].0 - iso[i].0) / z as f64);
            }
        }
    }
    out.sort_by(f64::total_cmp);
    out
}

/// A 64-bit linear congruential generator (Knuth's MMIX constants); the top 53 bits give a float in [0, 1).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn below(&mut self, n: usize) -> usize {
        ((self.next() * n as f64) as usize).min(n - 1)
    }
}

/// Spectrum `k`: a few random points, an isotope cluster of a random charge, duplicated m/z now and then, f32-rounded on odd `k`, and on every
/// fifth spectrum one value replaced by NaN, +inf or -inf at the top, the middle or the bottom.
fn spectrum(k: usize, rng: &mut Lcg) -> (Vec<f64>, Vec<f64>) {
    let len = [0, 1, 2, 3, 5, 12, 40, 120, 300][k % 9];
    let mut pts: Vec<Pt> = Vec::with_capacity(len + 16);
    let mut cur = 80.0 + rng.next() * 900.0;
    for _ in 0..len {
        let r = rng.below(10);
        cur += match r {
            0 => 0.0,
            1..=3 => 1.003_354_8 / (1 + rng.below(5)) as f64 + (rng.next() - 0.5) * 0.01,
            _ => rng.next() * 4.0,
        };
        pts.push((cur, 1e2 + rng.next() * 1e7));
    }
    if len >= 3 {
        let z = CHARGES[rng.below(CHARGES.len())] as f64;
        let m0 = pts[0].0 + rng.next() * (cur - pts[0].0);
        for i in -2..=4 {
            let m = m0 + i as f64 * 1.003_354_8 / z + (rng.next() - 0.5) * 0.004;
            let p = (m, 1e4 + rng.next() * 5e6);
            pts.push(p);
            if rng.below(4) == 0 {
                pts.push((p.0, p.1 * 0.5));
            }
        }
    }
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    if k % 2 == 1 {
        pts.iter_mut()
            .for_each(|p| *p = (p.0 as f32 as f64, p.1 as f32 as f64));
    }
    let (mut mz, y): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
    if k.is_multiple_of(5) && !mz.is_empty() {
        let n = mz.len();
        let at = [n - 1, n / 2, 0][(k / 5) % 3];
        mz[at] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY][(k / 15) % 3];
    }
    (mz, y)
}

fn same(a: &[Pt], b: &[Pt]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(p, q)| p.0.to_bits() == q.0.to_bits() && p.1.to_bits() == q.1.to_bits())
}

#[test]
fn start_below_the_target_gives_the_candidates_of_the_walk_from_the_top() {
    let tols = [
        (0.005, 15.0),
        (0.001, 5.0),
        (0.05, 15.0),
        (0.5, 15.0),
        (0.005, 2000.0),
        (0.2, 15.0),
    ];
    let diffs: Vec<Vec<f64>> = CHARGES.iter().map(|&z| diffs_for(z)).collect();
    let mut rng = Lcg(0x2545_F491_4F6C_DD1D);
    let (mut calls, mut differ, mut shortcut_differ, mut shortcut_differ_nonfinite) =
        (0usize, 0usize, 0usize, 0usize);
    let mut shortcut = 0usize;
    for k in 0..6000 {
        let (abs, ppm) = tols[k % tols.len()];
        let tol = Tol { abs, ppm };
        let (mz, y) = spectrum(k, &mut rng);
        let nonfinite = mz.iter().any(|v| !v.is_finite());
        let mut targets = vec![500.0];
        if let (Some(&lo), Some(&hi)) = (mz.first(), mz.last()) {
            targets.extend([lo - 5.0, lo, mz[mz.len() / 2], hi, hi + 5.0]);
            if mz.len() > 4 {
                targets.push(mz[rng.below(mz.len())] + (rng.next() - 0.5) * 0.01);
            }
        }
        for &target in &targets {
            for d in &diffs {
                let last = *d.last().unwrap();
                let max_diff = last + 10.0 * tol.for_mass(last);
                let want = walk(d, max_diff, &tol, &mz, &y, target, true);
                let got = find_candidates(d, max_diff, &tol, &mz, &y, target);
                calls += 1;
                let top = mz.last().copied().unwrap_or(0.0);
                let lo = closest_index(&mz, target).map_or(0.0, |t| mz[t]);
                shortcut += (!nonfinite
                    && !mz.is_empty()
                    && d.iter()
                        .all(|&x| x > tol.for_mass(lo + x).max(tol.for_mass(top + x))))
                    as usize;
                if !same(&want, &got) {
                    differ += 1;
                    eprintln!(
                        "spectrum {k} tol ({abs}, {ppm}) target {target}: {want:?} != {got:?}"
                    );
                }
                if !same(&want, &walk(d, max_diff, &tol, &mz, &y, target, false)) {
                    shortcut_differ += 1;
                    shortcut_differ_nonfinite += nonfinite as usize;
                }
            }
        }
    }
    eprintln!(
        "calls {calls}, {shortcut} start below the target; differing: guarded {differ}, unguarded {shortcut_differ} ({shortcut_differ_nonfinite} non-finite)"
    );
    assert_eq!(differ, 0);
    assert!(
        shortcut > calls / 4,
        "the shortcut must be taken in a good share of the calls"
    );
    // the comparison is not vacuous: starting at `t - 1` without the guard changes the result, at wide tolerance and with non-finite m/z
    assert!(shortcut_differ > shortcut_differ_nonfinite && shortcut_differ_nonfinite > 0);
}

#[test]
fn tiny_spectra_and_targets_outside_the_range() {
    let tol = Tol::default();
    let d = diffs_for(1);
    let max_diff = d.last().unwrap() + 10.0 * tol.for_mass(*d.last().unwrap());
    let spectra: [(&[f64], &[f64]); 4] = [
        (&[], &[]),
        (&[300.0], &[1.0]),
        (&[300.0, 301.003_354_8], &[10.0, 1.0]),
        (&[300.0, 300.0], &[10.0, 5.0]),
    ];
    for (mz, y) in spectra {
        for target in [0.0, 299.0, 300.0, 300.5, 301.003_354_8, 1e6] {
            let want = walk(&d, max_diff, &tol, mz, y, target, true);
            assert!(
                same(&want, &find_candidates(&d, max_diff, &tol, mz, y, target)),
                "{mz:?} {target}"
            );
        }
    }
}
