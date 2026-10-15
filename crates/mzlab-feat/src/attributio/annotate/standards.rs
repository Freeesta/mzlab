// Ported from Attributio annotate.py.
//! Level 1 for the detected features of both polarities: mass ≤ `mass_tol_ppm`, RT within `rt_tol_min` and MS/MS (cosine and fragments) against the
//! reference spectrum of that ion, then the chemist's Level 1 decisions. The spectra and the decision ids of the sorbent twins follow the kept row.

use crate::attributio::campaign::{Campaign, POLARITIES};
use crate::attributio::detection::Detection;
use crate::attributio::error::Result;
use crate::attributio::readers::ms2::{RefLibrary, Spectrum};
use crate::attributio::readers::standards::{
    match_reference_standards, Comparer, FeatureRef, Level1Decision, Ms2Check, Similarity,
    StandardMatch, StandardsSettings, Tolerances,
};
use crate::attributio::sources::Sources;
use mzlab_lib::scores::{gnps_cosine, Spec};
use std::collections::HashMap;

/// `standards_settings`: Level 1 settings of `[standards]`, `None` when the campaign has no reference-standard library.
pub fn standards_settings(c: &Campaign) -> Option<StandardsSettings> {
    StandardsSettings::from_campaign(c)
}

fn spec(s: &Spectrum) -> Spec {
    Spec {
        mz: s.mz.clone(),
        it: s.intensity.clone(),
        prec: s.precursor,
    }
}

/// The comparison of a feature spectrum with the reference spectra of a standard (`readers/ms2.py::similarity`): the best of the reference spectra whose
/// precursor is within `ppm`, by cosine, then reverse cosine, then matched fragments. The cosine is the GNPS one of `mzlab-lib`, equal to Attributio's.
pub fn gnps_comparer(
    sample: &Spectrum,
    lib: &RefLibrary,
    name: &str,
    ppm: f64,
    tol: f64,
) -> Option<Similarity> {
    let refs = lib.get(name.trim())?.iter().filter(|r| {
        r.precursor.is_finite()
            && sample.precursor.is_finite()
            && (sample.precursor - r.precursor).abs() / r.precursor * 1e6 <= ppm
    });
    let a = spec(sample);
    refs.map(|r| {
        let m = gnps_cosine(&a, &spec(r), tol, false);
        Similarity {
            cosine: m.cosine,
            reverse: m.reverse,
            matched: m.matched,
        }
    })
    .max_by(|x, y| {
        (x.cosine, x.reverse, x.matched)
            .partial_cmp(&(y.cosine, y.reverse, y.matched))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// A kept twin without its own spectrum takes the spectrum of the row joined into it.
fn remap_spectra(
    mut spectra: HashMap<i64, Spectrum>,
    twins: &[(i64, i64)],
) -> HashMap<i64, Spectrum> {
    for (gone, keep) in twins {
        if !spectra.contains_key(keep) {
            if let Some(s) = spectra.get(gone).cloned() {
                spectra.insert(*keep, s);
            }
        }
    }
    spectra
}

/// Decision ids of joined twins point to the kept row.
fn remap_level1(decisions: &[Level1Decision], src: &Sources) -> Vec<Level1Decision> {
    decisions
        .iter()
        .map(|d| {
            let tm = src.pol(&d.pol).map_or(&[][..], |p| p.twins.as_slice());
            let mut d = d.clone();
            if let Some((_, to)) = tm.iter().find(|(from, _)| *from == d.feature_id) {
                d.feature_id = *to;
            }
            d
        })
        .collect()
}

/// The Level 1 evaluation for the detected features of both polarities (empty without a standards library).
pub fn reference_standard_matches(
    c: &Campaign,
    src: &Sources,
    det: &[Detection],
    compare: Comparer,
    warnings: &mut Vec<String>,
) -> Result<Vec<StandardMatch>> {
    let Some(set) = standards_settings(c) else {
        return Ok(Vec::new());
    };
    let decisions = remap_level1(&src.level1_decisions, src);
    let mut out = Vec::new();
    for pol in POLARITIES {
        let Some(p) = src.pol(pol) else {
            continue;
        };
        let i = StandardsSettings::index(pol);
        let features: Vec<FeatureRef> = det
            .iter()
            .filter(|d| d.pol == pol)
            .map(|d| FeatureRef {
                id: d.id,
                mz: d.mz,
                rt: d.rt,
            })
            .collect();
        let tol = Tolerances {
            mass_tol_ppm: set.mass_tol_ppm,
            rt_tol_min: set.rt_tol_min,
            rt_column: &set.rt_column[i],
            adducts: Some(&set.adducts[i]),
        };
        let spectra = remap_spectra(p.feature_spectra.clone(), &p.twins);
        let ms2 = Ms2Check {
            feature_spectra: &spectra,
            library_spectra: &p.library_spectra,
            compare,
            min_cosine: set.ms2_min_cosine,
            min_matched: set.ms2_min_matched,
            tol_da: set.ms2_tol_da,
            precursor_tol_ppm: set.precursor_tol_ppm,
        };
        out.extend(match_reference_standards(
            &features,
            pol,
            &src.standards_library,
            &tol,
            &ms2,
            &decisions,
            warnings,
        )?);
    }
    Ok(out)
}
