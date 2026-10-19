//! Parts of the score of one charge in the isotope finder: coverage of the envelope, self-consistency for charges above 1, the offsets kept and
//! emitted, agreement of the intensities with the envelope and consistency of the spacing. Used by `Engine::score_with`.

use std::collections::{BTreeMap, BTreeSet};

use super::envelope::{jround, Envelope, SUPPORT_CUTOFF};
use super::ladder::{Ladder, OffsetPeak};
use super::{Pt, Tol};

const MIN_GAP_HORIZON: usize = 4;
const GAP_HORIZON_FRACTION: f64 = 0.25;
const SPACING_GRID_FACTOR: f64 = 0.6;
const SPACING_SIGMA_FACTOR: f64 = 0.35;
const MIN_BRIDGED_REL_INTENSITY: f64 = 0.005;

/// Share of the expected height (offsets with at least 0.02) that the observed offsets, shifted by the placement, cover.
pub(super) fn coverage(env: &Envelope, observed: &BTreeSet<i64>, placement: i64) -> f64 {
    let (mut want, mut have) = (0.0, 0.0);
    for o in 0..=env.max_offset() {
        let w = env.expected_at(o);
        if w >= SUPPORT_CUTOFF {
            want += w;
            if observed.contains(&(o - placement)) {
                have += w;
            }
        }
    }
    if want <= 0.0 {
        1.0
    } else {
        have / want
    }
}

/// For charges above 1: the offsets that are not multiples of the charge and are required are those present, or absent although the expected
/// height is at least the faintest observed one; the share present. 1 for charge 1, 0 if nothing is required.
pub(super) fn self_consistency(
    z: i32,
    observed: &BTreeMap<i64, f64>,
    env: &Envelope,
    placement: i64,
    base_int: f64,
) -> f64 {
    if z == 1 {
        return 1.0;
    }
    let floor = if base_int > 0.0 {
        observed.values().fold(f64::MAX, |m, v| m.min(v / base_int))
    } else {
        f64::MAX
    };
    let floor = if floor == f64::MAX { 0.0 } else { floor };
    let (mut total, mut present) = (0, 0);
    for o in 1..=env.max_offset() {
        if o % z as i64 != 0 && env.expected_at(o) >= SUPPORT_CUTOFF {
            if observed.contains_key(&(o - placement)) {
                total += 1;
                present += 1;
            } else if env.expected_at(o) >= floor {
                total += 1;
            }
        }
    }
    if total == 0 {
        0.0
    } else {
        present as f64 / total as f64
    }
}

pub(super) fn significant(p: Option<&OffsetPeak>, base_int: f64) -> bool {
    match p {
        Some(p) if base_int > 0.0 => p.intensity / base_int >= MIN_BRIDGED_REL_INTENSITY,
        _ => true,
    }
}

/// Offsets kept when 13C is not required: from 0 outwards in both directions, the next offset always, one further away (up to the horizon) only if the
/// envelope supports it and it is not negligible.
pub(super) fn kept_offsets(
    obs: &BTreeMap<i64, OffsetPeak>,
    env: &Envelope,
    placement: i64,
    base_int: f64,
) -> BTreeSet<i64> {
    let mut kept = BTreeSet::new();
    let (Some(&min_o), Some(&max_o)) = (obs.keys().next(), obs.keys().next_back()) else {
        return kept;
    };
    kept.insert(0);
    let supported = (0..=env.max_offset())
        .filter(|o| env.expected_at(*o) >= SUPPORT_CUTOFF)
        .count();
    let horizon =
        MIN_GAP_HORIZON.max(jround(supported as f64 * GAP_HORIZON_FRACTION) as usize) as i64;
    for dir in [1i64, -1] {
        let mut cur = 0i64;
        loop {
            let next = (1..=horizon)
                .map(|d| cur + dir * d)
                .take_while(|k| if dir > 0 { *k <= max_o } else { *k >= min_o })
                .find(|k| {
                    obs.contains_key(k)
                        && ((*k - cur).abs() == 1
                            || (env.upper_at(k + placement) >= SUPPORT_CUTOFF
                                && significant(obs.get(k), base_int)))
                });
            match next {
                Some(k) => {
                    kept.insert(k);
                    cur = k;
                }
                None => break,
            }
        }
    }
    kept
}

/// The kept offsets plus, if the searched peak lies outside them, every observed offset between it and 0.
pub(super) fn emitted_offsets(
    kept: &BTreeSet<i64>,
    obs: &BTreeMap<i64, OffsetPeak>,
    anchor: i64,
) -> BTreeSet<i64> {
    let mut out = kept.clone();
    if !kept.contains(&anchor) {
        out.extend((anchor.min(0)..=anchor.max(0)).filter(|k| obs.contains_key(k)));
    }
    out
}

/// The candidates on the emitted offsets (all of them if none is).
pub(super) fn collect_emitted(
    cands: &[Pt],
    emit: &BTreeSet<i64>,
    base_mz: f64,
    spacing: f64,
) -> Vec<Pt> {
    let kept: Vec<Pt> = cands
        .iter()
        .copied()
        .filter(|p| emit.contains(&jround((p.0 - base_mz) / spacing)))
        .collect();
    if kept.is_empty() {
        cands.to_vec()
    } else {
        kept
    }
}

/// 1 minus the share of the intensity of the kept offsets that exceeds the upper bound of the envelope.
pub(super) fn intensity_agreement(
    kept: &BTreeSet<i64>,
    obs: &BTreeMap<i64, OffsetPeak>,
    env: &Envelope,
    placement: i64,
    base_int: f64,
) -> f64 {
    let (mut excess, mut total) = (0.0, 0.0);
    for k in kept {
        let Some(p) = obs.get(k) else { continue };
        let rel = p.intensity / base_int;
        excess += (rel - env.upper_at(k + placement)).max(0.0);
        total += rel;
    }
    if total > 0.0 {
        (1.0 - excess / total).max(0.0)
    } else {
        1.0
    }
}

/// How well the on-grid peaks keep one spacing: `exp(-(rms residual of a line through the origin / (0.35 x tolerance))^2)`; 1 with fewer than 2 peaks.
pub(super) fn spacing_consistency(ladder: &Ladder, base_mz: f64, tol: &Tol) -> f64 {
    let mz_by_offset = ladder.on_grid_mz(SPACING_GRID_FACTOR);
    if mz_by_offset.len() < 2 {
        return 1.0;
    }
    let (mut kd, mut k2) = (0.0, 0.0);
    for (k, m) in &mz_by_offset {
        kd += *k as f64 * (m - base_mz);
        k2 += (*k as f64).powi(2);
    }
    if k2 == 0.0 {
        return 1.0;
    }
    let slope = kd / k2;
    let ss: f64 = mz_by_offset
        .iter()
        .map(|(k, m)| (m - base_mz - slope * *k as f64).powi(2))
        .sum();
    let eps = (ss / mz_by_offset.len() as f64).sqrt();
    let sigma = SPACING_SIGMA_FACTOR * tol.for_mass(base_mz);
    if sigma <= 0.0 {
        return 1.0;
    }
    (-(eps / sigma).powi(2)).exp()
}
