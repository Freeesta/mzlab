// Ported from Attributio diagnostics/publication_table.py.
//! The README sheet of the publication table: title, description, the dictionary of the columns, the criteria of the levels read from the configuration of the
//! campaign (never retyped), the definitions of the flags and the provenance stamp.

use super::cells::Sheet;
use super::publication_text::*;
use super::pyfmt::py_float;
use crate::attributio::annotate::{standards_settings, AnnotationSettings};
use crate::attributio::campaign::Campaign;
use crate::attributio::error::Result;
use crate::attributio::frame::Cell;
use mzlab_core::json::fmt_g;

/// The counts of the paper, in the order of the reference tool.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Counts {
    pub rows: i64,
    pub anthropogenic: i64,
    pub level_1: i64,
    pub level_2a: i64,
    pub level_2b: i64,
    pub level_3: i64,
    pub rows_flag_mzlib_only: i64,
    pub features_mzlib_flag_in_rows: i64,
    pub features_mzlib_flag: i64,
    pub anthropogenic_all_sites_detected: i64,
    pub features_mzlib_only: i64,
    pub anthropogenic_all_sites: i64,
    pub excluded: i64,
}

impl Counts {
    pub fn pairs(&self) -> [(&'static str, i64); 13] {
        [
            ("rows", self.rows),
            ("anthropogenic", self.anthropogenic),
            ("level_1", self.level_1),
            ("level_2a", self.level_2a),
            ("level_2b", self.level_2b),
            ("level_3", self.level_3),
            ("rows_flag_mzlib_only", self.rows_flag_mzlib_only),
            (
                "features_mzlib_flag_in_rows",
                self.features_mzlib_flag_in_rows,
            ),
            ("features_mzlib_flag", self.features_mzlib_flag),
            (
                "anthropogenic_all_sites_detected",
                self.anthropogenic_all_sites_detected,
            ),
            ("features_mzlib_only", self.features_mzlib_only),
            ("anthropogenic_all_sites", self.anthropogenic_all_sites),
            ("excluded", self.excluded),
        ]
    }
}

/// What the stamp of the table says about where it came from (the caller knows it; empty values are written as empty cells).
#[derive(Clone, Debug, Default)]
pub struct Provenance {
    pub script: String,
    pub commit: String,
    pub core: String,
    pub curation: String,
    pub generated: String,
}

/// The inputs of the README.
pub struct ReadmeInput<'a> {
    pub campaign: &'a Campaign,
    pub counts: &'a Counts,
    pub sites: &'a [String],
    pub short: &'a [String],
    pub sorbents: &'a [String],
    pub title: &'a str,
    pub description: &'a str,
    pub prov: &'a Provenance,
}

fn fold_text(f: Option<f64>) -> String {
    fmt_g(f.unwrap_or(5.0))
}

/// The names of the columns of `Identities` for the sites and sorbents of the matrix.
pub fn column_names(
    sites: &[String],
    short: &[String],
    sorbents: &[String],
    blank_fold: Option<f64>,
) -> Vec<String> {
    let _ = sites;
    let mut v: Vec<String> = DICT_HEAD.iter().map(|(k, _)| (*k).to_string()).collect();
    v.extend(short.iter().map(|s| format!("Detected {s}")));
    v.extend(short.iter().map(|s| format!("Max area {s}")));
    v.extend(sorbents.iter().map(|s| format!("Detected on {s}")));
    v.extend(DICT_MID.iter().map(|(k, _)| (*k).to_string()));
    v.push(format!(
        "Robust above {}x blank at all sites",
        fold_text(blank_fold)
    ));
    v.push("Detected in control".to_string());
    v.extend(DICT_TAIL_CLASSES.iter().map(|s| (*s).to_string()));
    v.extend(DICT_TAIL.iter().map(|(k, _)| (*k).to_string()));
    v
}

const DICT_TAIL_CLASSES: [&str; 2] = ["ClassyFire superclass", "NPClassifier pathway"];

/// The presence rule of every count of the campaign (`[presence]` of the campaign file).
pub fn presence_rule(fold: Option<f64>) -> String {
    let with_fold = fold.map_or(String::new(), |f| {
        format!(
            " with an area ≥ {} x its abundance in the procedural blanks (or absent from them)",
            fmt_g(f)
        )
    });
    format!(
        "DETECTED by MZmine{with_fold} (either sorbent; gap-filled values are never detections)"
    )
}

fn column_dictionary(x: &ReadmeInput, canopus_min: f64) -> Vec<(String, String)> {
    let fold = fold_text(x.campaign.presence_blank_fold);
    let pres = presence_rule(x.campaign.presence_blank_fold);
    let mut d: Vec<(String, String)> = DICT_HEAD
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    for s in x.short {
        d.push((format!("Detected {s}"), format!("yes if at least one Level 1-3 feature of the compound is present in the sample {s}: {pres}.")));
    }
    for s in x.short {
        d.push((format!("Max area {s}"), format!("Largest peak area among the features of the compound present in {s} (larger of the two sorbents). Arbitrary units: a signal, not a concentration; not comparable between compounds.")));
    }
    for lab in x.sorbents {
        d.push((format!("Detected on {lab}"), format!("yes if at least one feature of the compound is present in a {lab} extract at any site (same rule).")));
    }
    d.extend(
        DICT_MID
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string())),
    );
    d.push((format!("Robust above {fold}x blank at all sites"), format!("Only for compounds DETECTED at all sites when the blanks are ignored: yes if at every site at least one DETECTED feature has an area ≥ {fold} x its procedural-blank abundance (or is absent from the blanks); no = within {fold}x the blank at ≥ 1 site.")));
    d.push(("Detected in control".into(), format!("yes if at least one feature is DETECTED in the control sample (none supplied) with an area ≥ {fold} x its procedural-blank abundance (same rule as the sites).")));
    let conf = format!("{canopus_min:.2}");
    d.push(("ClassyFire superclass".into(), format!("CANOPUS (SIRIUS) ClassyFire superclass predicted for the representative feature, shown only with probability ≥ {conf}.")));
    d.push(("NPClassifier pathway".into(), format!("CANOPUS (SIRIUS) NPClassifier pathway predicted for the representative feature, shown only with probability ≥ {conf}.")));
    d.extend(
        DICT_TAIL
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string())),
    );
    d
}

fn level_criteria(x: &ReadmeInput) -> Result<Vec<(String, String)>> {
    let c = x.campaign;
    let s = AnnotationSettings::for_campaign(c)?;
    let peaks = if s.spec_peaks_min.fract() == 0.0 {
        format!("{:.0}", s.spec_peaks_min)
    } else {
        py_float(s.spec_peaks_min)
    };
    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(st) = standards_settings(c) {
        out.push(("Level 1".into(), format!("In-house reference standard measured on the same instrument and LC method: precursor m/z within {} ppm, RT within {} min of the standard, and MS/MS cosine ≥ {:.2} on ≥ {} matched fragments against the standard's own spectrum of the same ion (MS/MS tolerance {} Da). Borderline MS/MS cases are decided by the analyst on the mirror plot (flagged).", fmt_g(st.mass_tol_ppm), fmt_g(st.rt_tol_min), st.ms2_min_cosine, st.ms2_min_matched, fmt_g(st.ms2_tol_da))));
    }
    out.push(("Level 2a".into(), format!("Spectral-library match: cosine ≥ {:.2} and ≥ {peaks} matched fragment ions, for every library (SIRIUS spectral library, GNPS2, MZmine), with the library precursor within {} ppm (GNPS2) or {} ppm (MZmine) of the feature (analogue hits excluded). A match below these gates is not evidence: it gives neither a level nor a name. Level 2a resting on the MZmine library alone is flagged for the reader.", s.spec_sim_min, fmt_g(s.gnps_mass_tol_ppm), fmt_g(s.mzlib_mass_tol_ppm))));
    out.push(("One compound, one retention time".into(), format!("The features of an identity (same curated name, both polarities, same LC) are grouped by retention time: consecutive features more than {:.2} min apart are different peaks. The group with the best evidence (level, number of valid sources, sites, area) keeps the name; the others are reported as '<name> isomer, RT x.x min'. When two groups rest on a library match of the same compound the spectrum does not tell the isomers apart and all of them are Level 3; when the reference standard (Level 1) fixes the retention time, a library match at another retention time is Level 3 (another isomer).", s.rt_group_tol_min)));
    out.push(("Level 3".into(), format!("Tentative structure: CSI:FingerID candidate with COSMIC confidence ≥ {:.2} (about 10% false discovery rate; Hoffmann et al., Nat. Biotechnol. 2022, 40, 411); or a Level 2a library match downgraded because the valid library matches of the same feature name structures with different connectivity (different InChIKey first blocks), or because the analyst judged it insufficient (both flagged).", s.cosmic_min)));
    out.push(("Not counted".into(), "A reference standard matching precursor m/z and RT whose MS/MS could not confirm it is a suspect, not an identification: it is not in this table.".into()));
    if x.counts.level_2b > 0 {
        let count = if c.count_2b_as_2 {
            "counted with Level 2 in the totals"
        } else {
            "counted with Level 3 in the totals"
        };
        let at = out.len() - 3;
        out.insert(at, ("Level 2b".into(), format!("Structure supported by diagnostic evidence: a reference spectrum of the same structure that matches on diagnostic fragments (also below the Level 2a gates), or a fragment specific of the structure, judged by the analyst on the MS/MS spectrum and recorded per feature with the evidence (library entry, cosine, matched ions); never assigned automatically. An analogue match (reference precursor shifted by a mass difference) is never Level 2a; it supports 2b only with the diagnostic fragments confirmed on the raw spectrum. Reported apart from Level 2a and {count}.")));
    }
    out.push((
        "Controls".into(),
        "No procedural-blank / control-sample audit was supplied: the control columns are empty."
            .into(),
    ));
    Ok(out)
}

fn content_line(x: &ReadmeInput) -> String {
    let c = x.counts;
    let two_b = if c.level_2b > 0 {
        format!("Level 2b: {}, ", c.level_2b)
    } else {
        String::new()
    };
    format!(
        "{} compounds (Level 1: {}, Level 2a: {}, {two_b}Level 3: {}; {} anthropogenic contaminants) detected in {} samples ({}); {} excluded identities in sheet Excluded.",
        c.rows, c.level_1, c.level_2a, c.level_3, c.anthropogenic, x.sites.len(), x.short.join(", "), c.excluded
    )
}

/// The README sheet.
pub fn readme(x: &ReadmeInput) -> Result<Sheet> {
    let c = x.campaign;
    let canopus = AnnotationSettings::for_campaign(c)?.canopus_min;
    let blank = ("".to_string(), "".to_string());
    let mut rows: Vec<(String, String)> = vec![
        ("Title".into(), x.title.into()),
        ("Description".into(), x.description.into()),
        ("Content".into(), content_line(x)),
        ("Dataset".into(), format!("{}. {}, {}, ESI+ and ESI-; two solid-phase extraction sorbents per sample ({}).", c.name, c.instrument, c.fragmentation, x.sorbents.join(", "))),
        ("Detection".into(), format!("A compound is present at a sample when one of its features is {}. n = 1 per site: no statistical test.", presence_rule(c.presence_blank_fold))),
        blank.clone(),
        ("COLUMNS".into(), "".into()),
    ];
    rows.extend(column_dictionary(x, canopus));
    rows.push(blank.clone());
    rows.push(("LEVEL CRITERIA".into(), "".into()));
    rows.extend(level_criteria(x)?);
    rows.push(blank.clone());
    rows.push(("FLAGS".into(), "".into()));
    let z = fmt_g(crate::attributio::rt_logp::Z_FLAG);
    for (k, v) in FLAG_DEFS {
        let key = if k == "@RTLOGP@" {
            format!("{FLAG_RT_LOGP} (|z| > {z})")
        } else {
            k.to_string()
        };
        rows.push((key, v.to_string()));
    }
    if x.counts.level_2b > 0 {
        rows.push((FLAG_2B.into(), "the compound rests on Level 2b features only: the analyst's decision on diagnostic evidence (criteria above).".into()));
    }
    rows.push(blank);
    rows.push(("PROVENANCE".into(), "".into()));
    let p = x.prov;
    rows.push(("Generated by".into(), p.script.clone()));
    rows.push(("Project commit".into(), p.commit.clone()));
    rows.push(("attributio".into(), p.core.clone()));
    rows.push(("Curation fingerprint".into(), p.curation.clone()));
    rows.push(("Generated (UTC)".into(), p.generated.clone()));
    let mut sheet = Sheet::new("README", &["Item", "Description"]);
    sheet.rows = rows
        .into_iter()
        .map(|(a, b)| vec![Cell::Str(a), Cell::Str(b)])
        .collect();
    Ok(sheet)
}
