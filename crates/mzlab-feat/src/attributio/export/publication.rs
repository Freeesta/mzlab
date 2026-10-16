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
use crate::attributio::annotate::Feature;
use crate::attributio::campaign::Campaign;
use crate::attributio::curation::{cec_category_label, noncec_category_label};
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::formula::{
    best_common_adduct, common_adducts, formula_mass, ion_mz, neutral_equivalent, parse_adduct, ppm,
};
use crate::attributio::frame::Cell;
use crate::attributio::identity::IdentityRow;
use crate::attributio::io::Io;
use crate::attributio::readers::standards::py_round;
use crate::attributio::rt_logp::Z_FLAG;
use std::collections::BTreeSet;

const LEVEL_LABEL: [&str; 3] = ["1", "2a", "3"];
const SOURCE_LABEL: [(&str, &str); 6] = [
    ("reference-std", "reference standard"),
    ("SIRIUS-spectral", "SIRIUS spectral match"),
    ("GNPS2", "GNPS2"),
    ("mzMine-library", "MZmine library"),
    ("CSI:FingerID", "CSI:FingerID"),
    ("reference-RT", "reference standard (mass + RT only)"),
];
/// Contaminant classes in the wording of a publication (keys: curation category).
const CLASS_LABEL: [(&str, &str); 10] = [
    ("herbicide", "pesticide (herbicide)"),
    ("insecticide", "pesticide (insecticide)"),
    ("fungicide", "pesticide (fungicide / biocide)"),
    ("pharma", "pharmaceutical"),
    ("ppcp", "personal care / lifestyle"),
    ("surfactant", "surfactant"),
    ("industrial", "industrial chemical / additive"),
    ("hormone", "hormone / steroid"),
    ("mycotoxin", "mycotoxin"),
    ("food", "food additive"),
];
const LIBRARY_L3: [&str; 4] = [
    "name conflict",
    "expert review",
    "isomers not discriminated",
    "RT isomer",
];
const STD_COLUMNS: [(&str, &str); 8] = [
    ("mz_MH_pos", "[M+H]+"),
    ("mz_MNa_pos", "[M+Na]+"),
    ("mz_MNH4_pos", "[M+NH4]+"),
    ("mz_MK_pos", "[M+K]+"),
    ("mz_MH_neg", "[M-H]-"),
    ("mz_MHCOO_neg", "[M+HCOO]-"),
    ("mz_MCl_neg", "[M+Cl]-"),
    ("mz_M2H_pos2", "[M+2H]2+"),
];

fn text(s: impl Into<String>) -> Cell {
    Cell::Str(s.into())
}

fn rounded(v: Option<f64>, n: usize) -> Cell {
    Cell::num(v.filter(|x| !x.is_nan()).map(|x| py_round(x, n)))
}

fn yes_no(b: bool) -> Cell {
    text(if b { "yes" } else { "no" })
}

fn int_of(v: Option<f64>) -> Cell {
    v.filter(|x| x.is_finite())
        .map_or(Cell::Null, |x| Cell::Int(x as i64))
}

// ── the evidence of one row ──────────────────────────────────────────────

/// `mzspec:` USI of the mzMine `usi` field (a JSON-like list or a plain text): the first one, else the text itself.
fn first_usi(v: &str) -> Option<String> {
    let t = v.trim();
    if t.is_empty() {
        return None;
    }
    let stop = |c: char| matches!(c, '"' | '\'' | ',' | ']') || c.is_whitespace();
    let mut from = 0;
    while let Some(i) = t[from..].find("mzspec:") {
        let start = from + i;
        let body = &t[start + 7..];
        let n = body.find(stop).unwrap_or(body.len());
        if n > 0 {
            return Some(t[start..start + 7 + n].to_string());
        }
        from = start + 7;
    }
    Some(t.to_string())
}

/// `('db', 'ID')` from SIRIUS `referenceLinks` such as `public_spectra_2506:(CCMSLIB00003139303)`.
fn spec_reference(v: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(v) = v.filter(|s| !s.trim().is_empty()) else {
        return (None, None);
    };
    let s = v.trim_start();
    match s.find(':').filter(|&i| i > 0) {
        None => (None, Some(v.trim().to_string())),
        Some(i) => {
            let rest = s[i + 1..].strip_prefix('(').unwrap_or(&s[i + 1..]);
            let id = rest.split(')').next().unwrap_or("").trim();
            (
                Some(s[..i].trim().to_string()),
                Some(id.to_string()).filter(|x| !x.is_empty()),
            )
        }
    }
}

/// Library, reference spectrum, cosine and matched ions of one source.
#[derive(Default)]
struct SrcEv {
    lib: Option<String>,
    reference: Option<String>,
    cos: Option<f64>,
    ions: Option<f64>,
}

fn source_evidence(p: &PFeat, src: &str) -> SrcEv {
    let f = p.f();
    match src {
        "SIRIUS-spectral" => {
            let (db, id) = spec_reference(p.ev.spec_reference.as_deref());
            SrcEv {
                lib: Some(match db {
                    Some(d) => format!("SIRIUS spectral library ({d})"),
                    None => "SIRIUS spectral library".into(),
                }),
                reference: id,
                cos: f.spec.sim,
                ions: f.spec.peaks,
            }
        }
        "GNPS2" => SrcEv {
            lib: Some(
                match p.ev.gnps_library.as_deref().filter(|l| !l.is_empty()) {
                    Some(l) => format!("GNPS2 ({l})"),
                    None => "GNPS2".into(),
                },
            ),
            reference: p.ev.gnps_spectrum_id.clone(),
            cos: f.gnps.score,
            ions: f.gnps.peaks,
        },
        "mzMine-library" => SrcEv {
            lib: Some("MZmine spectral library".into()),
            reference: p
                .fr
                .mz_usi
                .as_deref()
                .and_then(first_usi)
                .or_else(|| p.fr.mz_entry.clone()),
            cos: f.mzlib.score,
            ions: f.mzlib.peaks,
        },
        _ => SrcEv::default(),
    }
}

fn has_source(f: &Feature, s: &str) -> bool {
    f.sources.contains(&s)
}

/// The score that decided the level of a feature, for choosing the representative.
fn score(p: &PFeat) -> f64 {
    let f = p.f();
    let lib = |s: &str, v: Option<f64>| v.filter(|_| has_source(f, s)).filter(|x| !x.is_nan());
    let v = if f.level == 1 {
        f.std.as_ref().and_then(|s| s.cosine)
    } else if f.level == 2 || LIBRARY_L3.contains(&f.level_basis.as_str()) {
        [
            lib("SIRIUS-spectral", f.spec.sim),
            lib("GNPS2", f.gnps.score),
            lib("mzMine-library", f.mzlib.score),
            Some(-1.0),
        ]
        .into_iter()
        .flatten()
        .reduce(f64::max)
    } else {
        f.csi.cosmic
    };
    v.filter(|x| !x.is_nan()).unwrap_or(-1.0)
}

fn largest(p: &PFeat) -> f64 {
    p.max_area
        .iter()
        .copied()
        .filter(|a| !a.is_nan())
        .fold(f64::NEG_INFINITY, f64::max)
}

/// The best-evidence feature at the best level of the row (score, then largest area; the first of equals).
fn representative<'a, 'b>(sub: &'b [&'a PFeat<'a>]) -> &'b PFeat<'a> {
    let best = sub.iter().map(|p| p.f().level).min().unwrap_or(3);
    let mut pick: Option<(&PFeat, f64, f64)> = None;
    for p in sub.iter().copied().filter(|p| p.f().level == best) {
        let (s, a) = (score(p), largest(p));
        if pick.is_none_or(|(_, ps, pa)| s > ps || (s == ps && a > pa)) {
            pick = Some((p, s, a));
        }
    }
    pick.map(|x| x.0).expect("a feature at the best level")
}

/// The passing library source with the highest cosine (every library has the same gates; the first of equals).
fn best_source(p: &PFeat) -> Option<&'static str> {
    let mut best = (None, -1.0);
    for s in ["SIRIUS-spectral", "GNPS2", "mzMine-library"] {
        if has_source(p.f(), s) {
            let c = source_evidence(p, s)
                .cos
                .filter(|x| !x.is_nan())
                .unwrap_or(-1.0);
            if c > best.1 {
                best = (Some(s), c);
            }
        }
    }
    best.0
}

/// `(\[\d*M[^\]]*\]\d*[+-])\s*$` found in a GNPS2 name such as `Valsartan [M-H]-`.
fn adduct_in_name(name: &str) -> Option<String> {
    let t = name.trim_end();
    for (i, _) in t.match_indices('[') {
        let body = &t[i + 1..];
        let digits = body.len() - body.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let Some(rest) = body[digits..].strip_prefix('M') else {
            continue;
        };
        let Some(close) = rest.find(']') else {
            continue;
        };
        let tail = &rest[close + 1..];
        let charge = tail.trim_start_matches(|c: char| c.is_ascii_digit());
        if charge == "+" || charge == "-" {
            return Some(t[i..].to_string());
        }
    }
    None
}

/// The ion type reported for the representative feature: the reference standard (Level 1), the source of the name, then the other exact-mass library hits.
fn reported_adduct(p: &PFeat, lv: i32) -> Option<String> {
    let f = p.f();
    if lv == 1 {
        if let Some(s) = &f.std {
            return Some(s.adduct.clone());
        }
    }
    if let Some(a) = p.ev.adduct.as_deref().filter(|a| !a.trim().is_empty()) {
        return Some(a.to_string());
    }
    if f.mzlib.exact {
        if let Some(a) = &f.mzlib.adduct {
            return Some(a.clone());
        }
    }
    if f.gnps.exact {
        if let Some(a) = f.gnps.name.as_deref().and_then(adduct_in_name) {
            return Some(a);
        }
    }
    None
}

/// The adduct in the usual notation (`[M + H3N + H]+` of SIRIUS gives `[M+NH4]+`); `None` for a blank text.
fn common_adduct(adduct: &str, pol: &str) -> Option<String> {
    if adduct.trim().is_empty() {
        return None;
    }
    let a = adduct.replace(' ', "");
    let Some(p) = parse_adduct(&a) else {
        return Some(a);
    };
    for c in common_adducts(pol) {
        if let Some(q) = parse_adduct(c) {
            if q.0 == p.0 && q.2 == p.2 && (q.1 - p.1).abs() < 1e-6 {
                return Some((*c).to_string());
            }
        }
    }
    Some(a)
}

/// The adduct, the mass error and whether the adduct was inferred from the m/z.
struct IonFit {
    adduct: Option<String>,
    err: Option<f64>,
    mono: Option<f64>,
    inferred: bool,
}

fn ion_fit(rep: &PFeat, lv: i32, formula: Option<&str>) -> IonFit {
    let f = rep.f();
    let std = |a: String| {
        STD_COLUMNS
            .iter()
            .find(|(k, _)| *k == a)
            .map_or(a, |(_, v)| (*v).to_string())
    };
    let mut adduct = reported_adduct(rep, lv)
        .map(std)
        .and_then(|a| common_adduct(&a, &f.pol));
    let mono = formula.and_then(formula_mass);
    let (mut err, mut inferred) = (None, false);
    if let (Some(m), Some(fm)) = (mono, formula) {
        let n = neutral_equivalent(m, fm);
        let th = adduct
            .as_deref()
            .filter(|a| !a.is_empty())
            .and_then(|a| ion_mz(n, a));
        match th {
            None => {
                let (a, e) = best_common_adduct(f.mz.unwrap_or(f64::NAN), n, &f.pol);
                inferred = !a.is_empty();
                adduct = Some(a);
                err = Some(e);
            }
            Some(t) => err = Some(ppm(f.mz.unwrap_or(f64::NAN), t)),
        }
    }
    IonFit {
        adduct,
        err,
        mono,
        inferred,
    }
}

/// `_structure` without the registry: the InChIKey (the connectivity block when the source gives no full key), the SMILES and the source that named it.
fn structure(rep: &PFeat) -> (Cell, Cell, Cell) {
    let e = &rep.ev;
    let Some(block) = e.name_block.as_deref() else {
        return (text(""), text(""), text(""));
    };
    let ik = e
        .name_inchikey
        .as_deref()
        .filter(|k| !k.is_empty())
        .unwrap_or(block);
    (
        text(ik),
        Cell::str(&e.name_smiles),
        text(e.name_source.clone()),
    )
}

// ── flags ────────────────────────────────────────────────────────────────

fn word_isomers(low: &str) -> bool {
    let w = |c: char| c.is_alphanumeric() || c == '_';
    low.match_indices("isomers").any(|(i, m)| {
        !low[..i].chars().next_back().is_some_and(w)
            && !low[i + m.len()..].chars().next().is_some_and(w)
    })
}

/// A parenthesis without parentheses inside that holds `\w/\w`.
fn slash_group(low: &str) -> bool {
    let w = |c: char| c.is_alphanumeric() || c == '_';
    low.match_indices('(').any(|(i, _)| {
        let body = &low[i + 1..];
        let Some(end) = body
            .find(['(', ')'])
            .filter(|&e| body[e..].starts_with(')'))
        else {
            return false;
        };
        let chars: Vec<char> = body[..end].chars().collect();
        chars.windows(3).any(|t| t[1] == '/' && w(t[0]) && w(t[2]))
    })
}

/// `_SERIES`: a name that groups isomers (a polyether series by formula, `isomers`, a curated union such as `(sucrose/palatinose)`).
fn is_series(nome: &str) -> bool {
    let low = nome.to_lowercase();
    low.starts_with("peg/polyether [") || word_isomers(&low) || slash_group(&low)
}

struct FlagInput<'a, 'b> {
    sub: &'b [&'a PFeat<'a>],
    rep: &'b PFeat<'a>,
    best_feats: &'b [&'a PFeat<'a>],
    nome: &'b str,
    lv: i32,
    lv_sub: &'b str,
    watch: &'b [String],
}

fn count_mzlib_only(sub: &[&PFeat]) -> usize {
    sub.iter()
        .filter(|p| p.f().level_flag.as_deref() == Some("mzlib-only"))
        .count()
}

fn flags(x: &FlagInput, inferred: bool) -> Vec<String> {
    let (sub, rep, lv) = (x.sub, x.rep, x.lv);
    let basis = rep.f().level_basis.as_str();
    let mut fl: Vec<String> = Vec::new();
    if inferred {
        fl.push(FLAG_ADDUCT.into());
    }
    let n_l2 = sub.iter().filter(|p| p.f().level == 2).count();
    let n_mz = count_mzlib_only(sub);
    if n_mz > 0 {
        let more = if n_mz < n_l2 {
            format!(" ({n_mz} of {n_l2} Level 2a features)")
        } else {
            String::new()
        };
        fl.insert(0, format!("{FLAG_MZLIB_ONLY}{more}"));
    }
    if lv == 3 && basis == "name conflict" {
        fl.insert(0, FLAG_NAME_CONFLICT.into());
    }
    let choices: BTreeSet<&str> = sub
        .iter()
        .filter_map(|p| p.f().name_choice.as_deref())
        .collect();
    if !choices.is_empty() {
        let list: Vec<&str> = choices.into_iter().collect();
        fl.push(format!("{FLAG_LIBRARY_CHOICE} ({})", list.join("; ")));
    }
    if lv == 3 && basis == "expert review" {
        fl.insert(0, FLAG_EXPERT.into());
    }
    if lv == 3 && basis == "RT isomer" {
        fl.insert(0, FLAG_RT_ISOMER.into());
    }
    if x.lv_sub == "b" {
        fl.insert(0, FLAG_2B.into());
    }
    tail_flags(x, &mut fl);
    fl
}

fn tail_flags(x: &FlagInput, fl: &mut Vec<String>) {
    let sub = x.sub;
    let blocks: BTreeSet<&str> = sub
        .iter()
        .filter(|p| p.f().decision.feature_decision.is_empty())
        .filter_map(|p| p.ev.name_block.as_deref())
        .collect();
    if is_series(x.nome)
        || blocks.len() > 1
        || sub
            .iter()
            .any(|p| p.f().level_basis == "isomers not discriminated")
    {
        fl.push(FLAG_ISOMERS.into());
    }
    if sub
        .iter()
        .any(|p| p.f().rt_info.rt_groups.is_some_and(|g| g > 1))
    {
        fl.push(FLAG_RT_SPLIT.into());
    }
    if sub.iter().any(|p| p.fr.rt_outlier) {
        fl.push(format!(
            "{FLAG_RT_LOGP} (|z| > {})",
            mzlab_core::json::fmt_g(Z_FLAG)
        ));
    }
    let mut names = vec![x.nome.to_string()];
    names.extend(
        sub.iter()
            .map(|p| p.ev.identity.clone())
            .collect::<BTreeSet<_>>(),
    );
    let names = names.join(" ").to_lowercase();
    if x.watch.iter().any(|w| names.contains(w.as_str())) {
        fl.push(FLAG_WATCHLIST.into());
    }
    if x.lv == 1
        && x.best_feats.iter().any(|p| {
            p.f()
                .std
                .as_ref()
                .is_some_and(|s| s.status == "confirmed_manual")
        })
    {
        fl.push(FLAG_L1_MANUAL.into());
    }
}

// ── the table ────────────────────────────────────────────────────────────

/// What the counts and the sort need of a row.
struct Meta {
    lv: i32,
    anthro: bool,
    sub_b: bool,
    n_mz: usize,
    core: bool,
    flags: String,
    key: String,
}

fn is_isomer_name(name: &str) -> bool {
    let Some(i) = name.rfind(" isomer, RT ") else {
        return false;
    };
    let tail = &name[i + 12..];
    tail.strip_suffix(" min")
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '.'))
}

/// The hazard columns of a compound that no registry knows: the fixed record with its caveats.
fn hazard_cells(nome: &str, lv: i32) -> Vec<Cell> {
    let mut caveats: Vec<&str> = Vec::new();
    if is_isomer_name(nome) {
        caveats.push("RT isomer: hazard of the named structure, the peak may be another isomer");
    }
    if lv == 3 {
        caveats.push("Level 3: hazard of the CSI:FingerID candidate structure");
    }
    caveats.push("no structure in the registry");
    vec![
        text("Not assessable"),
        text("not assessable"),
        text(""),
        text("Not assessable"),
        text("not assessable"),
        text(""),
        text(caveats.join("; ")),
        text(""),
    ]
}

struct Build<'a> {
    c: &'a Campaign,
    l: &'a Loaded,
    short: Vec<String>,
    blank_fold: Option<f64>,
}

fn class_label(it: &Dd) -> String {
    if !it.is_anthro {
        return String::new();
    }
    let cats = it.cec_category.as_str();
    CLASS_LABEL
        .iter()
        .find(|(k, _)| *k == cats)
        .map(|(_, v)| (*v).to_string())
        .or_else(|| cec_category_label(cats).map(str::to_string))
        .unwrap_or_else(|| cats.to_string())
        .to_lowercase()
}

fn evidence_cells(best_feats: &[&PFeat], rep: &PFeat, lv: i32) -> Vec<Cell> {
    let mut srcs: BTreeSet<&str> = BTreeSet::new();
    for p in best_feats {
        srcs.extend(p.f().sources.iter().copied());
    }
    let evidence: Vec<&str> = SOURCE_LABEL
        .iter()
        .filter(|(k, _)| srcs.contains(k))
        .map(|(_, v)| *v)
        .collect();
    let f = rep.f();
    let (mut lib, mut reference, mut cos, mut ions) = (None, None, None, None);
    if lv == 2 || LIBRARY_L3.contains(&f.level_basis.as_str()) {
        if let Some(s) = best_source(rep) {
            let e = source_evidence(rep, s);
            (lib, reference, cos, ions) = (e.lib, e.reference, e.cos, e.ions);
        }
    } else if lv == 1 || (!has_source(f, "CSI:FingerID") && has_source(f, "reference-RT")) {
        lib = Some("in-house reference standard (same instrument)".to_string());
        reference = f.std.as_ref().and_then(|s| s.name.clone());
    } else if has_source(f, "CSI:FingerID") {
        lib = Some("CSI:FingerID (SIRIUS)".to_string());
    }
    vec![
        text(evidence.join("; ")),
        text(lib.unwrap_or_default()),
        text(reference.unwrap_or_default()),
        rounded(cos, 2),
        int_of(ions),
    ]
}

fn std_cells(rep: &PFeat, lv: i32) -> Vec<Cell> {
    let f = rep.f();
    let s = f.std.as_ref().filter(|_| lv == 1);
    vec![
        rounded(f.csi.cosmic.filter(|_| lv == 3), 2),
        rounded(s.map(|s| s.ppm), 1),
        rounded(s.map(|s| s.drt), 2),
        rounded(s.and_then(|s| s.cosine), 2),
        s.and_then(|s| s.matched).map_or(Cell::Null, Cell::Int),
    ]
}

fn feature_ids(sub: &[&PFeat]) -> String {
    let mut ids: Vec<(bool, i64, &str)> = sub
        .iter()
        .map(|p| (p.f().pol != "POS", p.f().id, p.f().pol.as_str()))
        .collect();
    ids.sort();
    ids.iter()
        .map(|(_, i, p)| format!("{p}:{i}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn rt_range(sub: &[&PFeat]) -> String {
    let rts = || sub.iter().filter_map(|p| p.f().rt).filter(|x| !x.is_nan());
    let (lo, hi) = (
        rts().fold(f64::NAN, f64::min),
        rts().fold(f64::NAN, f64::max),
    );
    let t = |x: f64| {
        if x.is_nan() {
            "nan".to_string()
        } else {
            format!("{x:.2}")
        }
    };
    format!("{}-{}", t(lo), t(hi))
}

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
        let mut row = vec![
            Cell::Int(0),
            text(it.nome.clone()),
            text(if lv_sub == "b" {
                "2b"
            } else {
                LEVEL_LABEL[lv as usize - 1]
            }),
            text(if it.is_anthro {
                "anthropogenic contaminant"
            } else {
                "not classified as anthropogenic"
            }),
            text(class_label(it)),
        ];
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
