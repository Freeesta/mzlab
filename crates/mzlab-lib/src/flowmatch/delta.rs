//! Precursor mass difference (`query - library`) annotation by curated metabolic/environmental biotransformations
//! (Watrous et al. 2012, *PNAS* 109:E1743, DOI:10.1073/pnas.1203689109) and bounded CHNOPS formula differences
//! respecting valence/nitrogen-rule electron-pair parity (Kind & Fiehn 2007, *BMC Bioinformatics* 8:105,
//! DOI:10.1186/1471-2105-8-105).

/// Monoisotopic IUPAC atomic masses [C, H, N, O, P, S, F, Cl, Br] in Da.
const MASSES: [f64; 9] = [
    12.0,
    1.007825032,
    14.003074005,
    15.994914620,
    30.973761998,
    31.972071174,
    18.998403163,
    34.968852682,
    78.9183376,
];

/// `(fwd_label, [C, H, N, O, P, S, F, Cl, Br])` curated biotransformations; negation flips `+`/`-`.
const CURATED: &[(&str, [i8; 9])] = &[
    ("+O", [0, 0, 0, 1, 0, 0, 0, 0, 0]),
    ("+O2", [0, 0, 0, 2, 0, 0, 0, 0, 0]),
    ("+H2", [0, 2, 0, 0, 0, 0, 0, 0, 0]),
    ("+H2O", [0, 2, 0, 1, 0, 0, 0, 0, 0]),
    ("+CH2", [1, 2, 0, 0, 0, 0, 0, 0, 0]),
    ("+CH2O", [1, 2, 0, 1, 0, 0, 0, 0, 0]),
    ("+CO", [1, 0, 0, 1, 0, 0, 0, 0, 0]),
    ("+CO2", [1, 0, 0, 2, 0, 0, 0, 0, 0]),
    ("+C2H2O", [2, 2, 0, 1, 0, 0, 0, 0, 0]),
    ("+C2H4", [2, 4, 0, 0, 0, 0, 0, 0, 0]),
    ("+C2H2", [2, 2, 0, 0, 0, 0, 0, 0, 0]),
    ("+NH3", [0, 3, 1, 0, 0, 0, 0, 0, 0]),
    ("+NH", [0, 1, 1, 0, 0, 0, 0, 0, 0]),
    ("+CH3N", [1, 3, 1, 0, 0, 0, 0, 0, 0]),
    ("+SO3", [0, 0, 0, 3, 0, 1, 0, 0, 0]),
    ("+HPO3", [0, 1, 0, 3, 1, 0, 0, 0, 0]),
    ("+C6H8O6", [6, 8, 0, 6, 0, 0, 0, 0, 0]),
    ("+C6H10O5", [6, 10, 0, 5, 0, 0, 0, 0, 0]),
    ("+C5H8O4", [5, 8, 0, 4, 0, 0, 0, 0, 0]),
    ("+C2H3NO", [2, 3, 1, 1, 0, 0, 0, 0, 0]),
    ("+C2H5NO2S", [2, 5, 1, 2, 0, 1, 0, 0, 0]),
    ("+C3H5NO2S", [3, 5, 1, 2, 0, 1, 0, 0, 0]),
    ("+C5H7NO3S", [5, 7, 1, 3, 0, 1, 0, 0, 0]),
    ("+C10H15N3O5S", [10, 15, 3, 5, 0, 1, 0, 0, 0]),
    ("+C10H15N3O6S", [10, 15, 3, 6, 0, 1, 0, 0, 0]),
    ("-Cl+H", [0, 1, 0, 0, 0, 0, 0, -1, 0]),
    ("-Br+H", [0, 1, 0, 0, 0, 0, 0, 0, -1]),
    ("-F+H", [0, 1, 0, 0, 0, 0, -1, 0, 0]),
    ("-Cl+OH", [0, 1, 0, 1, 0, 0, 0, -1, 0]),
    ("-Br+OH", [0, 1, 0, 1, 0, 0, 0, 0, -1]),
    ("-F+OH", [0, 1, 0, 1, 0, 0, -1, 0, 0]),
    ("+O-H2", [0, -2, 0, 1, 0, 0, 0, 0, 0]),
    ("+O-NH", [0, -1, -1, 1, 0, 0, 0, 0, 0]),
    ("+O-NH3", [0, -3, -1, 1, 0, 0, 0, 0, 0]),
    ("+H2-O2", [0, 2, 0, -2, 0, 0, 0, 0, 0]),
    ("+S-O", [0, 0, 0, -1, 0, 1, 0, 0, 0]),
];

/// Annotated precursor delta formula (e.g. `"+O"`) and unsigned mass error in mDa.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeltaHit {
    pub(crate) formula: String,
    pub(crate) err_mda: f64,
}

fn better(err: f64, atoms: u32, best_err: f64, best_atoms: u32) -> bool {
    let key = ((err * 10.0).round_ties_even() as u32, atoms, err);
    let best_key = (
        (best_err * 10.0).round_ties_even() as u32,
        best_atoms,
        best_err,
    );
    key < best_key
}

fn flip_signs(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'+' => '-',
            b'-' => '+',
            _ => b as char,
        })
        .collect()
}

fn best_curated(delta_da: f64, tol_mda: f64) -> Option<DeltaHit> {
    let mut best: Option<(&'static str, bool, f64, u32)> = None;
    for &(fwd, counts) in CURATED {
        let mut m = 0.0;
        let mut atoms = 0_u32;
        for (&k, &w) in counts.iter().zip(MASSES.iter()) {
            m += k as f64 * w;
            atoms += k.unsigned_abs() as u32;
        }
        for (rev, theo) in [(false, m), (true, -m)] {
            let err = (delta_da - theo).abs() * 1000.0;
            if err <= tol_mda && best.is_none_or(|(_, _, be, ba)| better(err, atoms, be, ba)) {
                best = Some((fwd, rev, err, atoms));
            }
        }
    }
    best.map(|(f, rev, err_mda, _)| DeltaHit {
        formula: if rev { flip_signs(f) } else { f.to_string() },
        err_mda,
    })
}

fn format_chnops(sign: char, counts: [i32; 5]) -> String {
    const SYMS: [char; 5] = ['C', 'H', 'N', 'O', 'S'];
    let mut s = String::with_capacity(16);
    s.push(sign);
    for (&k, &sym) in counts.iter().zip(SYMS.iter()) {
        if k == 0 {
            continue;
        }
        s.push(sym);
        if k >= 10 {
            s.push('1');
            s.push((b'0' + (k - 10) as u8) as char);
        } else if k > 1 {
            s.push((b'0' + k as u8) as char);
        }
    }
    s
}

fn best_chnops(delta_da: f64, tol_mda: f64) -> Option<DeltaHit> {
    let target = delta_da.abs();
    let sign = if delta_da >= 0.0 { '+' } else { '-' };
    let mut best: Option<([i32; 5], f64, u32)> = None;
    for dc in 0_i32..=6 {
        for dn in 0_i32..=2 {
            for do_ in 0_i32..=4 {
                for ds in 0_i32..=1 {
                    let base = dc as f64 * MASSES[0]
                        + dn as f64 * MASSES[2]
                        + do_ as f64 * MASSES[3]
                        + ds as f64 * MASSES[5];
                    let est = (target - base) / MASSES[1];
                    if !(-0.5..=12.5).contains(&est) {
                        continue;
                    }
                    let dh = est.round_ties_even() as i32;
                    if !(0..=12).contains(&dh)
                        || (dc == 0 && dh == 0 && dn == 0 && do_ == 0 && ds == 0)
                        || dh.rem_euclid(2) != dn.rem_euclid(2)
                    {
                        continue;
                    }
                    let err = (target - (base + dh as f64 * MASSES[1])).abs() * 1000.0;
                    let atoms = (dc + dh + dn + do_ + ds) as u32;
                    if err <= tol_mda && best.is_none_or(|(_, be, ba)| better(err, atoms, be, ba)) {
                        best = Some(([dc, dh, dn, do_, ds], err, atoms));
                    }
                }
            }
        }
    }
    best.map(|(counts, err_mda, _)| DeltaHit {
        formula: format_chnops(sign, counts),
        err_mda,
    })
}

/// Annotate a precursor delta (`query - library`, Da) within `tol_mda` (mDa).
pub(crate) fn annotate_delta(delta_da: f64, tol_mda: f64) -> Option<DeltaHit> {
    if !delta_da.is_finite() || !tol_mda.is_finite() || tol_mda <= 0.0 || delta_da == 0.0 {
        return None;
    }
    best_curated(delta_da, tol_mda).or_else(|| best_chnops(delta_da, tol_mda))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_hit(delta_da: f64, tol_mda: f64, want: &str, max_err_mda: f64) {
        let hit = annotate_delta(delta_da, tol_mda)
            .unwrap_or_else(|| panic!("expected {want} for delta {delta_da} within {tol_mda} mDa"));
        assert_eq!(hit.formula, want, "delta={delta_da}");
        assert!(
            hit.err_mda >= 0.0 && hit.err_mda < max_err_mda,
            "delta={delta_da}: err={} >= {max_err_mda}",
            hit.err_mda
        );
    }

    #[test]
    fn curated_biotransformations_resolve_both_directions() {
        assert_hit(15.994915, 5.0, "+O", 0.05);
        assert_hit(-15.994915, 5.0, "-O", 0.05);
        assert_hit(14.015650, 5.0, "+CH2", 0.05);
        assert_hit(176.032088, 5.0, "+C6H8O6", 0.05);
        assert_hit(-33.961028, 5.0, "-Cl+H", 0.5);
        assert_hit(33.961028, 5.0, "+Cl-H", 0.5);
    }

    #[test]
    fn tolerance_and_degenerate_inputs_return_none() {
        assert!(annotate_delta(16.02, 1.0).is_none());
        assert!(annotate_delta(15.994915, 0.0).is_none());
        assert!(annotate_delta(15.994915, -1.0).is_none());
        assert!(annotate_delta(0.0, 5.0).is_none());
        assert!(annotate_delta(f64::NAN, 5.0).is_none());
        assert!(annotate_delta(15.994915, f64::NAN).is_none());
    }

    #[test]
    fn fallback_bounded_chnops_resolves_cleanly() {
        assert_hit(44.026215, 5.0, "+C2H4O", 0.05);
        assert_hit(-44.026215, 5.0, "-C2H4O", 0.05);
        assert_hit(31.989829, 5.0, "+O2", 0.05);
    }
}
