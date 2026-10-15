// Ported from Attributio campaign.py, annotate.py.
//! The common entry: `load_sources` reads everything a campaign holds for both polarities (the merged feature tables with the per-sorbent blanks and twins,
//! the mzMine, GNPS2 and SIRIUS annotations, the FBMN spectra, the standards library and its spectra, the chemist's decisions) into memory, with the counts per
//! source that the panel and the tests show. Nothing is decided here: the cascade of levels reads `Sources` without reading the files again.

use super::campaign::{Campaign, POLARITIES};
use super::detection::{detection_table, Detection};
use super::error::{CampaignError, Result};
use super::frame::Frame;
use super::io::{join, Io, MemFiles};
use super::readers::feature_decisions::{load_feature_decisions, DecisionTable, FeatureKey};
use super::readers::gnps::{load_gnps_library, load_gnps_network, GnpsHit, GnpsNode};
use super::readers::ms2::{
    feature_mgf, read_feature_spectra, read_library_spectra, RefLibrary, Spectrum,
};
use super::readers::mzmine::{load_mzmine_library, MzmineHit};
use super::readers::sirius::{load_sirius_tables, SiriusTables};
use super::readers::standards::{
    load_decisions, load_standards_library, mass_rt_candidates, match_reference_standards,
    Comparer, FeatureRef, Level1Decision, Ms2Check, StandardMatch, StandardsSettings, Tolerances,
};
use super::sorbents::{add_sorbent_blanks, collapse_twins, TwinMap};
use std::collections::HashMap;

/// The mass tolerance of a GNPS2 hit to count as the compound and not an analogue.
pub const GNPS_MASS_TOL_PPM: f64 = 15.0;

/// The merged feature table of a polarity: the CSV of the merged folder that has `:feature_state` columns and the most columns (not the quant, edges,
/// network or annotation exports); robust to file names.
pub fn merged_table_path(c: &Campaign, io: &Io, pol: &str) -> Result<String> {
    let dir = c.merged_dir(pol);
    let mut names: Vec<String> = io
        .list(&dir)
        .into_iter()
        .filter(|e| !e.is_dir && e.name.ends_with(".csv"))
        .map(|e| e.name)
        .collect();
    names.sort();
    let mut best: Option<(String, usize)> = None;
    for name in names {
        let low = name.to_lowercase();
        if ["quant", "edges", "network", "annotation"]
            .iter()
            .any(|k| low.contains(k))
        {
            continue;
        }
        let Some(text) = io.text(&join(&dir, &name)) else {
            continue;
        };
        let head = Frame::header(&text, ',');
        if head.iter().any(|h| h.ends_with(":feature_state"))
            && head.len() > best.as_ref().map_or(0, |b| b.1)
        {
            best = Some((name, head.len()));
        }
    }
    best.map(|(n, _)| join(&dir, &n)).ok_or_else(|| {
        CampaignError::not_found(format!("No full merged feature table found in {dir}"))
    })
}

/// The merged feature table of a polarity with the blank of each sorbent and, when the campaign asks, the sorbent twins joined.
pub fn load_feature_table(
    c: &Campaign,
    io: &Io,
    pol: &str,
    warnings: &mut Vec<String>,
) -> Result<(Frame, TwinMap)> {
    let path = merged_table_path(c, io, pol)?;
    let text = io.text(&path).ok_or_else(|| {
        CampaignError::not_found(format!("No full merged feature table found in {path}"))
    })?;
    let mut f = Frame::parse(&text, ',')?;
    let mut twins = Vec::new();
    if c.has_sorbent_tables() {
        add_sorbent_blanks(c, &mut f, io, pol, warnings)?;
        if c.collapse_sorbent_twins {
            twins = collapse_twins(c, &mut f, pol)?;
        }
    }
    Ok((f, twins))
}

/// What one polarity holds.
#[derive(Clone, Debug)]
pub struct PolSources {
    pub pol: String,
    pub table: Frame,
    pub twins: TwinMap,
    pub mzmine: Vec<MzmineHit>,
    pub gnps_library: Vec<GnpsHit>,
    pub gnps_network: Vec<GnpsNode>,
    pub feature_spectra: HashMap<i64, Spectrum>,
    pub library_spectra: RefLibrary,
}

impl PolSources {
    /// The features as the matching of decisions and standards sees them.
    pub fn feature_keys(&self) -> Vec<FeatureKey> {
        let (id, mz, rt) = (
            self.table.col("id"),
            self.table.col("mz"),
            self.table.col("rt"),
        );
        (0..self.table.len())
            .map(|r| FeatureKey {
                pol: self.pol.clone(),
                id: id.and_then(|k| self.table.int(r, k)).unwrap_or(0),
                mz: mz.and_then(|k| self.table.num(r, k)),
                rt: rt.and_then(|k| self.table.num(r, k)),
            })
            .collect()
    }
}

/// Everything the campaign holds, read once.
#[derive(Clone, Debug)]
pub struct Sources {
    pub pols: Vec<PolSources>,
    pub sirius: SiriusTables,
    pub standards_library: Frame,
    pub standards: Option<StandardsSettings>,
    pub level1_decisions: Vec<Level1Decision>,
    pub decisions: DecisionTable,
    pub warnings: Vec<String>,
    /// Rows per source, in reading order: `("features POS", 15)`, `("gnps hits NEG", 0)`, ...
    pub counts: Vec<(String, usize)>,
}

impl Sources {
    pub fn pol(&self, pol: &str) -> Option<&PolSources> {
        self.pols.iter().find(|p| p.pol == pol)
    }
    pub fn count(&self, key: &str) -> Option<usize> {
        self.counts.iter().find(|(k, _)| k == key).map(|(_, v)| *v)
    }
    /// The detection table of a polarity for some sites (`None` when the polarity has no table).
    pub fn detection(
        &self,
        c: &Campaign,
        pol: &str,
        sites: &[String],
    ) -> Option<Result<Vec<Detection>>> {
        self.pol(pol)
            .map(|p| detection_table(c, &p.table, pol, sites))
    }
    /// The Level 1 matches of a polarity (mass, RT and MS/MS through `compare`, then the chemist's decisions); empty without a standards library.
    pub fn match_standards(&mut self, pol: &str, compare: Comparer) -> Result<Vec<StandardMatch>> {
        let (Some(set), Some(p)) = (
            self.standards.clone(),
            self.pols.iter().find(|p| p.pol == pol),
        ) else {
            return Ok(Vec::new());
        };
        let i = StandardsSettings::index(pol);
        let features = feature_refs(p);
        let tol = Tolerances {
            mass_tol_ppm: set.mass_tol_ppm,
            rt_tol_min: set.rt_tol_min,
            rt_column: &set.rt_column[i],
            adducts: Some(&set.adducts[i]),
        };
        let ms2 = Ms2Check {
            feature_spectra: &p.feature_spectra,
            library_spectra: &p.library_spectra,
            compare,
            min_cosine: set.ms2_min_cosine,
            min_matched: set.ms2_min_matched,
            tol_da: set.ms2_tol_da,
            precursor_tol_ppm: set.precursor_tol_ppm,
        };
        match_reference_standards(
            &features,
            pol,
            &self.standards_library,
            &tol,
            &ms2,
            &self.level1_decisions,
            &mut self.warnings,
        )
    }
}

fn feature_refs(p: &PolSources) -> Vec<FeatureRef> {
    p.feature_keys()
        .into_iter()
        .map(|k| FeatureRef {
            id: k.id,
            mz: k.mz,
            rt: k.rt,
        })
        .collect()
}

fn read_pol(
    c: &Campaign,
    io: &Io,
    pol: &str,
    settings: Option<&StandardsSettings>,
    warnings: &mut Vec<String>,
) -> Result<Option<PolSources>> {
    let (table, twins) = match load_feature_table(c, io, pol, warnings) {
        Ok(t) => t,
        Err(e) if e.kind == super::error::Kind::FileNotFound => {
            warnings.push(format!("[{}] {pol}: {}", c.id, e.msg));
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    let feature_spectra = feature_mgf(c, io, pol)
        .and_then(|p| io.text(&p))
        .map(|t| read_feature_spectra(&t))
        .unwrap_or_default();
    let library_spectra = match settings
        .and_then(|s| s.ms2_library[StandardsSettings::index(pol)].as_ref())
        .and_then(|p| io.text(p))
    {
        Some(t) => read_library_spectra(&t)?,
        None => RefLibrary::new(),
    };
    Ok(Some(PolSources {
        pol: pol.to_string(),
        mzmine: load_mzmine_library(c, io, &table, pol)?,
        gnps_library: load_gnps_library(c, io, pol, GNPS_MASS_TOL_PPM)?,
        gnps_network: load_gnps_network(c, io, pol)?,
        table,
        twins,
        feature_spectra,
        library_spectra,
    }))
}

fn count_rows(s: &Sources, pol_counts: bool) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    for p in &s.pols {
        let n = |k: &str, v: usize| (format!("{k} {}", p.pol), v);
        out.push(n("features", p.table.len()));
        out.push(n("twins joined", p.twins.len()));
        out.push(n("mzmine hits", p.mzmine.len()));
        out.push(n("gnps hits", p.gnps_library.len()));
        out.push(n("gnps nodes", p.gnps_network.len()));
        out.push(n("feature spectra", p.feature_spectra.len()));
        out.push(n(
            "library spectra",
            super::readers::ms2::library_size(&p.library_spectra),
        ));
        let sirius = |rows: usize| rows;
        let _ = sirius;
    }
    if pol_counts {
        for pol in POLARITIES {
            let by_pol = |it: &mut dyn Iterator<Item = &str>| it.filter(|p| *p == pol).count();
            out.push((
                format!("sirius canopus {pol}"),
                by_pol(&mut s.sirius.canopus.iter().map(|r| r.pol.as_str())),
            ));
            out.push((
                format!("sirius structures {pol}"),
                by_pol(&mut s.sirius.structure.iter().map(|r| r.pol.as_str())),
            ));
            out.push((
                format!("sirius spectral {pol}"),
                by_pol(&mut s.sirius.spectral.iter().map(|r| r.pol.as_str())),
            ));
        }
    }
    out.push(("standards library".into(), s.standards_library.len()));
    out.push(("level 1 decisions".into(), s.level1_decisions.len()));
    out.push(("feature decisions".into(), s.decisions.len()));
    out
}

/// Reads the campaign's sources for POS and NEG. A polarity without a merged feature table is left out with a warning; with none at all the error of the
/// first polarity is returned.
pub fn load_sources(c: &Campaign, io: &Io) -> Result<Sources> {
    let mut warnings = Vec::new();
    let standards = StandardsSettings::from_campaign(c);
    let mut pols = Vec::new();
    for pol in POLARITIES {
        if let Some(p) = read_pol(c, io, pol, standards.as_ref(), &mut warnings)? {
            pols.push(p);
        }
    }
    if pols.is_empty() {
        let dir = c.merged_dir(POLARITIES[0]);
        return Err(CampaignError::not_found(format!(
            "No full merged feature table found in {dir}"
        )));
    }
    let standards_library = load_standards_library(c, io, &mut warnings)?;
    let level1_text = standards
        .as_ref()
        .and_then(|s| s.decisions.as_ref())
        .and_then(|p| io.text(p));
    let level1_path = standards
        .as_ref()
        .and_then(|s| s.decisions.clone())
        .unwrap_or_default();
    let decisions_text = c.feature_decisions_csv.as_ref().and_then(|p| io.text(p));
    let decisions_path = c.feature_decisions_csv.clone().unwrap_or_default();
    let mut s = Sources {
        sirius: load_sirius_tables(c, io)?,
        level1_decisions: load_decisions(level1_text.as_deref(), &level1_path)?,
        decisions: load_feature_decisions(decisions_text.as_deref(), &decisions_path)?,
        standards_library,
        standards,
        pols,
        warnings,
        counts: Vec::new(),
    };
    s.counts = count_rows(&s, true);
    let candidates = candidate_counts(&s);
    s.counts.extend(candidates);
    Ok(s)
}

/// The mass + RT candidates per polarity (before the MS/MS check), a count the panel shows next to the library.
fn candidate_counts(s: &Sources) -> Vec<(String, usize)> {
    let Some(set) = &s.standards else {
        return Vec::new();
    };
    s.pols
        .iter()
        .map(|p| {
            let i = StandardsSettings::index(&p.pol);
            let tol = Tolerances {
                mass_tol_ppm: set.mass_tol_ppm,
                rt_tol_min: set.rt_tol_min,
                rt_column: &set.rt_column[i],
                adducts: Some(&set.adducts[i]),
            };
            let n = mass_rt_candidates(&feature_refs(p), &p.pol, &s.standards_library, &tol)
                .map_or(0, |c| c.len());
            (format!("standard candidates {}", p.pol), n)
        })
        .collect()
}

/// `load_sources` over files held in memory (the files a page has already read, and the tests).
pub fn load_sources_mem(c: &Campaign, files: &MemFiles) -> Result<Sources> {
    let read = |p: &str| files.read(p);
    let list = |d: &str| files.list(d);
    load_sources(c, &Io::new(&read, &list))
}
