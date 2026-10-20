// Ported from Attributio annotate.py.
//! The gates of each source and the basis of the level: the first that holds wins (reference standard, spectral library, putative library, CSI:FingerID,
//! suspect-RT, none). Also the structure that a feature carries (SIRIUS, GNPS2 exact, mzMine) and the sorbents and sites it was detected in.

use super::settings::{basis_level, AnnotationSettings};
use super::types::Feature;
use crate::attributio::campaign::Campaign;
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::readers::standards::{LEVEL1_STATUSES, SUPPORTIVE_STATUSES};

/// The sources of a level, in the order of the reference tool.
pub const SOURCE_NAMES: [&str; 7] = [
    "reference-std",
    "SIRIUS-spectral",
    "GNPS2",
    "MS2-Engine",
    "mzMine-library",
    "CSI:FingerID",
    "reference-RT",
];

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// GNPS2 propagated or rule-based annotations (`Candidate ...`, `... (known structural isomers ...)`, `... (delta mass: ...)`): not reference spectra.
pub fn is_propagated(name: &str) -> bool {
    let low = name.to_lowercase();
    let head = low.trim_start();
    let candidate = head
        .strip_prefix("candidate")
        .is_some_and(|rest| !rest.chars().next().is_some_and(is_word));
    candidate || low.contains("known structural isomers") || low.contains("(delta mass:")
}

fn is_putative(name: &str, s: &AnnotationSettings) -> bool {
    s.putative_library_level3
        && (name.to_lowercase().starts_with("spectral match to") || is_propagated(name))
}

fn at_least(v: Option<f64>, min: f64) -> bool {
    v.is_some_and(|x| x >= min)
}

/// Sets the gates of one feature, then its basis, level and label.
fn gate(f: &mut Feature, s: &AnnotationSettings) {
    let status = f.std.as_ref().map(|m| m.status.as_str());
    let std_named = f.std.as_ref().is_some_and(|m| m.name.is_some());
    f.gates.l1 = status.is_some_and(|x| LEVEL1_STATUSES.contains(&x));
    f.gates.std_rt = status.is_some_and(|x| SUPPORTIVE_STATUSES.contains(&x)) && std_named;
    let spec = f.spec.name.as_deref().filter(|_| {
        at_least(f.spec.sim, s.spec_sim_min) && at_least(f.spec.peaks, s.spec_peaks_min)
    });
    let gnps = f.gnps.name.as_deref().filter(|_| {
        at_least(f.gnps.score, s.spec_sim_min)
            && at_least(f.gnps.peaks, s.spec_peaks_min)
            && f.gnps.exact
    });
    let mzlib = f.mzlib.name.as_deref().filter(|_| {
        at_least(f.mzlib.score, s.spec_sim_min)
            && at_least(f.mzlib.peaks, s.spec_peaks_min)
            && f.mzlib.exact
    });
    f.gates.spec_putative = spec.is_some_and(|n| is_putative(n, s));
    f.gates.sirius_spectral = spec.is_some_and(|n| !is_putative(n, s) && !is_propagated(n));
    f.gates.gnps_putative = gnps.is_some_and(|n| is_putative(n, s));
    f.gates.gnps2 = gnps.is_some_and(|n| !is_putative(n, s) && !is_propagated(n));
    f.gates.mzlib_putative = mzlib.is_some_and(|n| is_putative(n, s));
    f.gates.mzmine_library = mzlib.is_some_and(|n| !is_putative(n, s) && !is_propagated(n));
    f.gates.csi = f.csi.name.is_some() && at_least(f.csi.cosmic, s.cosmic_min);
}

fn basis_of(f: &Feature) -> &'static str {
    let g = &f.gates;
    let lib = g.sirius_spectral || g.gnps2 || g.mzmine_library;
    let putative = g.spec_putative || g.gnps_putative || g.mzlib_putative;
    if g.l1 {
        "reference standard"
    } else if lib {
        "spectral library"
    } else if putative {
        "putative library"
    } else if g.csi {
        "CSI:FingerID"
    } else if g.std_rt {
        "suspect-RT"
    } else {
        "none"
    }
}

fn skeleton(key: Option<&str>) -> Option<String> {
    let k = key?.trim().to_uppercase();
    (k.chars().count() >= 14).then(|| k.chars().take(14).collect())
}

/// The connectivity block of the InChIKey of a source, as `inchikey_skeleton` of the reference tool gives it.
pub fn inchikey_block(key: Option<&str>) -> Option<String> {
    skeleton(key)
}

fn structure(f: &mut Feature) {
    let sir = skeleton(f.csi.inchikey2d.as_deref());
    let gn = skeleton(f.gnps.inchikey.as_deref().filter(|_| f.gnps.exact));
    let mz = skeleton(f.mzlib.inchikey.as_deref());
    f.inchikey_skeleton = sir.clone().or(gn.clone()).or(mz.clone());
    f.smiles = if sir.is_some() {
        f.csi.smiles.clone()
    } else if gn.is_some() {
        f.gnps.smiles.clone()
    } else {
        f.mzlib.smiles.clone()
    };
    f.inchikey_source = if sir.is_some() {
        Some("SIRIUS")
    } else if gn.is_some() {
        Some("GNPS")
    } else if mz.is_some() {
        Some("mzMine-library")
    } else {
        None
    }
    .map(String::from);
}

fn where_detected(f: &mut Feature, c: &Campaign, sites: &[String]) -> Result<()> {
    if c.sorbents.len() < 2 {
        return Err(CampaignError::value("IndexError: list index out of range"));
    }
    let any = |f: &Feature, s: &str| f.any_sorbent.iter().any(|(k, v)| k == s && *v);
    let (a, b) = (&c.sorbents[0], &c.sorbents[1]);
    f.sorbent_marker = if any(f, a) && any(f, b) {
        "Both".into()
    } else if any(f, a) {
        a.clone()
    } else {
        b.clone()
    };
    f.sites = sites.iter().filter(|s| f.in_site(s)).cloned().collect();
    f.n_sites = f.sites.len();
    Ok(())
}

/// Gates, basis, level and label of every feature, and the structure, sorbent marker, sites and sources.
pub fn apply_gates(
    feats: &mut [Feature],
    c: &Campaign,
    sites: &[String],
    s: &AnnotationSettings,
) -> Result<()> {
    for f in feats.iter_mut() {
        gate(f, s);
        let basis = basis_of(f);
        f.level_basis = basis.into();
        f.level = basis_level(basis);
        f.level_label = s.label(basis);
        f.high_conf_struct = f.gates.csi && f.level == 3;
        structure(f);
        where_detected(f, c, sites)?;
        let flags = [
            f.gates.l1,
            f.gates.sirius_spectral,
            f.gates.gnps2,
            false,
            f.gates.mzmine_library,
            f.gates.csi,
            f.gates.std_rt,
        ];
        f.sources = SOURCE_NAMES
            .iter()
            .zip(flags)
            .filter(|(_, on)| *on)
            .map(|(n, _)| *n)
            .collect();
    }
    Ok(())
}
