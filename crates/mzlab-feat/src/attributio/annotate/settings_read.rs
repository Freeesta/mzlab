// Ported from Attributio annotate.py.
//! Reading `[annotation]` of `campaign.toml` into `AnnotationSettings`, and the text of each basis.

use super::settings::AnnotationSettings;
use super::settings::LIBRARY_CONFLICT_RULES;
use crate::attributio::campaign::Campaign;
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::toml::Value;
use crate::sortc::SortC;

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
            bad.sortc();
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
}

impl AnnotationSettings {
    /// The text of a basis, with the thresholds filled in.
    pub fn label(&self, basis: &str) -> String {
        match basis {
            "reference standard" => "Confirmed by reference standard: mass + RT + MS/MS (L1)".into(),
            "spectral library" => "Spectral library match (L2a)".into(),
            "putative library" => "Putative library match (analog/in-silico, L3a)".into(),
            "MS2DeepScore" => format!(
                "AI Structural Similarity (MS2DeepScore ≥ {:.2}, L3a)",
                self.engine_dl_sim_min
            ),
            "CSI:FingerID" => format!(
                "CSI:FingerID structure (COSMIC conf ≥ {:.2}, ~10% FDR, L3a)",
                self.cosmic_min
            ),
            "name conflict" => "Spectral library matches of different structures that the spectrum does not tell apart (best match ahead by less than the margin): named by the best match (L3a)".into(),
            "expert review" => "Spectral library match judged insufficient by the chemist (L3a)".into(),
            "diagnostic evidence" => "Structure supported by diagnostic evidence, judged by the chemist on the MS/MS spectrum (L2b)".into(),
            "isomers not discriminated" => "Spectral library match of the same compound at two or more retention times: the spectrum does not tell the isomers apart (L3a)".into(),
            "RT isomer" => "Spectral library match at a retention time other than the reference standard's: another isomer (L3a)".into(),
            "suspect-RT" => "Reference standard, mass + RT only, MS/MS not confirmed (suspect-RT, L0)".into(),
            "CANOPUS class" => format!("CANOPUS class only, probability ≥ {:.2} (L4)", self.canopus_min),
            _ => "Unannotated (DETECTED)".into(),
        }
    }
}
