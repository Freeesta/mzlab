// Ported from Attributio annotate.py.
//! The thresholds of the cascade (`AnnotationSettings`) with Attributio's defaults, read from `[annotation]` of `campaign.toml`, and the text of each basis.

use crate::attributio::campaign::Campaign;
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::toml::Value;

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

const KNOWN: [&str; 14] = [
    "spec_sim_min",
    "spec_peaks_min",
    "engine_sim_min",
    "engine_dl_sim_min",
    "engine_mass_tol_ppm",
    "use_ms2_engine",
    "putative_library_level3",
    "library_conflict",
    "library_conflict_margin",
    "gnps_mass_tol_ppm",
    "mzlib_mass_tol_ppm",
    "cosmic_min",
    "canopus_min",
    "rt_group_tol_min",
];

impl AnnotationSettings {
    /// The defaults with the keys of `[annotation]`; an unknown key or a library conflict rule that does not exist is a `ValueError`.
    pub fn for_campaign(c: &Campaign) -> Result<Self> {
        let t = &c.annotation;
        let mut bad: Vec<&str> = t.keys().filter(|k| !KNOWN.contains(k)).collect();
        if !bad.is_empty() {
            bad.sort_unstable();
            return Err(CampaignError::value(format!(
                "{} [annotation]: unknown keys {}",
                c.config_name(),
                crate::attributio::error::py_list(&bad)
            )));
        }
        let mut s = Self::default();
        let num = |k: &str, d: f64| t.get(k).and_then(Value::as_f64).unwrap_or(d);
        let flag = |k: &str, d: bool| t.get(k).and_then(Value::as_bool).unwrap_or(d);
        s.spec_sim_min = num("spec_sim_min", s.spec_sim_min);
        s.spec_peaks_min = num("spec_peaks_min", s.spec_peaks_min);
        s.engine_sim_min = num("engine_sim_min", s.engine_sim_min);
        s.engine_dl_sim_min = num("engine_dl_sim_min", s.engine_dl_sim_min);
        s.engine_mass_tol_ppm = num("engine_mass_tol_ppm", s.engine_mass_tol_ppm);
        s.use_ms2_engine = flag("use_ms2_engine", s.use_ms2_engine);
        s.putative_library_level3 = flag("putative_library_level3", s.putative_library_level3);
        if let Some(v) = t.get("library_conflict").and_then(Value::as_str) {
            s.library_conflict = v.to_string();
        }
        s.library_conflict_margin = num("library_conflict_margin", s.library_conflict_margin);
        s.gnps_mass_tol_ppm = num("gnps_mass_tol_ppm", s.gnps_mass_tol_ppm);
        s.mzlib_mass_tol_ppm = num("mzlib_mass_tol_ppm", s.mzlib_mass_tol_ppm);
        s.cosmic_min = num("cosmic_min", s.cosmic_min);
        s.canopus_min = num("canopus_min", s.canopus_min);
        s.rt_group_tol_min = num("rt_group_tol_min", s.rt_group_tol_min);
        if !LIBRARY_CONFLICT_RULES.contains(&s.library_conflict.as_str()) {
            return Err(CampaignError::value(format!(
                "{} [annotation] library_conflict: '{}' not in ('best_match', 'level3')",
                c.config_name(),
                s.library_conflict
            )));
        }
        Ok(s)
    }

    /// The text of a basis, with the thresholds filled in.
    pub fn label(&self, basis: &str) -> String {
        match basis {
            "reference standard" => "Confirmed by reference standard: mass + RT + MS/MS (Lvl 1)".into(),
            "spectral library" => "Spectral library match (Lvl 2a)".into(),
            "putative library" => "Putative library match (analog/in-silico, Lvl 3)".into(),
            "MS2DeepScore" => format!(
                "AI Structural Similarity (MS2DeepScore ≥ {:.2}, Lvl 3)",
                self.engine_dl_sim_min
            ),
            "CSI:FingerID" => format!(
                "CSI:FingerID structure (COSMIC conf ≥ {:.2}, ~10% FDR, Lvl 3)",
                self.cosmic_min
            ),
            "name conflict" => "Spectral library matches of different structures that the spectrum does not tell apart (best match ahead by less than the margin): named by the best match (Lvl 3)".into(),
            "expert review" => "Spectral library match judged insufficient by the chemist (Lvl 3)".into(),
            "diagnostic evidence" => "Structure supported by diagnostic evidence, judged by the chemist on the MS/MS spectrum (Lvl 2b)".into(),
            "isomers not discriminated" => "Spectral library match of the same compound at two or more retention times: the spectrum does not tell the isomers apart (Lvl 3)".into(),
            "RT isomer" => "Spectral library match at a retention time other than the reference standard's: another isomer (Lvl 3)".into(),
            "suspect-RT" => "Reference standard, mass + RT only, MS/MS not confirmed (suspect-RT, Lvl 0)".into(),
            "CANOPUS class" => format!("CANOPUS class only, probability ≥ {:.2} (Lvl 4)", self.canopus_min),
            _ => "Unannotated (DETECTED)".into(),
        }
    }
}

/// The level that follows from a basis.
pub fn basis_level(basis: &str) -> i32 {
    match basis {
        "reference standard" => 1,
        "spectral library" => 2,
        "putative library" | "MS2DeepScore" | "CSI:FingerID" => 3,
        "CANOPUS class" => 4,
        _ => 0,
    }
}
