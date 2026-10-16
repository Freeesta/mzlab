// Ported from Attributio diagnostics/schymanski_export.py.
//! The reading of the mass difference between a sample ion and its library hit (`_interpret_delta`).

pub(super) const DELTA_TOL: f64 = 0.01;
pub(super) const CH2: f64 = 14.01565;
pub(super) const DELTA_ARTIFACTS: [(f64, &str); 4] = [
    (1.003355, "13C isotopologue"),
    (0.984016, "deamidation (NH->OH)"),
    (21.981943, "Na/H adduct swap"),
    (37.955882, "K/H adduct swap"),
];
pub(super) const DELTA_TRANSFORM: [(f64, &str); 16] = [
    (2.01565, "2H (reduction / (de)saturation)"),
    (15.994915, "O (oxidation / hydroxylation)"),
    (18.010565, "H2O ((de)hydration)"),
    (26.01565, "C2H2"),
    (27.994915, "CO"),
    (30.010565, "CH2O"),
    (42.010565, "C2H2O (acetylation)"),
    (43.98983, "CO2 ((de)carboxylation)"),
    (44.026215, "C2H4O"),
    (33.961028, "Cl<->H (chlorination)"),
    (79.956815, "SO3 (sulfation)"),
    (79.966331, "HPO3 (phosphorylation)"),
    (132.042259, "pentose (C5H8O4)"),
    (146.057909, "deoxyhexose (C6H10O4)"),
    (162.052824, "hexose (glycosylation, C6H10O5)"),
    (176.032088, "glucuronide (C6H8O6)"),
];

/// Columns that open the identity sheet, in this order.
pub const FRONT: [&str; 28] = [
    "schymanski_level",
    "identity",
    "category",
    "is_contaminant_CEC",
    "cec_category",
    "confidence_score",
    "why_level",
    "rt_logp_z",
    "rt_logp_check",
    "match_type",
    "putative_class",
    "analog_hypothesis",
    "mass_diff_Da",
    "matched_peaks",
    "mz_error_ppm",
    "evidence_sources",
    "library",
    "neutral_formula",
    "inchikey",
    "smiles",
    "adducts",
    "polarity",
    "rt_min_repr",
    "mz_repr",
    "n_features_collapsed",
    "feature_locations (pol:mzMine_id)",
    "mzMine_files",
    "gnps_network_component",
];

/// `_interpret_delta`: `(hypothesis, putative class)` from the mass difference (sample neutral mass minus library).
pub fn interpret_delta(dm: f64) -> (String, &'static str) {
    let a = dm.abs();
    let sign = if dm > 0.0 { "+" } else { "-" };
    if a < 0.02 {
        return (
            "near-isobaric: likely SAME compound (low-res library precursor)".into(),
            "near-isobaric",
        );
    }
    if let Some((_, n)) = DELTA_ARTIFACTS
        .iter()
        .find(|(m, _)| (a - m).abs() <= DELTA_TOL)
    {
        return (
            format!("likely artifact: {n} (not a real transformation)"),
            "artifact",
        );
    }
    let k = (a / CH2).round_ties_even();
    if k >= 1.0 && (a - k * CH2).abs() <= DELTA_TOL {
        return (
            format!(
                "putative homolog: sample {sign}{}xCH2 vs library hit",
                k as i64
            ),
            "homolog",
        );
    }
    if let Some((_, n)) = DELTA_TRANSFORM
        .iter()
        .find(|(m, _)| (a - m).abs() <= DELTA_TOL)
    {
        return (
            format!("putative transformation: sample {sign}{n} vs library hit"),
            "transformation",
        );
    }
    (
        format!("unassigned (delta {sign}{a:.4} Da; analog, relationship unknown)"),
        "unassigned",
    )
}
