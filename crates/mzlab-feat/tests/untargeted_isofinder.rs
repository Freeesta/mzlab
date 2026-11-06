//! Isotope finder (one charge): candidates and score on synthetic spectra with an exact binomial 13C envelope (`tools/isofinder_riferimento.py`).

use mzlab_feat::untargeted::isofinder::{Config, Engine, Tol};
use serde_json::Value;

fn cases() -> Vec<Value> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/untargeted/isofinder/casi.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn vec_of(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn case(name: &str) -> (Vec<f64>, Vec<f64>, f64, i32) {
    let c = cases().into_iter().find(|c| c["name"] == name).unwrap();
    (
        vec_of(&c["mz"]),
        vec_of(&c["y"]),
        c["mz0"].as_f64().unwrap(),
        c["z"].as_i64().unwrap() as i32,
    )
}

fn engine(require_c13: bool) -> Engine {
    Engine::new(Config {
        require_c13,
        ..Config::default()
    })
    .unwrap()
}

#[test]
fn singly_charged_envelopes_are_found_and_fit_the_reference() {
    for name in ["c10_z1", "c20_z1"] {
        let (mz, y, mz0, z) = case(name);
        let e = engine(true);
        let cands = e.candidates(z, &mz, &y, mz0);
        assert_eq!(cands.len(), mz.len(), "{name}: every peak is a candidate");
        let ev = e
            .score_charge(z, mz0, &cands, 1)
            .unwrap_or_else(|| panic!("{name}: no score"));
        assert!(ev.raw > 0.0 && ev.coverage > 0.99, "{name}: {ev:?}");
        assert!(
            ev.carbon_fit > 0.98 && ev.intensity_agreement > 0.9 && ev.spacing_consistency > 0.99,
            "{name}: {ev:?}"
        );
        let reference = cases().into_iter().find(|c| c["name"] == name).unwrap()["cos_poisson"]
            .as_f64()
            .unwrap();
        assert!(
            (ev.carbon_fit - reference).abs() < 0.03,
            "{name}: {} vs {reference}",
            ev.carbon_fit
        );
        assert_eq!(ev.kept.len(), mz.len(), "{name}");
    }
}

#[test]
fn a_doubly_charged_envelope_scores_for_charge_two_and_not_for_one() {
    let (mz, y, mz0, _) = case("c40_z2");
    let e = engine(true);
    let c2 = e.candidates(2, &mz, &y, mz0);
    let ev2 = e.score_charge(2, mz0, &c2, 1).expect("z=2");
    assert!(ev2.raw > 0.0 && ev2.self_consistency > 0.0, "{ev2:?}");
    // as a singly charged series the half-step peaks are not on the 13C grid, so the charge does not count better
    let c1 = e.candidates(1, &mz, &y, mz0);
    let ev1 = e.score_charge(1, mz0, &c1, 1);
    assert!(
        ev1.as_ref().is_none_or(|v| v.raw < ev2.raw),
        "{ev1:?} vs {ev2:?}"
    );
}

#[test]
fn require_c13_rejects_a_series_without_the_m_plus_1_peak() {
    let (mz, y, mz0, z) = case("no_m1");
    let strict = engine(true);
    let cands = strict.candidates(z, &mz, &y, mz0);
    assert!(strict.score_charge(z, mz0, &cands, 1).is_none());
    let loose = engine(false);
    let cands = loose.candidates(z, &mz, &y, mz0);
    assert!(loose.score_charge(z, mz0, &cands, 1).is_some());
}

#[test]
fn a_lone_peak_and_unrelated_noise_give_no_score() {
    let e = engine(true);
    let (mz, y) = (vec![100.0, 117.3, 150.77], vec![1e5, 5e4, 3e4]);
    let cands = e.candidates(1, &mz, &y, 100.0);
    assert_eq!(cands.len(), 1);
    assert!(e.score_charge(1, 100.0, &cands, 1).is_none());
    assert!(e.candidates(1, &[], &[], 100.0).is_empty());
}

#[test]
fn the_tolerance_is_the_larger_of_the_absolute_and_the_ppm_part() {
    let t = Tol::default();
    assert_eq!(t.for_mass(100.0), 0.005);
    assert!((t.for_mass(1000.0) - 0.015).abs() < 1e-12);
    assert!(t.within(500.0, 500.0074) && !t.within(500.0, 500.0076));
}

#[test]
fn unknown_elements_and_bad_charges_are_refused() {
    assert!(Engine::new(Config {
        elements: vec!["Xx".into()],
        ..Config::default()
    })
    .is_err());
    assert!(Engine::new(Config {
        max_charge: 0,
        ..Config::default()
    })
    .is_err());
}

mod selection {
    use super::*;
    use mzlab_feat::untargeted::isofinder::select::{detect, ALT_MARGIN, MIN_ALT_QUALITY};

    const C13: f64 = 1.003354831;

    fn spectrum(m0: f64, step: f64, heights: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let mz = (0..heights.len()).map(|i| m0 + i as f64 * step).collect();
        (mz, heights.iter().map(|h| h * 1e6).collect())
    }

    #[test]
    fn one_valid_charge_gives_one_pattern_with_probability_one() {
        let (mz, y, mz0, _) = case("c10_z1");
        let d = detect(&engine(true), &mz, &y, mz0, 1).expect("detected");
        assert_eq!(d.best_charge, 1);
        assert_eq!(d.patterns.len(), 1);
        assert_eq!(d.probabilities.len(), 1);
        assert!((d.patterns[0].probability - 1.0).abs() < 1e-12);
        assert_eq!(d.patterns[0].peaks.len(), mz.len());
        let p = &d.patterns[0];
        assert!((p.score - p.eval.quality * p.eval.intensity_agreement).abs() < 1e-12);
    }

    #[test]
    fn a_doubly_charged_envelope_wins_as_charge_two() {
        let (mz, y, mz0, _) = case("c40_z2");
        let d = detect(&engine(true), &mz, &y, mz0, 1).expect("detected");
        assert_eq!(d.best_charge, 2);
        assert_eq!(d.patterns[0].charge, 2);
    }

    #[test]
    fn close_charges_are_ranked_and_the_probabilities_sum_to_one() {
        // half-step peaks with a full 13C ladder on top: charges 1 and 2 both find support; the winner goes by `raw` (quality and the peak-count reward), so its probability, which is quality only, need not be the largest
        let (mz, y) = spectrum(400.0, C13 / 2.0, &[1.0, 0.45, 0.4, 0.2, 0.1, 0.04, 0.02]);
        let d = detect(&engine(false), &mz, &y, 400.0, 1).expect("detected");
        let sum: f64 = d.probabilities.iter().map(|p| p.1).sum();
        assert_eq!(d.probabilities.len(), 2, "{:?}", d.probabilities);
        assert_eq!(d.patterns.len(), 2);
        assert!((sum - 1.0).abs() < 1e-12, "{:?}", d.probabilities);
        assert_eq!(d.patterns[0].charge, d.best_charge);
        let best = d.patterns[0].eval.quality;
        for p in &d.patterns[1..] {
            assert!(
                p.eval.quality >= best - ALT_MARGIN && p.eval.quality >= MIN_ALT_QUALITY,
                "{p:?}"
            );
            assert!(p.eval.raw <= d.patterns[0].eval.raw);
        }
        assert!(d.patterns.len() <= d.probabilities.len());
    }

    #[test]
    fn no_charge_scoring_gives_none() {
        let (mz, y, mz0, z) = case("no_m1");
        assert!(detect(&engine(true), &mz, &y, mz0, 1).is_none(), "z={z}");
        assert!(detect(&engine(true), &[100.0], &[1e5], 100.0, 1).is_none());
    }
}

fn possible_isotope_old(
    mz: f64,
    known: &[mzlab_feat::untargeted::isofinder::Pt],
    diffs: &[f64],
    tol: &Tol,
    up: bool,
) -> bool {
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

fn insert_if_new_old(
    list: &mut Vec<mzlab_feat::untargeted::isofinder::Pt>,
    p: mzlab_feat::untargeted::isofinder::Pt,
) {
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

fn find_candidates_old(
    diffs: &[f64],
    max_diff: f64,
    tol: &Tol,
    mz: &[f64],
    y: &[f64],
    target_mz: f64,
) -> Vec<mzlab_feat::untargeted::isofinder::Pt> {
    let Some(t) = mzlab_feat::untargeted::isofinder::ladder::closest_index(mz, target_mz) else {
        return Vec::new();
    };
    let n = mz.len() as i64;
    let mut cands = vec![(mz[t], y[t])];
    let (mut m, mut last) = (mz[t], mz[t]);
    let mut dp = n - 1;
    while dp >= 0 && m >= last - max_diff {
        m = mz[dp as usize];
        if possible_isotope_old(m, &cands, diffs, tol, false) {
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
        if possible_isotope_old(m, &cands, diffs, tol, true) {
            let p = (m, y[dp as usize]);
            if m > max_mz {
                cands.push(p);
                max_mz = m;
            } else {
                insert_if_new_old(&mut cands, p);
            }
        }
        dp += 1;
    }
    cands.sort_by(|a, b| a.0.total_cmp(&b.0));
    cands
}

fn isotope_diffs_for(elements: &[&str], max_charge: i32) -> Vec<Vec<f64>> {
    let mut out = vec![Vec::new(); max_charge as usize];
    for el in elements {
        let iso = mzlab_core::isoscore::isotopes_of(el).unwrap();
        for i in 0..iso.len() {
            for j in i + 1..iso.len() {
                for z in 1..=max_charge {
                    out[z as usize - 1].push((iso[j].0 - iso[i].0) / z as f64);
                }
            }
        }
    }
    out.iter_mut().for_each(|d| d.sort_by(f64::total_cmp));
    out
}

fn next_rand(state: &mut u64) -> f64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    (x >> 11) as f64 * (1.0 / ((1u64 << 53) as f64))
}

fn make_random_spectrum(
    spec_idx: usize,
    max_charge: i32,
    rng: &mut u64,
) -> (Vec<f64>, Vec<f64>, f64) {
    let as_f32 = spec_idx % 2 == 1;
    let base_len = [1, 2, 8, 32, 120, 300][spec_idx % 6];
    let mut pairs = Vec::with_capacity(base_len + 16);
    let mut cur_mz = 80.0 + next_rand(rng) * 900.0;
    for _ in 0..base_len {
        let r = *rng;
        let step = if r % 7 == 0 {
            0.0
        } else if r % 5 == 0 {
            (1.0033548378 / ((r % 5 + 1) as f64)) + (next_rand(rng) - 0.5) * 0.008
        } else {
            next_rand(rng) * 4.5
        };
        cur_mz += step;
        let inten = 1e2 + next_rand(rng) * 1e7;
        pairs.push(if as_f32 {
            (cur_mz as f32 as f64, inten as f32 as f64)
        } else {
            (cur_mz, inten)
        });
    }
    let cluster_z = (spec_idx % max_charge as usize) + 1;
    let cluster_m0 = 150.0 + next_rand(rng) * 600.0;
    for iso_k in -2..=4 {
        let m = cluster_m0
            + (iso_k as f64) * (1.0033548378 / cluster_z as f64)
            + (next_rand(rng) - 0.5) * 0.002;
        let inten = 1e4 + next_rand(rng) * 5e6;
        let pt = if as_f32 {
            (m as f32 as f64, inten as f32 as f64)
        } else {
            (m, inten)
        };
        pairs.push(pt);
        if *rng % 4 == 0 {
            pairs.push((pt.0, pt.1 * 0.5));
        }
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    (
        pairs.iter().map(|p| p.0).collect(),
        pairs.iter().map(|p| p.1).collect(),
        cluster_m0,
    )
}

#[test]
fn find_candidates_from_t_minus_1_is_bit_identical_on_3000_random_spectra() {
    let max_charge = 5;
    let diffs_all = isotope_diffs_for(&["H", "C", "N", "O", "S", "Cl", "Br", "Si"], max_charge);
    let tols = [
        Tol {
            abs: 0.005,
            ppm: 15.0,
        },
        Tol {
            abs: 0.002,
            ppm: 5.0,
        },
        Tol {
            abs: 0.015,
            ppm: 25.0,
        },
    ];
    let mut rng = 0x9E37_79B9_7F4A_7C15u64;
    for spec_idx in 0..3_000 {
        let tol = tols[spec_idx % tols.len()];
        let (mz, y, cluster_m0) = make_random_spectrum(spec_idx, max_charge, &mut rng);
        let n = mz.len();
        let targets = [
            mz[0] - 5.0,
            mz[0],
            mz[n / 2],
            mz[n - 1],
            mz[n - 1] + 5.0,
            cluster_m0,
        ];
        for &target_mz in &targets {
            for z in 1..=max_charge {
                let diffs = &diffs_all[z as usize - 1];
                let last_d = diffs.last().copied().unwrap_or(0.0);
                let max_diff = last_d + 10.0 * tol.for_mass(last_d);
                let old_c = find_candidates_old(diffs, max_diff, &tol, &mz, &y, target_mz);
                let new_c = mzlab_feat::untargeted::isofinder::find_candidates(
                    diffs, max_diff, &tol, &mz, &y, target_mz,
                );
                assert_eq!(old_c.len(), new_c.len(), "spec={spec_idx} z={z}");
                for (pa, pb) in old_c.iter().zip(new_c.iter()) {
                    assert_eq!(
                        (pa.0.to_bits(), pa.1.to_bits()),
                        (pb.0.to_bits(), pb.1.to_bits())
                    );
                }
            }
        }
    }
}
