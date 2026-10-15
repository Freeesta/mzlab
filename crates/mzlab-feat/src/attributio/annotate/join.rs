// Ported from Attributio annotate.py.
//! The join of the sources on `(pol, id)`: the features detected at some of the sites, with the rows of SIRIUS, GNPS2, mzMine and the reference standards
//! that belong to each. Rows of the sorbent twins that were joined into another feature move to the kept row first (the best hit stays).

use super::settings::AnnotationSettings;
use super::types::{CsiEvidence, Feature, GnpsEvidence, MzlibEvidence, SpecEvidence};
use crate::attributio::campaign::{Campaign, POLARITIES};
use crate::attributio::detection::Detection;
use crate::attributio::error::Result;
use crate::attributio::readers::gnps::{GnpsHit, GnpsNode};
use crate::attributio::readers::mzmine::MzmineHit;
use crate::attributio::readers::sirius::{Canopus, SpectralMatch, Structure};
use crate::attributio::readers::standards::StandardMatch;
use crate::attributio::sorbents::remap_ids;
use crate::attributio::sources::Sources;
use std::collections::HashMap;

/// `detection_table` of both polarities for some sites, keeping the features detected at one of them at least.
pub fn detected(c: &Campaign, src: &Sources, sites: &[String]) -> Result<Vec<Detection>> {
    let mut out = Vec::new();
    for pol in POLARITIES {
        if let Some(rows) = src.detection(c, pol, sites) {
            out.extend(
                rows?
                    .into_iter()
                    .filter(|d| d.in_site.iter().any(|(_, v)| *v)),
            );
        }
    }
    Ok(out)
}

/// The source tables with the rows of joined twins moved to the kept row.
pub struct Remapped {
    pub canopus: Vec<Canopus>,
    pub structure: Vec<Structure>,
    pub spectral: Vec<SpectralMatch>,
    pub gnps: Vec<GnpsHit>,
    pub mzmine: Vec<MzmineHit>,
    pub network: Vec<GnpsNode>,
}

fn flag(b: bool) -> Option<f64> {
    Some(if b { 1.0 } else { 0.0 })
}

/// `_remap_sources`: a pair of twins with a hit each keeps the better hit, by the score of each source.
pub fn remap_sources(src: &Sources) -> Remapped {
    let mut r = Remapped {
        canopus: src.sirius.canopus.clone(),
        structure: src.sirius.structure.clone(),
        spectral: src.sirius.spectral.clone(),
        gnps: src
            .pols
            .iter()
            .flat_map(|p| p.gnps_library.clone())
            .collect(),
        mzmine: src.pols.iter().flat_map(|p| p.mzmine.clone()).collect(),
        network: src
            .pols
            .iter()
            .flat_map(|p| p.gnps_network.clone())
            .collect(),
    };
    for p in &src.pols {
        let (tm, pol) = (p.twins.as_slice(), p.pol.as_str());
        if tm.is_empty() {
            continue;
        }
        r.canopus = remap_ids(
            r.canopus,
            tm,
            pol,
            |x: &Canopus| (x.pol.as_str(), x.id),
            |x, id| x.id = id,
            |x| vec![x.probs.iter().flatten().cloned().reduce(f64::max)],
            false,
        );
        r.structure = remap_ids(
            r.structure,
            tm,
            pol,
            |x: &Structure| (x.pol.as_str(), x.id),
            |x, id| x.id = id,
            |x| vec![x.cosmic],
            false,
        );
        r.spectral = remap_ids(
            r.spectral,
            tm,
            pol,
            |x: &SpectralMatch| (x.pol.as_str(), x.id),
            |x, id| x.id = id,
            |x| vec![x.sim],
            false,
        );
        r.gnps = remap_ids(
            r.gnps,
            tm,
            pol,
            |x: &GnpsHit| (x.pol.as_str(), x.id),
            |x, id| x.id = id,
            |x| vec![flag(x.exact), x.score],
            false,
        );
        r.mzmine = remap_ids(
            r.mzmine,
            tm,
            pol,
            |x: &MzmineHit| (x.pol.as_str(), x.id),
            |x, id| x.id = id,
            |x| vec![x.score],
            false,
        );
        r.network = remap_ids(
            r.network,
            tm,
            pol,
            |x: &GnpsNode| (x.pol.as_str(), x.id),
            |x, id| x.id = id,
            |_| Vec::new(),
            false,
        );
    }
    r
}

fn index<T>(rows: &[T], key: impl Fn(&T) -> (&str, i64)) -> HashMap<(String, i64), usize> {
    let mut m = HashMap::new();
    for (n, r) in rows.iter().enumerate() {
        let (p, id) = key(r);
        m.entry((p.to_string(), id)).or_insert(n);
    }
    m
}

fn gated_class(f: &mut Feature, s: &AnnotationSettings) {
    // a class below canopus_min is not shown; a class also needs its parent shown
    const PARENT: [Option<usize>; 5] = [None, Some(0), Some(1), None, Some(3)];
    for k in 0..5 {
        let ok = f.class_probs[k].is_some_and(|p| p >= s.canopus_min)
            && PARENT[k].is_none_or(|q| f.classes[q].is_some());
        if !ok {
            f.classes[k] = None;
        }
    }
}

fn hit_ppm(mz: Option<f64>, precursor: Option<f64>) -> Option<f64> {
    match (mz, precursor) {
        (Some(m), Some(p)) => Some((m - p).abs() / p * 1e6),
        _ => None,
    }
}

/// One row per detected feature with the evidence of every source.
pub fn join(
    det: Vec<Detection>,
    rm: &Remapped,
    std: &[StandardMatch],
    s: &AnnotationSettings,
) -> Vec<Feature> {
    let cano = index(&rm.canopus, |x| (x.pol.as_str(), x.id));
    let st = index(&rm.structure, |x| (x.pol.as_str(), x.id));
    let sp = index(&rm.spectral, |x| (x.pol.as_str(), x.id));
    let gn = index(&rm.gnps, |x| (x.pol.as_str(), x.id));
    let ml = index(&rm.mzmine, |x| (x.pol.as_str(), x.id));
    let nw = index(&rm.network, |x| (x.pol.as_str(), x.id));
    let sd = index(std, |x| (x.pol.as_str(), x.id));
    det.into_iter()
        .map(|d| {
            let key = (d.pol.clone(), d.id);
            let mut f = Feature {
                pol: d.pol,
                id: d.id,
                mz: d.mz,
                rt: d.rt,
                in_site: d.in_site,
                any_sorbent: d.any_sorbent,
                ..Feature::default()
            };
            if let Some(&n) = cano.get(&key) {
                f.classes = rm.canopus[n].classes.clone();
                f.class_probs = rm.canopus[n].probs;
            }
            gated_class(&mut f, s);
            if let Some(&n) = st.get(&key) {
                let x = &rm.structure[n];
                f.csi = CsiEvidence {
                    name: x.name.clone(),
                    formula: x.formula.clone(),
                    xlogp: x.xlogp,
                    conf: x.conf,
                    cosmic: x.cosmic,
                    inchikey2d: x.inchikey2d.clone(),
                    smiles: x.smiles.clone(),
                };
            }
            if let Some(&n) = sp.get(&key) {
                let x = &rm.spectral[n];
                f.spec = SpecEvidence {
                    name: x.name.clone(),
                    sim: x.sim,
                    peaks: x.peaks,
                    inchikey2d: x.inchikey2d.clone(),
                };
            }
            if let Some(&n) = gn.get(&key) {
                let x = &rm.gnps[n];
                f.gnps = GnpsEvidence {
                    name: Some(x.name.clone()),
                    score: x.score,
                    peaks: x.peaks,
                    massdiff: x.massdiff,
                    exact: x.exact,
                    inchikey: x.inchikey.clone(),
                    smiles: x.smiles.clone(),
                };
            }
            if let Some(&n) = ml.get(&key) {
                let x = &rm.mzmine[n];
                f.mzlib = MzlibEvidence {
                    name: Some(x.name.clone()),
                    score: x.score,
                    peaks: x.peaks,
                    precursor_mz: x.precursor_mz,
                    adduct: x.adduct.clone(),
                    formula: x.formula.clone(),
                    inchikey: x.inchikey.clone(),
                    smiles: x.smiles.clone(),
                    ..MzlibEvidence::default()
                };
            }
            f.mzlib.massdiff_ppm = hit_ppm(f.mz, f.mzlib.precursor_mz);
            f.mzlib.exact = f
                .mzlib
                .massdiff_ppm
                .is_some_and(|p| p <= s.mzlib_mass_tol_ppm);
            if let Some(&n) = nw.get(&key) {
                f.gnps_component = rm.network[n].component;
                f.gnps_component_size = Some(rm.network[n].component_size);
            }
            f.std = sd.get(&key).map(|&n| std[n].clone());
            f
        })
        .collect()
}
