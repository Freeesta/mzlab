// Ported from Attributio diagnostics/schymanski_export.py.
//! The legend of the Schymanski workbook.

use super::cells::Sheet;
use super::pyfmt::toml_text;
use super::schymanski::site_short;
use crate::attributio::campaign::Campaign;
use crate::attributio::frame::Cell;

/// `_notes`: the legend of the columns and the caveats.
pub fn notes(c: &Campaign, sites: &[String], has_quant: bool) -> Sheet {
    let std = |k: &str| toml_text(c.standards.get(k));
    let or = |k: &str, d: &str| {
        c.standards
            .get(k)
            .map_or(d.to_string(), |v| toml_text(Some(v)))
    };
    let samples = sites
        .iter()
        .map(|s| format!("{s} ({})", site_short(c, s)))
        .collect::<Vec<_>>()
        .join(", ");
    let mut rows: Vec<(String, String)> = vec![
        ("File".into(), "All three levels in one table; filter on 'schymanski_level'. One compound = ONE row, at its best level (L1 > L2a > L3).".into()),
        ("Dataset".into(), format!("{}: {}, {}, DDA POS+NEG, sorbents {}.", c.name, c.instrument, c.fragmentation, c.sorbents.join(" + "))),
        ("Samples".into(), format!("{samples}. A sample is a site; the sorbents are two extractions of the same water (areas summed).")),
        ("Count".into(), "by_identity = identities from identity_table() (the count of the paper). features = one row per feature/ion.".into()),
        ("L1".into(), format!("Reference standard: mass <= {} ppm, RT within {} min of the standard on this instrument, and MS/MS cosine ≥ {} on ≥ {} fragments against the standard's own spectrum of the same ion (or confirmed by the chemist in level1_decisions.csv). confidence_score = that MS/MS cosine.", std("mass_tol_ppm"), std("rt_tol_min"), or("ms2_min_cosine", "0.7"), or("ms2_min_matched", "2"))),
    ];
    rows.extend(
        NOTES_FIXED
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string())),
    );
    if has_quant {
        let ef = c
            .quantification
            .get("spe_enrichment_factor")
            .and_then(|v| v.as_f64());
        let ef = ef.map_or("None".into(), mzlab_core::json::fmt_g);
        rows.push(("conc_env_ngL_<sample> (L1)".into(), format!("Estimated concentration in water: extract concentration from the external calibration (recommended adduct) / EF {ef}x * 1000. Shown even when extrapolated.")));
        rows.push(("quantification caveat".into(), "No correction for SPE recovery (not validated): the true concentration is likely higher. xLOQ_<sample> = sample area / area of the lowest calibration point; < 1 = below it (order of magnitude only). calib_R2 below threshold = unreliable line.".into()));
    }
    let mut s = Sheet::new("notes", &["Field", "Meaning"]);
    s.rows = rows
        .into_iter()
        .map(|(a, b)| vec![Cell::Str(a), Cell::Str(b)])
        .collect();
    s
}

const NOTES_FIXED: [(&str, &str); 10] = [
    ("L2a", "Spectral-library match: cosine ≥ 0.70 with ≥ 4 shared fragment ions and the library precursor within 15 ppm (otherwise an analogue). confidence_score = best cosine."),
    ("L3", "CSI:FingerID structure with COSMIC (ConfidenceScoreApproximate) ≥ 0.64, ~10% FDR (Hoffmann et al., Nat. Biotechnol. 2022) -- confidence_score = COSMIC; or a reference standard matching mass + RT whose MS/MS could not confirm it (see why_level)."),
    ("match_type", "direct = library hit on the SAME precursor (|ppm| <= 20) -> true L2a; analog = similar spectrum of a DIFFERENT mass (GNPS analogue search): not the same molecule; mzMine (no ppm) = mzMine hit, no error reported."),
    ("putative_class / analog_hypothesis", "Analogue hits only: reading of the mass difference (homologue, transformation, artifact, near-isobaric, unassigned). Always putative."),
    ("matched_peaks / mz_error_ppm", "Shared MS2 peaks and precursor error of the best library hit (GNPS2; SIRIUS for the peaks). mzMine does not report them."),
    ("rt_logp_z / rt_logp_check (L3)", "RT vs logP consistency relative to the other L3 candidates of the batch (isotonic fit, robust z). |z| > 3.5 = outlier: a clue, not a proof, of a wrong structure."),
    ("area_sum_<sample>", "Sum of the areas of all collapsed features (all sorbents), DETECTED only. Semi-quantitative proxy."),
    ("category 'Other / non-target'", "Non-contaminants: a library name does not prove a natural origin (the key 'natural' is a name heuristic)."),
    ("excluded", "Identities left out of the count: library codes and internal standards / carry-over."),
    ("n = 1 per sample", "Exploratory: no statistical tests, no p-values."),
];
