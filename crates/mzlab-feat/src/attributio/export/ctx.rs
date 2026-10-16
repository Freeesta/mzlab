// Ported from Attributio diagnostics/schymanski_export.py.
//! What the exports of a campaign read once: the sources, the annotated features, the curation, and per feature the provenance of the GNPS2 hit, the
//! areas at the sites and the flag of the RT-logP fit (`build_feature_frame`).

use crate::attributio::annotate::join::remap_sources;
use crate::attributio::annotate::{gnps_comparer, load_annotations, Annotations, Feature};
use crate::attributio::campaign::{Campaign, POLARITIES};
use crate::attributio::checks::csi_level3;
use crate::attributio::curation::TableCuration;
use crate::attributio::detection::{has_site, site_area, Agg};
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::identity::{curated, identity_table, IdentityRow};
use crate::attributio::io::{join, Io};
use crate::attributio::readers::{desc_na_last, read_table};
use crate::attributio::rt_logp::build_batch_fit;
use crate::attributio::sources::load_sources;
use crate::attributio::Sources;
use std::collections::HashMap;

/// The first matrix of the campaign when none is asked, with its sites.
pub fn matrix_and_sites(c: &Campaign, matrix: Option<&str>) -> Result<(String, Vec<String>)> {
    let name = match matrix {
        Some(m) => m.to_string(),
        None => c
            .matrices
            .first()
            .map(|(k, _)| k.clone())
            .ok_or_else(|| CampaignError::campaign("the campaign declares no matrix"))?,
    };
    let sites = c
        .matrix_sites(&name)
        .ok_or_else(|| CampaignError::campaign(format!("unknown matrix: {name}")))?
        .to_vec();
    Ok((name, sites))
}

/// Everything an export reads from a campaign.
pub struct Loaded {
    pub src: Sources,
    pub ann: Annotations,
    pub cur: TableCuration,
    pub matrix: String,
    pub sites: Vec<String>,
}

impl Loaded {
    pub fn load(c: &Campaign, io: &Io, matrix: Option<&str>) -> Result<Loaded> {
        let (matrix, sites) = matrix_and_sites(c, matrix)?;
        let src = load_sources(c, io)?;
        let cur = TableCuration::load(c, io)?;
        let ann = load_annotations(c, &src, Some(&sites), &cur, &gnps_comparer)?;
        Ok(Loaded {
            src,
            ann,
            cur,
            matrix,
            sites,
        })
    }

    pub fn features(&self) -> &[Feature] {
        &self.ann.features
    }

    /// `identity_table(feat, sites, drop_groups=())`.
    pub fn all_identities(&self) -> Vec<IdentityRow> {
        identity_table(self.features(), &self.sites, &self.cur, &[])
    }
}

/// The GNPS2 hit of a feature as the provenance table of the library search holds it.
#[derive(Clone, Debug, Default)]
pub struct GnpsProv {
    pub library: Option<String>,
    pub adduct: Option<String>,
    pub shared_peaks: Option<f64>,
    pub mz_error_ppm: Option<f64>,
    pub mass_diff: Option<f64>,
    pub library_usi: Option<String>,
    pub inchikey: Option<String>,
}

fn text(g: &crate::attributio::Frame, r: usize, col: &str) -> Option<String> {
    g.col(col)
        .and_then(|c| g.str_na(r, c))
        .map(str::to_string)
}

fn library_label(g: &crate::attributio::Frame, r: usize) -> Option<String> {
    let org = text(g, r, "Organism").unwrap_or_default();
    let coll = text(g, r, "LibraryName").unwrap_or_default();
    let label = if org.is_empty() {
        coll
    } else if !coll.is_empty() && coll != org {
        format!("{org} ({coll})")
    } else {
        org
    };
    (!label.is_empty()).then_some(label)
}

/// `_load_gnps_provenance`: the best row (highest MQScore, the first of equals) of each feature of `merged_results_with_gnps.tsv`.
pub fn gnps_provenance(c: &Campaign, io: &Io) -> Result<HashMap<(String, i64), GnpsProv>> {
    let mut out = HashMap::new();
    for pol in POLARITIES {
        let path = join(
            &c.gnps_output_dir(pol),
            "nf_output/library/merged_results_with_gnps.tsv",
        );
        let Some(g) = read_table(io, &path, '\t')? else {
            continue;
        };
        let Some(kscan) = g.col("#Scan#") else {
            continue;
        };
        let num = |r: usize, n: &str| g.col(n).and_then(|k| g.num(r, k));
        let mut rows: Vec<usize> = (0..g.len()).filter(|&r| g.num(r, kscan).is_some()).collect();
        rows.sort_by(|&a, &b| desc_na_last(num(a, "MQScore"), num(b, "MQScore")));
        for r in rows {
            let id = g.num(r, kscan).unwrap_or_default() as i64;
            out.entry((pol.to_string(), id)).or_insert_with(|| GnpsProv {
                library: library_label(&g, r),
                adduct: text(&g, r, "Adduct"),
                shared_peaks: num(r, "SharedPeaks"),
                mz_error_ppm: num(r, "MZErrorPPM"),
                mass_diff: num(r, "MassDiff"),
                library_usi: text(&g, r, "library_usi"),
                inchikey: text(&g, r, "InChIKey"),
            });
        }
    }
    Ok(out)
}

/// One feature with what `build_feature_frame` adds to it.
pub struct FRow<'a> {
    pub f: &'a Feature,
    /// The curated identity of a Level 1-3 feature.
    pub identity: Option<String>,
    pub gp: GnpsProv,
    pub mz_method: Option<String>,
    pub mz_usi: Option<String>,
    /// Area at each site of the matrix (zero when the feature is not DETECTED).
    pub area: Vec<f64>,
    pub total_area: f64,
    pub rt_z: Option<f64>,
    pub rt_note: Option<String>,
}

impl FRow<'_> {
    pub fn matched_peaks(&self) -> Option<f64> {
        self.gp.shared_peaks.or(self.f.spec.peaks)
    }
    pub fn match_inchikey(&self) -> Option<&str> {
        self.gp.inchikey.as_deref().or(self.f.mzlib.inchikey.as_deref())
    }
    pub fn match_smiles(&self) -> Option<&str> {
        self.f.gnps.smiles.as_deref().or(self.f.mzlib.smiles.as_deref())
    }
    pub fn ref_usi(&self) -> Option<&str> {
        self.gp.library_usi.as_deref().or(self.mz_usi.as_deref())
    }
}

fn areas_by_feature(
    c: &Campaign,
    src: &Sources,
    sites: &[String],
) -> Result<HashMap<(String, i64), Vec<f64>>> {
    let mut out = HashMap::new();
    for pol in POLARITIES {
        let Some(t) = src.pol(pol).map(|p| &p.table) else {
            continue;
        };
        let Some(kid) = t.col("id") else { continue };
        let mut per_site = Vec::new();
        for s in sites {
            per_site.push(if has_site(c, t, s, pol) {
                site_area(c, t, s, pol, None, Agg::Sum)?
            } else {
                vec![0.0; t.len()]
            });
        }
        for r in 0..t.len() {
            if let Some(id) = t.int(r, kid) {
                out.entry((pol.to_string(), id))
                    .or_insert_with(|| per_site.iter().map(|a| a[r]).collect());
            }
        }
    }
    Ok(out)
}

/// `build_feature_frame`: every feature of the matrix with its identity, the provenance of the GNPS2 hit, the areas and the RT-logP flag.
pub fn feature_rows<'a>(c: &Campaign, io: &Io, l: &'a Loaded) -> Result<Vec<FRow<'a>>> {
    let feats = l.features();
    let ident: HashMap<(&str, i64), String> = curated(feats, &[1, 2, 3], &l.cur)
        .into_iter()
        .map(|d| ((d.feature.pol.as_str(), d.feature.id), d.identity))
        .collect();
    let prov = gnps_provenance(c, io)?;
    let areas = areas_by_feature(c, &l.src, &l.sites)?;
    let rtfit: HashMap<(String, i64), _> = build_batch_fit(&csi_level3(feats))
        .into_iter()
        .map(|f| ((f.pol.clone(), f.id), f))
        .collect();
    let remapped = remap_sources(&l.src);
    let hits: HashMap<(&str, i64), &crate::attributio::readers::mzmine::MzmineHit> = {
        let mut m = HashMap::new();
        for h in &remapped.mzmine {
            m.entry((h.pol.as_str(), h.id)).or_insert(h);
        }
        m
    };
    Ok(feats
        .iter()
        .map(|f| {
            let key = (f.pol.clone(), f.id);
            let area = areas
                .get(&key)
                .cloned()
                .unwrap_or_else(|| vec![0.0; l.sites.len()]);
            let fit = rtfit.get(&key);
            let hit = hits.get(&(f.pol.as_str(), f.id));
            FRow {
                f,
                identity: ident.get(&(f.pol.as_str(), f.id)).cloned(),
                gp: prov.get(&key).cloned().unwrap_or_default(),
                mz_method: hit.map(|h| h.method.clone()).filter(|_| f.mzlib.name.is_some()),
                mz_usi: hit
                    .and_then(|h| h.usi.clone())
                    .filter(|_| f.mzlib.name.is_some()),
                total_area: area.iter().sum(),
                area,
                rt_z: fit.and_then(|x| x.z),
                rt_note: fit.map(|x| x.note.clone()),
            }
        })
        .collect())
}
