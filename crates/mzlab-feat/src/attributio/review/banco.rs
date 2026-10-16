//! The Banco 2b of a campaign folder: everything the chemist needs to review ONE feature, computed on request (one feature at a time, nothing is kept). It
//! reads the folder through `Io` like the table of levels: the cascade for the level of the feature, the MS2 block of the FBMN MGF, the hit tables of the
//! pipeline tools, the spectral libraries found in the folder, and gives back the proposal of `review_feature` with its charts, the six-box checklist (each box
//! with the proof the program calculated, never ticked), the alternatives with the same formula, the RT-logP check and the decisions already written. It proposes,
//! the chemist decides.

use super::alternatives::{same_formula, Alt, AltSources};
use super::feature::{review_feature, Env, Inputs};
use super::libraries::{find_library_files, LibInput, LibraryIndex};
use super::matching::Ms2;
use super::proposal::{Proposal, J};
use super::propose::{read_mgf_blocks, MgfBlock};
use super::raw::NoRaw;
use super::rtlogp::{calibrate, check, organic_fraction, CalPoint, LogPValues, MIN_N};
use super::rules::{page_level, Current};
use super::settings::ReviewSettings;
use super::sources::{csi_top5, gnps_hits, sirius_hits, SourceTexts, StructureCandidate};
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
    ("alternatives_excluded", "Alternatives with the same formula excluded"),
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
    roots.extend(s.extra_libraries.iter().filter(|p| !p.starts_with('/')).cloned());
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
                (if r.is_empty() { p.clone() } else { r.clone() }, head.unwrap_or_default())
            })
            .collect();
        for (rel, format) in find_library_files(&listing) {
            let path = files.iter().find(|(r, p)| *r == rel || *p == rel).map(|f| f.1.clone());
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

fn num_or_null(v: Option<f64>) -> J {
    J::on(v)
}

/// The retention-time versus logP check of the feature, calibrated on the Level 1 and robust 2a features of its polarity that the sources give a logP for.
fn rt_logp(b: &Bench, f: &Feature) -> Option<(J, Option<bool>)> {
    let s = &b.settings;
    let grad = &s.lc_gradient;
    let floor = if s.rt_logp_floor > 0.0 {
        s.rt_logp_floor
    } else if grad.is_empty() {
        1.0
    } else {
        3.0
    };
    let y_of = |rt: f64| {
        if grad.is_empty() {
            rt
        } else {
            organic_fraction(rt, grad, s.lc_dwell_min)
        }
    };
    let values = |g: &Feature| LogPValues {
        xlogp3: g.csi.xlogp.filter(|v| v.is_finite()),
        ..Default::default()
    };
    let pts: Vec<CalPoint> = b
        .features
        .iter()
        .filter(|g| g.pol == f.pol && (g.level == 1 || (g.level == 2 && g.level_sub == "a")))
        .filter_map(|g| {
            Some(CalPoint {
                logp: values(g),
                y: Some(y_of(g.rt?)),
            })
        })
        .collect();
    let model = calibrate(&pts, s.rt_logp_k, floor, MIN_N);
    let y = f.rt.map(y_of);
    let c = check(&model, &values(f), y, s.rt_logp_k, floor);
    let j = J::Obj(vec![
        ("n_calibrants", J::Num(model.n as f64)),
        ("y", num_or_null(y)),
        (
            "y_label",
            J::Str(if grad.is_empty() { "RT (min)" } else { "organic fraction at elution (%B)" }.into()),
        ),
        ("logp", num_or_null(c.logp)),
        ("pred", num_or_null(c.pred)),
        ("resid", num_or_null(c.resid)),
        ("window", num_or_null(model.window)),
        ("ok", c.ok.map_or(J::Null, J::Bool)),
        ("note", J::Str(model.note.clone())),
    ]);
    Some((j, c.ok))
}

/// The proof the program calculated for each box of the checklist (`None`: not available); the chemist ticks the boxes, the program never does.
fn proofs(p: &Proposal, alts: &[Alt], rt: Option<bool>) -> Vec<Option<String>> {
    let e = &p.evidence;
    let view = e.view.as_ref();
    let frags: Vec<_> = view
        .map(|v| v.peaks.iter().filter_map(|k| k.frag.as_ref()).collect())
        .unwrap_or_default();
    let losses: Vec<&str> = frags
        .iter()
        .map(|f| f.loss.as_str())
        .filter(|l| !l.is_empty())
        .collect();
    let iso = e.raw.as_ref().and_then(|r| r.ms1.as_ref()).and_then(|m| m.iso_ok);
    let pur = e.purity.as_ref().and_then(|u| u.purity.map(|v| (v, u.badge.clone())));
    vec![
        (e.n_peaks > 0).then(|| {
            format!(
                "{} of {} peaks explained ({:.0}% of the intensity)",
                e.n_explained,
                e.n_peaks,
                e.frac_explained * 100.0
            )
        }),
        (!losses.is_empty()).then(|| format!("losses: {}", losses.join(", "))),
        iso.map(|ok| format!("isotope pattern {}", if ok { "consistent" } else { "not consistent" })),
        pur.map(|(v, b)| format!("isolation purity {v:.2} ({b})")),
        rt.map(|ok| format!("RT versus logP: {}", if ok { "within the window" } else { "outside the window" })),
        Some(format!("{} alternative(s) with the same formula listed", alts.len())),
    ]
}

fn alt_json(a: &Alt, best_ik1: Option<&str>) -> J {
    J::Obj(vec![
        ("name", J::s(&a.name)),
        ("source", J::s(&a.source)),
        ("entry", J::os(&a.entry)),
        ("ik1", J::os(&a.ik1)),
        ("smiles", J::os(&a.smiles)),
        ("score", J::on(a.score)),
        ("matched", a.matched.map_or(J::Null, |m| J::Num(m as f64))),
        ("logp", J::on(a.logp)),
        ("is_best", J::Bool(a.ik1.is_some() && a.ik1.as_deref() == best_ik1)),
    ])
}

/// The last decision row of the feature, as `{column: value}`, and how many rows the feature has.
fn decision_json(d: &DecisionTable, f: &Feature) -> (J, usize) {
    let mut last = None;
    let mut n = 0;
    for r in 0..d.len() {
        let same_pol = d.get(r, "pol").trim().eq_ignore_ascii_case(&f.pol);
        let same_id = d.get(r, "feature_id").trim().parse::<f64>().ok() == Some(f.id as f64);
        if same_pol && same_id {
            last = Some(r);
            n += 1;
        }
    }
    let j = last.map_or(J::Null, |r| {
        J::Arr(
            d.columns
                .iter()
                .map(|c| J::Arr(vec![J::s(c), J::s(d.get(r, c))]))
                .collect(),
        )
    });
    (j, n)
}

/// The review of one feature as JSON (see the module).
pub fn review_json(io: &Io, root: &str, pol: &str, id: i64) -> Result<String> {
    let b = Bench::load(io, root)?;
    let f = b
        .feature(pol, id)
        .ok_or_else(|| CampaignError::not_found(format!("feature {pol} {id} is not in the table")))?;
    let texts = source_texts(&b.campaign, io);
    let lib = folder_libraries(io, &b.settings);
    let mgf: Option<MgfBlock> = feature_mgf(&b.campaign, io, pol)
        .and_then(|p| io.text(&p))
        .and_then(|t| read_mgf_blocks(&t, Some(&[id])).remove(&id));
    let (sirius, gnps) = (sirius_hits(&texts), gnps_hits(&texts));
    let inputs = Inputs {
        mgf: mgf.as_ref(),
        sirius: sirius.iter().filter(|h| h.pol == pol && h.id == id).collect(),
        gnps: gnps.iter().filter(|h| h.pol == pol && h.id == id).collect(),
    };
    let st = GraphStructures;
    let env = Env {
        s: &b.settings,
        lib: &lib,
        st: &st,
        raw: &NoRaw,
    };
    let p = review_feature(&env, f, &inputs);
    let csi: Vec<StructureCandidate> = csi_top5(&texts)
        .into_iter()
        .find(|(k, _)| k.0 == pol && k.1 == id)
        .map(|x| x.1)
        .unwrap_or_default();
    let exp = p.evidence.view.as_ref().map(|v| Ms2 {
        mz: v.peaks.iter().map(|k| k.mz).collect(),
        it: v.peaks.iter().map(|k| k.rel).collect(),
        prec: mgf.as_ref().map_or(f.mz.unwrap_or(0.0), |m| m.prec),
    });
    let alts = match p.best.formula.as_deref() {
        Some(formula) => same_formula(
            formula,
            &lib,
            &AltSources { csi: &csi, exp: exp.as_ref(), tol: b.settings.tol_da, st: &st },
        ),
        None => Vec::new(),
    };
    let rt = rt_logp(&b, f);
    let marks = proofs(&p, &alts, rt.as_ref().and_then(|r| r.1));
    let checklist = J::Arr(
        CHECKLIST
            .iter()
            .zip(marks)
            .map(|((key, label), proof)| {
                J::Obj(vec![("key", J::s(key)), ("label", J::s(label)), ("proof", proof.map_or(J::Null, J::Str))])
            })
            .collect(),
    );
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
        (
            "current",
            J::Obj(vec![
                ("level", J::Str(page_level(cur.level, cur.level_sub, cur.decision_2b))),
                ("sub", J::s(&f.level_sub)),
                ("basis", J::s(&f.level_basis)),
                ("name", cur.name.map_or(J::Null, J::s)),
                ("identity", f.identity().map_or(J::Null, J::s)),
                ("group", cur.group.map_or(J::Null, J::s)),
            ]),
        ),
        ("proposal", J::Raw(p.to_json())),
        ("checklist", checklist),
        (
            "alternatives",
            J::Arr(alts.iter().map(|a| alt_json(a, p.best.ik1.as_deref())).collect()),
        ),
        ("rtlogp", rt.map_or(J::Null, |r| r.0)),
        ("decision", last),
        ("decision_rows", J::Num(n_rows as f64)),
        ("decisions_path", J::Str(b.decisions_path().0)),
        (
            "libraries",
            J::Arr(
                lib.summary()
                    .into_iter()
                    .map(|(l, n)| J::Arr(vec![J::Str(l), J::Num(n as f64)]))
                    .collect(),
            ),
        ),
        ("has_ms2", J::Bool(mgf.is_some())),
        ("warnings", J::strs(&b.warnings)),
    ]);
    let mut s = String::new();
    out.write(&mut s);
    Ok(s)
}
