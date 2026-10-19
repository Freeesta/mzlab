// Ported from Attributio reglists/confirm.py.
//! B3: MS2 confirmation of the B2 candidates (classes A, B, C) from the exports of SIRIUS (structures, formulas, library matches), an optional MetFrag
//! summary (support only) and the chemist's decisions. Level 3 needs (a) the suspect among the top 5 structures, or top 1 with COSMIC ≥ 0.64, (b) a
//! coherent RT and (c) the suspect formula as the top-1 SIRIUS formula; Level 2a needs an identity library match with cosine ≥ 0.70 and ≥ 6 shared peaks.
//! A decision of the chemist applies only to candidates that pass the formal rules; the formal verdict stays in `rules`. The inputs are plain rows.
//! Not ported here: `prepare` (the input files for SIRIUS and MetFrag are builders, see the native builders package).

use super::screen::Hit;
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::tox::vocab::canon;
use std::collections::{BTreeMap, HashMap};

pub const COSMIC_HIGH: f64 = 0.64;
pub const COS_MIN: f64 = 0.70;
pub const PEAKS_MIN: i64 = 6;
pub const LEVEL_2A: &str = "Level 2a";
pub const LEVEL_3: &str = "Level 3";
pub const LEVEL_3_RESERVED: &str = "Level 3 with reservation";
pub const REJECTED: &str = "rejected";
pub const UNCONFIRMED: &str = "unconfirmed";
pub const LEVEL_PREFIX: &str = "Level ";

/// One structure candidate of SIRIUS for a feature (`rank` missing counts as 1).
#[derive(Clone, Debug, Default)]
pub struct StructureIn {
    pub id: i64,
    pub rank: Option<i64>,
    pub inchikey2d: String,
    pub name: Option<String>,
    pub cosmic: Option<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct FormulaIn {
    pub id: i64,
    pub rank: Option<i64>,
    pub formula: String,
}

/// One spectral-library match; `analog` matches are never evidence of identity.
#[derive(Clone, Debug, Default)]
pub struct MatchIn {
    pub id: i64,
    pub inchikey2d: Option<String>,
    pub similarity: f64,
    pub shared_peaks: i64,
    pub analog: bool,
}

/// The SIRIUS tables of one polarity and the kind of input they come from («B3 v2», or a pre-check on the campaign's own run).
#[derive(Clone, Debug, Default)]
pub struct SiriusIn {
    pub kind: String,
    pub structures: Vec<StructureIn>,
    pub formulas: Vec<FormulaIn>,
    pub matches: Vec<MatchIn>,
}

/// One row of the MetFrag summary.
#[derive(Clone, Debug, Default)]
pub struct MetFragIn {
    pub pol: String,
    pub fid: i64,
    pub adduct: String,
    pub suspect: String,
    pub rank: Option<f64>,
    pub expl: Option<f64>,
    pub n_cand: Option<f64>,
}

/// A decision of the chemist; empty polarity or feature id match any.
#[derive(Clone, Debug, Default)]
pub struct Decision {
    pub suspect: String,
    pub polarity: String,
    pub feature_id: String,
    pub verdict: String,
    pub note: String,
    pub decided_on: String,
}

/// One candidate × feature with its verdict.
#[derive(Clone, Debug, PartialEq)]
pub struct Verdict {
    pub verdict: String,
    pub rules: String,
    pub note: String,
    pub decided_on: String,
    pub class_b2: String,
    pub suspect: String,
    pub lists: String,
    pub list_attributes: String,
    pub is_tp: String,
    pub parent: String,
    pub polarity: String,
    pub feature_id: i64,
    pub adduct: String,
    pub mz_obs: f64,
    pub ppm: f64,
    pub rt: f64,
    pub rt_pred: Option<f64>,
    pub formula_suspect: String,
    pub sirius_formula_top1: Option<String>,
    pub suspect_formula_rank: Option<i64>,
    pub sirius_structure_rank: Option<i64>,
    pub sirius_top1: Option<String>,
    pub cosmic_top1: Option<f64>,
    pub lib_cosine: Option<f64>,
    pub lib_shared_peaks: Option<i64>,
    pub metfrag_rank: Option<f64>,
    pub metfrag_expl_peaks: Option<f64>,
    pub metfrag_n_cand: Option<f64>,
    pub a_top5_or_cosmic: bool,
    pub b_rt_logp: bool,
    pub c_formula: bool,
    pub in_blanks: String,
    pub sites: String,
    pub sorbent: String,
    pub isolated_adduct: bool,
    pub suspect_skeleton: String,
    pub sirius_input: String,
}

/// Element counts of a molecular formula (`C9H10N2O` and `N2OC9H10` are equal); anything that is not an element symbol is skipped.
pub fn parse_formula(f: &str) -> BTreeMap<String, u64> {
    let c: Vec<char> = f.chars().collect();
    let mut d = BTreeMap::new();
    let mut i = 0;
    while i < c.len() {
        if !c[i].is_ascii_uppercase() {
            i += 1;
            continue;
        }
        let mut el = c[i].to_string();
        i += 1;
        if i < c.len() && c[i].is_ascii_lowercase() {
            el.push(c[i]);
            i += 1;
        }
        let start = i;
        while i < c.len() && c[i].is_ascii_digit() {
            i += 1;
        }
        let n = if i > start {
            c[start..i].iter().collect::<String>().parse().unwrap_or(0)
        } else {
            1
        };
        *d.entry(el).or_insert(0) += n;
    }
    d
}

fn rank_of(r: Option<i64>) -> i64 {
    r.unwrap_or(1)
}

/// What SIRIUS says about one hit: the suspect's structure and formula ranks, the top-1 structure and formula, the best identity library match.
struct Evidence3 {
    s_rank: Option<i64>,
    top1: Option<StructureIn>,
    f1: Option<String>,
    f_rank: Option<i64>,
    lib: Option<(f64, i64)>,
}

fn gather(h: &Hit, sir: &SiriusIn) -> Evidence3 {
    let mut s: Vec<&StructureIn> = sir
        .structures
        .iter()
        .filter(|x| x.id == h.feature_id)
        .collect();
    s.sort_by_key(|x| rank_of(x.rank));
    let mut f: Vec<&FormulaIn> = sir
        .formulas
        .iter()
        .filter(|x| x.id == h.feature_id)
        .collect();
    f.sort_by_key(|x| rank_of(x.rank));
    let want = parse_formula(&h.formula);
    let mut lib: Option<(f64, i64)> = None;
    let matches = sir.matches.iter().filter(|m| {
        m.id == h.feature_id
            && !m.analog
            && m.inchikey2d.as_deref() == Some(h.suspect_skeleton.as_str())
    });
    for m in matches {
        if lib.is_none_or(|(c, _)| m.similarity > c) {
            lib = Some((m.similarity, m.shared_peaks));
        }
    }
    Evidence3 {
        s_rank: s
            .iter()
            .filter(|x| x.inchikey2d == h.suspect_skeleton)
            .map(|x| rank_of(x.rank))
            .min(),
        top1: s.first().map(|x| (*x).clone()),
        f1: f.first().map(|x| x.formula.clone()),
        f_rank: f
            .iter()
            .filter(|x| parse_formula(&x.formula) == want)
            .map(|x| rank_of(x.rank))
            .min(),
        lib,
    }
}

/// The formal verdict of the rules.
fn formal(e: &Evidence3, rt_ok: bool) -> (String, [bool; 3]) {
    let cosmic = e.top1.as_ref().and_then(|t| t.cosmic);
    let a = e.s_rank.is_some_and(|r| r <= 5)
        || (e.s_rank == Some(1) && cosmic.is_some_and(|c| c >= COSMIC_HIGH));
    let c = e.f_rank == Some(1);
    let abc = [a, rt_ok, c];
    let verdict = if e
        .lib
        .is_some_and(|(cos, peaks)| cos >= COS_MIN && peaks >= PEAKS_MIN)
    {
        LEVEL_2A.to_string()
    } else if a && rt_ok && c {
        LEVEL_3.to_string()
    } else {
        let failed: Vec<&str> = ["a", "b", "c"]
            .iter()
            .zip(abc)
            .filter(|(_, ok)| !ok)
            .map(|(n, _)| *n)
            .collect();
        format!("{UNCONFIRMED} ({})", failed.join(","))
    };
    (verdict, abc)
}

fn decide<'a>(dec: &'a [Decision], h: &Hit) -> Option<&'a Decision> {
    let fid = h.feature_id.to_string();
    dec.iter().rev().find(|d| {
        d.suspect == h.suspect
            && (d.polarity.is_empty() || d.polarity == h.polarity)
            && (d.feature_id.is_empty() || d.feature_id == fid)
    })
}

fn order_of(v: &str) -> u8 {
    match v {
        LEVEL_2A => 0,
        LEVEL_3 => 1,
        LEVEL_3_RESERVED => 2,
        REJECTED => 3,
        _ => 4,
    }
}

fn verdict_row(h: &Hit, sir: &SiriusIn, mf: Option<&MetFragIn>, dec: &[Decision]) -> Verdict {
    let e = gather(h, sir);
    let (rules, [a, b, c]) = formal(&e, canon(&h.rt_logp) == "yes");
    let (mut verdict, mut note, mut who) = (rules.clone(), String::new(), String::new());
    if rules == LEVEL_3 || rules == LEVEL_2A {
        if let Some(d) = decide(dec, h) {
            (verdict, note, who) = (canon(&d.verdict), d.note.clone(), d.decided_on.clone());
        }
    }
    Verdict {
        verdict,
        rules,
        note,
        decided_on: who,
        class_b2: h.class.clone(),
        suspect: h.suspect.clone(),
        lists: h.list_labels.clone(),
        list_attributes: h.list_attributes.clone(),
        is_tp: h.is_tp.clone(),
        parent: h.parent.clone(),
        polarity: h.polarity.clone(),
        feature_id: h.feature_id,
        adduct: h.adduct.clone(),
        mz_obs: h.mz_obs,
        ppm: h.ppm,
        rt: h.rt,
        rt_pred: h.rt_pred,
        formula_suspect: h.formula.clone(),
        sirius_formula_top1: e.f1,
        suspect_formula_rank: e.f_rank,
        sirius_structure_rank: e.s_rank,
        sirius_top1: e.top1.as_ref().and_then(|t| t.name.clone()),
        cosmic_top1: e.top1.as_ref().and_then(|t| t.cosmic),
        lib_cosine: e.lib.map(|l| l.0),
        lib_shared_peaks: e.lib.map(|l| l.1),
        metfrag_rank: mf.and_then(|m| m.rank),
        metfrag_expl_peaks: mf.and_then(|m| m.expl),
        metfrag_n_cand: mf.and_then(|m| m.n_cand),
        a_top5_or_cosmic: a,
        b_rt_logp: b,
        c_formula: c,
        in_blanks: h.in_blanks.clone(),
        sites: h.sites.clone(),
        sorbent: h.sorbent.clone(),
        isolated_adduct: h.isolated_adduct,
        suspect_skeleton: h.suspect_skeleton.clone(),
        sirius_input: sir.kind.clone(),
    }
}

/// One row per B2 candidate of class A, B or C and feature, sorted by verdict (Level 2a first), suspect and feature. `sirius` maps a polarity to its tables.
pub fn evaluate(
    hits: &[Hit],
    sirius: &BTreeMap<String, SiriusIn>,
    metfrag: &[MetFragIn],
    decisions: &[Decision],
) -> Result<Vec<Verdict>> {
    let mf: HashMap<(&str, i64, &str, &str), &MetFragIn> = metfrag
        .iter()
        .map(|m| {
            (
                (m.pol.as_str(), m.fid, m.adduct.as_str(), m.suspect.as_str()),
                m,
            )
        })
        .collect();
    let mut rows = Vec::new();
    for h in hits
        .iter()
        .filter(|h| ["A", "B", "C"].contains(&h.class.as_str()))
    {
        let sir = sirius.get(&h.polarity).ok_or_else(|| {
            CampaignError::value(&format!("no SIRIUS input for polarity {}", h.polarity))
        })?;
        let short: String = h.suspect.chars().take(30).collect();
        rows.push(verdict_row(
            h,
            sir,
            mf.get(&(
                h.polarity.as_str(),
                h.feature_id,
                h.adduct.as_str(),
                short.as_str(),
            ))
            .copied(),
            decisions,
        ));
    }
    rows.sort_by(|a, b| {
        (order_of(&a.verdict), &a.suspect, a.feature_id).cmp(&(
            order_of(&b.verdict),
            &b.suspect,
            b.feature_id,
        ))
    });
    Ok(rows)
}

/// The candidates confirmed after the chemist's decisions (Level 2a, 3 and 3 with reservation).
pub fn accepted(rows: &[Verdict]) -> Vec<&Verdict> {
    rows.iter()
        .filter(|r| r.verdict.starts_with(LEVEL_PREFIX))
        .collect()
}

pub const METHOD: [&str; 6] = [
    "Level 3: (a) suspect in PubChem top-5 (SIRIUS, PubChem DB + list + public spectra) or top-1 with COSMIC >= 0.64; (b) RT coherent with logP (B2); (c) suspect formula = SIRIUS top-1 formula.",
    "Level 2a: library match (identity, not analog) on the suspect with cosine >= 0.70 and >= 6 shared peaks.",
    "MetFrag (PubChemLite, FragmenterScore) reported as support, does not enter the verdict.",
    "The chemist's decisions (campaign.toml [reglists] b3_decisions) apply only to candidates that pass the formal rules; the formal verdict stays in 'verdetto_regole'.",
    "No counts modified: a confirmed compound that adds an identity must be flagged and decided by the chemist.",
    "Without B3 export the campaign's SIRIUS is used: it is a PRE-CHECK (search on PubChem already done), not a B3 verdict (sirius_input column).",
];
