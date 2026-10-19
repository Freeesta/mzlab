//! The Banco 2b of a campaign folder: everything the chemist needs to review ONE feature, computed on request (one feature at a time, nothing is kept). It
//! reads the folder through `Io` like the table of levels: the cascade for the level of the feature, the MS2 block of the FBMN MGF, the hit tables of the
//! pipeline tools, the spectral libraries found in the folder, and gives back the proposal of `review_feature` with its charts, the six-box checklist (each box
//! with the proof the program calculated, never ticked), the alternatives with the same formula, the RT-logP check and the decisions already written. It proposes,
//! the chemist decides.

use super::banco_json::*;
use super::feature::{review_feature, Env, Inputs};
use super::libraries::{find_library_files, LibInput, LibraryIndex};
use super::proposal::{Proposal, J};
use super::propose::{read_mgf_blocks, MgfBlock};
use super::raw::NoRaw;
use super::rules::Current;
use super::settings::ReviewSettings;
use super::sources::{gnps_hits, sirius_hits, GnpsHit, SiriusHit, SourceTexts};
use super::structures::GraphStructures;
use crate::attributio::annotate::{gnps_comparer, load_annotations, Feature};
use crate::attributio::campaign::{Campaign, Env as CEnv, POLARITIES};
use crate::attributio::curation::TableCuration;
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::io::{join, Io};
use crate::attributio::readers::feature_decisions::{load_feature_decisions, DecisionTable};
use crate::attributio::readers::ms2::feature_mgf;
use crate::attributio::readers::sirius::sirius_output_dir;
use crate::attributio::sources::load_sources;

/// The folder of the decisions when `campaign.toml` does not declare one.
pub const DEFAULT_DECISIONS: &str = "03 Stats/scripts/shared/feature_decisions.csv";

/// The six boxes of the checklist: key (what `evidence_checklist` holds) and label.
pub const CHECKLIST: [(&str, &str); 6] = [
    ("diagnostic_fragment", "Diagnostic fragment explained"),
    ("neutral_loss", "Neutral loss consistent"),
    ("isotopes", "Isotopes consistent"),
    ("pure_precursor", "Precursor pure"),
    ("rt_plausible", "RT plausible"),
    (
        "alternatives_excluded",
        "Alternatives with the same formula excluded",
    ),
];

/// A campaign loaded for review: the folder, the cascade and the settings.
pub struct Bench {
    pub campaign: Campaign,
    pub features: Vec<Feature>,
    pub warnings: Vec<String>,
    pub settings: ReviewSettings,
}

impl Bench {
    pub fn load(io: &Io, root: &str) -> Result<Bench> {
        let text = io
            .text("campaign.toml")
            .ok_or_else(|| CampaignError::not_found("campaign.toml is not in the folder"))?;
        let campaign = Campaign::from_toml(&text, "campaign.toml", root, CEnv::default())?;
        let src = load_sources(&campaign, io)?;
        let cur = TableCuration::load(&campaign, io)?;
        let a = load_annotations(&campaign, &src, None, &cur, &gnps_comparer)?;
        let settings = ReviewSettings::from_campaign_toml("campaign.toml", &text)
            .map_err(CampaignError::value)?;
        Ok(Bench {
            campaign,
            features: a.features,
            warnings: a.warnings,
            settings,
        })
    }

    pub fn feature(&self, pol: &str, id: i64) -> Option<&Feature> {
        self.features.iter().find(|f| f.pol == pol && f.id == id)
    }

    /// The decisions file declared by the campaign (or the default place) and whether it was declared.
    pub fn decisions_path(&self) -> (String, bool) {
        match &self.campaign.feature_decisions_csv {
            Some(p) => (p.clone(), true),
            None => (DEFAULT_DECISIONS.to_string(), false),
        }
    }

    pub fn decisions(&self, io: &Io) -> Result<DecisionTable> {
        let (path, _) = self.decisions_path();
        load_feature_decisions(io.text(&path).as_deref(), &path)
    }
}

/// The text of the tool outputs the review reads (only the tables it uses).
fn source_texts(c: &Campaign, io: &Io) -> SourceTexts {
    let mut t = SourceTexts::default();
    for pol in POLARITIES {
        if let Some(d) = sirius_output_dir(c, io, pol) {
            for table in [
                "spectral_matches_top-5",
                "spectral_matches_analog_top-5",
                "formula_identifications_top-5",
                "structure_identifications_top-5",
            ] {
                if let Some(s) = io.text(&join(&d, &format!("{table}.tsv"))) {
                    t.sirius.push((pol.to_string(), table.to_string(), s));
                }
            }
        }
        let g = c.gnps_output_dir(pol);
        if let Some(s) = io.text(&join(&g, "nf_output/library/merged_results_with_gnps.tsv")) {
            t.gnps_library.push((pol.to_string(), s));
        }
        if let Some(s) = io.text(&join(&g, "nf_output/networking/filtered_pairs.tsv")) {
            t.gnps_pairs.push((pol.to_string(), s));
        }
    }
    t
}

/// Every file under `dir` (at most four levels down) as `(path relative to dir, path in the folder)`.
fn walk(io: &Io, dir: &str, rel: &str, depth: usize, out: &mut Vec<(String, String)>) {
    for e in io.list(dir) {
        let (r, p) = (join(rel, &e.name), join(dir, &e.name));
        if e.is_dir && depth < 4 {
            walk(io, &p, &r, depth + 1, out);
        } else if !e.is_dir {
            out.push((r, p));
        }
    }
}

/// The spectral libraries inside the folder: `library/`, `libraries/` and the paths of `[review] extra_libraries` that lie in the folder.
pub fn folder_libraries(io: &Io, s: &ReviewSettings) -> LibraryIndex {
    let mut roots: Vec<String> = vec!["library".into(), "libraries".into()];
    roots.extend(
        s.extra_libraries
            .iter()
            .filter(|p| !p.starts_with('/'))
            .cloned(),
    );
    let mut inputs = Vec::new();
    for root in roots {
        let mut files = Vec::new();
        if io.is_dir(&root) {
            walk(io, &root, "", 0, &mut files);
        } else if io.exists(&root) {
            files.push((String::new(), root.clone()));
        }
        let listing: Vec<(String, Vec<u8>)> = files
            .iter()
            .map(|(r, p)| {
                let head = io.read(p).map(|b| b[..b.len().min(4096)].to_vec());
                (
                    if r.is_empty() { p.clone() } else { r.clone() },
                    head.unwrap_or_default(),
                )
            })
            .collect();
        for (rel, format) in find_library_files(&listing) {
            let path = files
                .iter()
                .find(|(r, p)| *r == rel || *p == rel)
                .map(|f| f.1.clone());
            if let Some(bytes) = path.and_then(|p| io.read(&p)) {
                inputs.push(LibInput {
                    label: join(&root, &rel).trim_end_matches('/').to_string(),
                    format,
                    bytes,
                });
            }
        }
    }
    LibraryIndex::build(inputs)
}

/// A campaign loaded for review with what every feature needs: the texts of the tool outputs, the libraries and the hits.
pub(super) struct Run {
    pub(super) b: Bench,
    texts: SourceTexts,
    lib: LibraryIndex,
    sirius: Vec<SiriusHit>,
    gnps: Vec<GnpsHit>,
}

impl Run {
    pub(super) fn load(io: &Io, root: &str) -> Result<Run> {
        let b = Bench::load(io, root)?;
        let texts = source_texts(&b.campaign, io);
        let lib = folder_libraries(io, &b.settings);
        let (sirius, gnps) = (sirius_hits(&texts), gnps_hits(&texts));
        Ok(Run {
            b,
            texts,
            lib,
            sirius,
            gnps,
        })
    }

    /// The MS2 block of the feature in the FBMN MGF.
    fn mgf(&self, io: &Io, f: &Feature) -> Option<MgfBlock> {
        feature_mgf(&self.b.campaign, io, &f.pol)
            .and_then(|p| io.text(&p))
            .and_then(|t| read_mgf_blocks(&t, Some(&[f.id])).remove(&f.id))
    }

    /// The proposal of the review for one feature, with its MS2 block.
    pub(super) fn proposal(&self, io: &Io, f: &Feature) -> (Proposal, Option<MgfBlock>) {
        let mgf = self.mgf(io, f);
        let inputs = Inputs {
            mgf: mgf.as_ref(),
            sirius: self
                .sirius
                .iter()
                .filter(|h| h.pol == f.pol && h.id == f.id)
                .collect(),
            gnps: self
                .gnps
                .iter()
                .filter(|h| h.pol == f.pol && h.id == f.id)
                .collect(),
        };
        let st = GraphStructures;
        let env = Env {
            s: &self.b.settings,
            lib: &self.lib,
            st: &st,
            raw: &NoRaw,
        };
        (review_feature(&env, f, &inputs), mgf)
    }
}

/// The review of one feature as JSON (see the module).
pub fn review_json(io: &Io, root: &str, pol: &str, id: i64) -> Result<String> {
    let run = Run::load(io, root)?;
    let (b, texts, lib) = (&run.b, &run.texts, &run.lib);
    let f = b.feature(pol, id).ok_or_else(|| {
        CampaignError::not_found(format!("feature {pol} {id} is not in the table"))
    })?;
    let (p, mgf) = run.proposal(io, f);
    let alts = same_formula_alts(b, f, &p, texts, lib, mgf.as_ref());
    let rt = rt_logp(b, f);
    let marks = proofs(&p, &alts, rt.as_ref().and_then(|r| r.1));
    let checklist = checklist_json(marks);
    let table = b.decisions(io)?;
    let (last, n_rows) = decision_json(&table, f);
    let cur = Current {
        level: f.level,
        level_sub: &f.level_sub,
        decision_2b: f.decision.decision_2b,
        name: f.name(),
        group: f.decision.group_override.as_deref(),
    };
    let out = J::Obj(vec![
        ("pol", J::s(pol)),
        ("id", J::Num(id as f64)),
        ("mz", J::on(f.mz)),
        ("rt", J::on(f.rt)),
        ("current", current_json(f, &cur)),
        ("proposal", J::Raw(p.to_json())),
        ("checklist", checklist),
        (
            "alternatives",
            J::Arr(
                alts.iter()
                    .map(|a| alt_json(a, p.best.ik1.as_deref()))
                    .collect(),
            ),
        ),
        ("classes", classes_json(b, f, mgf.as_ref())),
        ("rtlogp", rt.map_or(J::Null, |r| r.0)),
        ("decision", last),
        ("decision_rows", J::Num(n_rows as f64)),
        ("decisions_path", J::Str(b.decisions_path().0)),
        ("libraries", libraries_json(lib)),
        ("has_ms2", J::Bool(mgf.is_some())),
        ("warnings", J::strs(&b.warnings)),
    ]);
    let mut s = String::new();
    out.write(&mut s);
    Ok(s)
}
