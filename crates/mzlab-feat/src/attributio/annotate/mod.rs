// Ported from Attributio annotate.py.
//! The cascade of confidence levels, one feature-level table per campaign. `load_annotations` gives every feature detected at one of the sites at least the
//! level, sub-level, basis and reasons of Attributio 1.12.0 (after Schymanski et al., Environ. Sci. Technol. 2014, 48, 2097); the evidence of every source is
//! joined on `(pol, id)` (`join`), the gates are tested in order (`gates`) and the first that holds wins:
//!
//! 1. reference standard: mass + RT + MS/MS (`standards`);
//! 2. spectral library (2a): cosine ≥ `spec_sim_min` and ≥ `spec_peaks_min` shared ions on SIRIUS spectral, GNPS2 (exact) or mzMine (exact); libraries that
//!    name different structures are one library: the best match names the feature and stays 2a if it is ahead by the margin, else Level 3 `name conflict`
//!    (`naming`); 2b exists only as the chemist's decision (`decide`);
//! 3. CSI:FingerID structure with COSMIC ≥ `cosmic_min`; 4. CANOPUS class ≥ `canopus_min`; 0. none, or a standard matched on mass + RT only (`suspect-RT`).
//!
//! Then the retention-time rule (`rt`) and the explanation of each level (`reasons`). The curation of names is read through `Curation` (`curate`).

pub mod curate;
pub mod decide;
pub mod fingerprint;
pub mod gates;
pub mod join;
pub mod naming;
pub mod reasons;
pub mod rt;
pub mod settings;
mod settings_read;
pub mod standards;
pub mod text;
pub mod types;

pub use curate::{BasicCuration, Curation};
pub use fingerprint::annotations_fingerprint;
pub use settings::AnnotationSettings;
pub use standards::{gnps_comparer, reference_standard_matches, standards_settings};
pub use types::Feature;

use crate::attributio::campaign::{Campaign, POLARITIES};
use crate::attributio::error::Result;
use crate::attributio::readers::feature_decisions::{DecisionTable, FeatureKey};
use crate::attributio::readers::standards::Comparer;
use crate::attributio::sources::{Sources, GNPS_MASS_TOL_PPM};

/// The annotated features and what the run had to say.
#[derive(Clone, Debug, Default)]
pub struct Annotations {
    pub features: Vec<Feature>,
    pub warnings: Vec<String>,
}

/// Decision files keyed by `(pol, feature_id)`: ids of joined twins point to the kept row.
fn remap_feature_decisions(t: &DecisionTable, src: &Sources) -> DecisionTable {
    let mut out = t.clone();
    let (Some(kp), Some(kf)) = (
        t.columns.iter().position(|c| c == "pol"),
        t.columns.iter().position(|c| c == "feature_id"),
    ) else {
        return out;
    };
    for row in out.rows.iter_mut() {
        let Some(p) = src.pol(&row[kp]).filter(|p| !p.twins.is_empty()) else {
            continue;
        };
        if let Ok(v) = row[kf].trim().parse::<f64>() {
            let id = v as i64;
            let to = p
                .twins
                .iter()
                .find(|(from, _)| *from == id)
                .map_or(id, |t| t.1);
            row[kf] = to.to_string();
        }
    }
    out
}

fn every_feature(src: &Sources) -> Vec<FeatureKey> {
    POLARITIES
        .iter()
        .filter_map(|p| src.pol(p))
        .flat_map(|p| p.feature_keys())
        .collect()
}

fn annotate(
    c: &Campaign,
    src: &Sources,
    sites: &[String],
    s: &AnnotationSettings,
    cur: &dyn Curation,
    compare: Comparer,
    warnings: &mut Vec<String>,
) -> Result<Vec<Feature>> {
    let det = join::detected(c, src, sites)?;
    let remapped = join::remap_sources(src);
    let std = reference_standard_matches(c, src, &det, compare, warnings)?;
    let mut feats = join::join(det, &remapped, &std, s);
    gates::apply_gates(&mut feats, c, sites, s)?;
    naming::name_features(&mut feats, s, cur);
    let decisions = remap_feature_decisions(&src.decisions, src);
    decide::apply_decisions(
        &mut feats,
        &decisions,
        &every_feature(src),
        c,
        s,
        cur,
        warnings,
    );
    Ok(feats)
}

/// The features detected at one of `sites` (default: every site of the campaign), one row per `(pol, id)`. The identities are then made coherent in retention
/// time; the rule is evaluated on every site of the campaign whatever `sites` are loaded, so that an identity has the same name and level in every matrix.
/// `compare` is the spectrum comparison of the Level 1 check (`gnps_comparer`).
pub fn load_annotations(
    c: &Campaign,
    src: &Sources,
    sites: Option<&[String]>,
    cur: &dyn Curation,
    compare: Comparer,
) -> Result<Annotations> {
    let every: Vec<String> = c.site_keys().iter().map(|s| s.to_string()).collect();
    let sites: Vec<String> = sites.map_or_else(|| every.clone(), <[String]>::to_vec);
    let s = AnnotationSettings::for_campaign(c)?;
    let mut warnings = src.warnings.clone();
    if s.use_ms2_engine {
        warnings.push(
            "use_ms2_engine is not available in this engine: the MS2 Engine hits are ignored"
                .into(),
        );
    }
    if (s.gnps_mass_tol_ppm - GNPS_MASS_TOL_PPM).abs() > 0.0 {
        warnings.push(format!(
            "gnps_mass_tol_ppm = {} is not applied: the GNPS2 hits are read with {GNPS_MASS_TOL_PPM} ppm",
            s.gnps_mass_tol_ppm
        ));
    }
    let mut feats = annotate(c, src, &sites, &s, cur, compare, &mut warnings)?;
    let scope = if every.iter().all(|e| sites.contains(e)) {
        None
    } else {
        Some(annotate(c, src, &every, &s, cur, compare, &mut Vec::new())?)
    };
    let rows = rt::rt_coherence(c, src, scope.as_deref().unwrap_or(&feats), &s, cur)?;
    rt::apply_rt_coherence(&mut feats, &rows, &s, &mut warnings);
    reasons::level_reasons(&mut feats, &s);
    Ok(Annotations {
        features: feats,
        warnings,
    })
}
