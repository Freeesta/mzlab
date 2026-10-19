// Ported from Attributio annotate.py.
//! The thresholds of the cascade (`AnnotationSettings`) with Attributio's defaults, read from `[annotation]` of `campaign.toml`, and the text of each basis.

/// How valid library sources that name different structures are settled.
pub const LIBRARY_CONFLICT_RULES: [&str; 2] = ["best_match", "level3"];

#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationSettings {
    /// Cosine, every spectral-library source.
    pub spec_sim_min: f64,
    /// Shared fragment ions, every library (SIRIUS, GNPS2, mzMine).
    pub spec_peaks_min: f64,
    pub engine_sim_min: f64,
    pub engine_dl_sim_min: f64,
    pub engine_mass_tol_ppm: f64,
    /// The MS2 Engine is experimental in Attributio and is not ported: the flag is read so that a campaign that sets it gets a warning.
    pub use_ms2_engine: bool,
    /// Library entries named `Spectral Match to ...` (propagated annotations) taken to Level 3.
    pub putative_library_level3: bool,
    /// `best_match` (the libraries are one library) or `level3` (the rule of Attributio <= 1.10).
    pub library_conflict: String,
    /// The best match keeps Level 2a only if its cosine exceeds the best match of a different structure by this much.
    pub library_conflict_margin: f64,
    pub gnps_mass_tol_ppm: f64,
    pub mzlib_mass_tol_ppm: f64,
    /// COSMIC confidence of the Level 3 gate (~10 % FDR).
    pub cosmic_min: f64,
    /// A CANOPUS class is shown (and gives Level 4) at or above this probability.
    pub canopus_min: f64,
    /// Features of one identity farther apart than this (RT, consecutive) are different peaks.
    pub rt_group_tol_min: f64,
}

/// The level that follows from a basis.
///
/// `CANOPUS class` (a class-only call, no structure and no formula) is Level 4 here, as in the Attributio reference tool, so it stays out of the
/// publication tables. Schymanski et al. 2014 would call a class assignment Level 3 and keep Level 4 for a unique molecular formula; moving it
/// changes every export and identity count that the reference goldens fix. MS2DeepScore is not computed (see `ms2_engine`), so it has no basis.
pub fn basis_level(basis: &str) -> i32 {
    match basis {
        "reference standard" => 1,
        "spectral library" => 2,
        "putative library" | "CSI:FingerID" => 3,
        "CANOPUS class" => 4,
        _ => 0,
    }
}

impl Default for AnnotationSettings {
    fn default() -> Self {
        Self {
            spec_sim_min: 0.70,
            spec_peaks_min: 4.0,
            engine_sim_min: 0.75,
            engine_dl_sim_min: 0.85,
            engine_mass_tol_ppm: 15.0,
            use_ms2_engine: false,
            putative_library_level3: false,
            library_conflict: "best_match".into(),
            library_conflict_margin: 0.05,
            gnps_mass_tol_ppm: 15.0,
            mzlib_mass_tol_ppm: 15.0,
            cosmic_min: 0.64,
            canopus_min: 0.70,
            rt_group_tol_min: 0.20,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_of_the_bases_follow_the_documented_cascade() {
        assert_eq!(basis_level("reference standard"), 1);
        assert_eq!(basis_level("spectral library"), 2);
        assert_eq!(basis_level("CSI:FingerID"), 3);
        // class-only calls are Level 4 in this tool (deviation from Schymanski 2014, see `basis_level`)
        assert_eq!(basis_level("CANOPUS class"), 4);
        // MS2DeepScore is never computed: it has neither a level nor a label
        assert_eq!(basis_level("MS2DeepScore"), 0);
        let s = AnnotationSettings::default();
        assert_eq!(s.label("MS2DeepScore"), s.label("none"));
    }
}
