// Ported from Attributio diagnostics/publication_table.py.
//! The publishable inventory of the chemical identities (Levels 1-3) of a campaign: one row per identity with the exact count of the paper, its structure,
//! the representative ion, the evidence that gave the level, the flags, the detection and the maximum area per site, and the sheet of the identities left
//! out. The hazard columns are the record of a compound that no registry knows (the registry snapshot and the ecotoxicological data are not in the
//! engine) and the controls (procedural blanks, control sample) are not supplied: their columns stay empty, as the reference tool does without them.

use super::cells::Sheet;
use super::ctx::{matrix_and_sites, FRow, Loaded};
use super::publication_ids::{canonical_identities, feature_frame, Dd, PFeat};
use super::publication_readme::{self, Counts};
use super::publication_text::*;
use super::schymanski::site_short;
use crate::attributio::campaign::Campaign;
use crate::attributio::curation::noncec_category_label;
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::frame::Cell;
use crate::attributio::identity::IdentityRow;
use crate::attributio::io::Io;

impl Build<'_> {
    fn site_cells(&self, it: &Dd, sub: &[&PFeat]) -> Vec<Cell> {
        let mut v: Vec<Cell> = it.in_site.iter().map(|b| yes_no(*b)).collect();
        for (k, present) in it.in_site.iter().enumerate() {
            let a = if *present {
                sub.iter()
                    .map(|p| p.max_area[k])
                    .filter(|x| !x.is_nan())
                    .fold(f64::NAN, f64::max)
            } else {
                0.0
            };
            v.push(if a > 0.0 {
                Cell::Int(a.round_ties_even() as i64)
            } else {
                Cell::Null
            });
        }
        for s in &self.c.sorbents {
            v.push(yes_no(sub.iter().any(|p| {
                p.f().any_sorbent.iter().any(|(k, on)| k == s && *on)
            })));
        }
        v
    }

    fn identity_row(&self, it: &Dd, fr: &[PFeat]) -> Result<(Vec<Cell>, Meta)> {
        let sub: Vec<&PFeat> = fr.iter().filter(|p| p.nome == it.nome).collect();
        if sub.is_empty() {
            return Err(CampaignError::campaign(format!(
                "'{}': no Level 1-3 feature found (identity table out of sync)",
                it.nome
            )));
        }
        let rep = representative(&sub);
        let lv = it.best_level;
        let lv_sub = if lv == 2 { it.best_sub.as_str() } else { "" };
        let formula = Some(it.formula.as_str()).filter(|f| !f.is_empty());
        let ion = ion_fit(rep, lv, formula);
        let best_feats: Vec<&PFeat> = sub.iter().copied().filter(|p| p.f().level == lv).collect();
        let fl = flags(
            &FlagInput {
                sub: &sub,
                rep,
                best_feats: &best_feats,
                nome: &it.nome,
                lv,
                lv_sub,
                watch: &self.c.rt_logp_watchlist,
            },
            ion.inferred,
        );
        let (ik, smi, basis) = structure(rep);
        let f = rep.f();
        let mut row = head_cells(it, lv, lv_sub);
        row.extend(hazard_cells(&it.nome, lv));
        row.extend([
            text(formula.unwrap_or("")),
            rounded(ion.mono, 4),
            ik,
            smi,
            basis,
            text(if f.pol == "POS" { "ESI+" } else { "ESI-" }),
            text(ion.adduct.clone().unwrap_or_default()),
            rounded(f.mz, 4),
            rounded(f.rt, 2),
            rounded(f.rt_info.rt_ref, 2),
            text(rt_range(&sub)),
            rounded(ion.err, 1),
        ]);
        row.extend(evidence_cells(&best_feats, rep, lv));
        row.extend(std_cells(rep, lv));
        row.push(text(fl.join("; ")));
        row.extend(self.site_cells(it, &sub));
        row.extend([Cell::Null, Cell::Null, text(""), text("")]);
        let cls = |i: usize| text(f.classes[i].clone().unwrap_or_default());
        row.extend([
            cls(3),
            cls(0),
            Cell::Int(sub.len() as i64),
            text(feature_ids(&sub)),
        ]);
        let meta = Meta {
            lv,
            anthro: it.is_anthro,
            sub_b: lv_sub == "b",
            n_mz: count_mzlib_only(&sub),
            core: it.in_site.iter().all(|b| *b),
            flags: fl.join("; "),
            key: it.nome.to_lowercase(),
        };
        Ok((row, meta))
    }
}

use super::publication_ev::*;
use super::publication_flags::*;
struct Build<'a> {
    c: &'a Campaign,
    l: &'a Loaded,
    short: Vec<String>,
    blank_fold: Option<f64>,
}

/// The columns of `Identities`, in order.
pub fn columns(
    sites: &[String],
    short: &[String],
    sorbents: &[String],
    blank_fold: Option<f64>,
) -> Vec<String> {
    publication_readme::column_names(sites, short, sorbents, blank_fold)
}

fn counts_of(metas: &[Meta], excluded: usize, l: &Loaded) -> Counts {
    let n = |f: &dyn Fn(&Meta) -> bool| metas.iter().filter(|m| f(m)).count() as i64;
    let feats = l.features();
    Counts {
        rows: metas.len() as i64,
        anthropogenic: n(&|m| m.anthro),
        level_1: n(&|m| m.lv == 1),
        level_2a: n(&|m| m.lv == 2 && !m.sub_b),
        level_2b: n(&|m| m.lv == 2 && m.sub_b),
        level_3: n(&|m| m.lv == 3),
        rows_flag_mzlib_only: n(&|m| m.flags.contains(FLAG_MZLIB_ONLY)),
        features_mzlib_flag_in_rows: metas.iter().map(|m| m.n_mz as i64).sum(),
        features_mzlib_flag: feats
            .iter()
            .filter(|f| f.level_flag.as_deref() == Some("mzlib-only"))
            .count() as i64,
        anthropogenic_all_sites_detected: 0,
        features_mzlib_only: feats
            .iter()
            .filter(|f| {
                f.level == 2
                    && f.sources.iter().all(|s| *s == "mzMine-library")
                    && !f.sources.is_empty()
            })
            .count() as i64,
        anthropogenic_all_sites: n(&|m| m.anthro && m.core),
        excluded: excluded as i64,
    }
}

// ── the excluded identities ──────────────────────────────────────────────

const EXCLUDED_HEAD: [&str; 8] = [
    "Compound",
    "Name in the source",
    "Group",
    "Reason excluded",
    "Confidence level",
    "Detected at",
    "N features",
    "Feature IDs (polarity:MZmine id)",
];

fn excluded_reason(group: &str) -> &'static str {
    if group == "ambiguous" {
        "not a single defensible compound or origin (library code, generic or composite name, or compound of undetermined origin): curation decision"
    } else {
        "internal standard, reagent or instrumental background"
    }
}

fn excluded_row(x: &Build, e: &IdentityRow, fr: &[PFeat]) -> (String, Vec<Cell>) {
    let sub: Vec<&PFeat> = fr.iter().filter(|p| p.ev.identity == e.identity).collect();
    let analyst = sub.iter().any(|p| {
        p.f()
            .decision
            .group_override
            .as_deref()
            .is_some_and(|g| !g.is_empty())
    });
    let why = sub
        .iter()
        .map(|p| p.f().decision.feature_decision.as_str())
        .find(|m| !m.trim().is_empty())
        .unwrap_or("");
    let compound = sub.first().map_or(e.identity.clone(), |p| p.nome.clone());
    let group = if analyst {
        "Excluded by the analyst".to_string()
    } else {
        noncec_category_label(&e.group).map_or(e.group.clone(), str::to_string)
    };
    let reason = if analyst {
        format!("excluded by the analyst after review of the evidence: {why}")
    } else {
        excluded_reason(&e.group).to_string()
    };
    let detected: Vec<&str> = e
        .in_site
        .iter()
        .zip(&x.short)
        .filter(|((_, on), _)| *on)
        .map(|(_, s)| s.as_str())
        .collect();
    let row = vec![
        text(compound.clone()),
        text(e.identity.clone()),
        text(group),
        text(reason),
        text(LEVEL_LABEL[e.best_level as usize - 1]),
        text(detected.join(", ")),
        Cell::Int(sub.len() as i64),
        text(feature_ids(&sub)),
    ];
    (compound.to_lowercase(), row)
}

fn excluded_sheet(x: &Build, fr: &[PFeat]) -> Sheet {
    let mut keyed: Vec<(String, Vec<Cell>)> =
        x.l.all_identities()
            .iter()
            .filter(|e| e.group == "ambiguous" || e.group == "lab_artifact")
            .map(|e| excluded_row(x, e, fr))
            .collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    let mut s = Sheet::new("Excluded", &EXCLUDED_HEAD);
    s.rows = keyed.into_iter().map(|k| k.1).collect();
    s
}

/// The three sheets of the table and its counts.
pub struct Publication {
    pub identities: Sheet,
    pub excluded: Sheet,
    pub counts: Counts,
    pub sites: Vec<String>,
    pub short: Vec<String>,
    pub sorbents: Vec<String>,
    pub blank_fold: Option<f64>,
}

/// `build`: the table of the identities and the sheet of the excluded ones for `matrix` (the first matrix of the campaign when `None`).
pub fn build(c: &Campaign, io: &Io, matrix: Option<&str>) -> Result<Publication> {
    let l = Loaded::load(c, io, matrix)?;
    let (_, sites) = matrix_and_sites(c, matrix)?;
    let rows: Vec<FRow> = super::ctx::feature_rows(c, io, &l)?;
    let (gt, dd) = canonical_identities(&l);
    let fr = feature_frame(c, io, &l, &rows, &gt)?;
    let short: Vec<String> = sites.iter().map(|s| site_short(c, s)).collect();
    let x = Build {
        c,
        l: &l,
        short: short.clone(),
        blank_fold: c.presence_blank_fold,
    };
    let mut done: Vec<(Vec<Cell>, Meta)> = Vec::new();
    for it in &dd {
        done.push(x.identity_row(it, &fr)?);
    }
    done.sort_by(|a, b| {
        b.1.anthro
            .cmp(&a.1.anthro)
            .then(a.1.lv.cmp(&b.1.lv))
            .then_with(|| a.1.key.cmp(&b.1.key))
    });
    let head = columns(&sites, &short, &c.sorbents, x.blank_fold);
    let mut identities = Sheet::with_head("Identities", head);
    let mut metas = Vec::new();
    for (i, (mut row, m)) in done.into_iter().enumerate() {
        row[0] = Cell::Int(i as i64 + 1);
        identities.rows.push(row);
        metas.push(m);
    }
    let excluded = excluded_sheet(&x, &fr);
    let counts = counts_of(&metas, excluded.rows.len(), &l);
    Ok(Publication {
        identities,
        excluded,
        counts,
        sites,
        short,
        sorbents: c.sorbents.clone(),
        blank_fold: x.blank_fold,
    })
}
