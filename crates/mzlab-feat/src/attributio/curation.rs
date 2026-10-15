// Ported from Attributio curation.py.
//! The chemical curation: which name is which compound. The decisions are data: three `;`-separated tables that a campaign names in `[curation]`
//! (`name_map`: pattern → curated contaminant and category, the first matching row wins; `category_override`: substring → non-contaminant bucket;
//! `identity_formula_override`: corrected formula of an identity). They are never part of the program: a campaign without them curates nothing (every name is
//! its own identity, without category). `TableCuration` is the `Curation` the cascade and the identity table read through.

use super::annotate::curate::{Cec, Curation};
use super::error::{CampaignError, Result};
use super::frame::Frame;
use super::io::Io;
use super::names;
use super::pattern::Pattern;
use super::Campaign;
use std::collections::HashMap;

/// Contaminant categories, in the order of the figures.
pub const CEC_CATEGORIES: [&str; 11] = [
    "herbicide",
    "insecticide",
    "fungicide",
    "pharma",
    "ppcp",
    "surfactant",
    "industrial",
    "hormone",
    "mycotoxin",
    "food",
    "is",
];

/// The label of a contaminant category.
pub fn cec_category_label(cat: &str) -> Option<&'static str> {
    Some(match cat {
        "herbicide" => "Herbicide",
        "insecticide" => "Insecticide",
        "fungicide" => "Fungicide / biocide",
        "pharma" => "Pharmaceutical / antibiotic",
        "ppcp" => "Personal-care / lifestyle",
        "surfactant" => "Surfactant / detergent",
        "industrial" => "Industrial / additives",
        "hormone" => "Hormone / steroid",
        "mycotoxin" => "Mycotoxin",
        "food" => "Food additive",
        "is" => "Internal standard",
        _ => return None,
    })
}

/// The label of a non-contaminant bucket (`natural` is the default for a name the map does not recognise, not a chemical class).
pub fn noncec_category_label(cat: &str) -> Option<&'static str> {
    Some(match cat {
        "natural" => "Not classified as anthropogenic",
        "ambiguous" => "Unidentified (library code)",
        "illicit_drug" => "Illicit drug biomarker",
        "lab_artifact" => "Analytical artifact / internal standard",
        _ => return None,
    })
}

/// The contaminants labelled in the overview figures.
const HEADLINE: &[&str] = &[
    "Carbamazepine",
    "Sulfamethoxazole (antibiotic)",
    "Ciprofloxacin (antibiotic)",
    "DEET",
    "Icaridin",
    "Ensulizole (UV filter)",
    "Caffeine",
    "Cotinine",
    "Ametryn",
    "Hydroxyatrazine",
    "Bentazon",
    "Bispyribac",
    "Carbendazim",
    "Azoxystrobin",
    "Metalaxyl",
    "Triazophos",
    "Trifloxystrobin",
    "Nonylphenol ethoxylate (NPEC)",
    "Linear alkylbenzene sulfonate (LAS)",
    "Linear alkylbenzene sulfonate (C10-LAS)",
    "TBEP",
    "TBOEP / TBEP (flame retardant)",
    "Dibutyl phthalate",
    "Diphenylguanidine (tyre)",
    "Pentachlorophenol",
    "Pentachlorophenol (wood preservative)",
    "Zearalenone (mycotoxin)",
    "Neotame (sweetener)",
    "Galaxolidone (musk)",
    "Lincomycin (antibiotic)",
];

fn ascii_ci_find(hay: &str, needle: &str) -> Option<usize> {
    hay.to_ascii_lowercase().find(&needle.to_ascii_lowercase())
}

/// `\s*\[CCS=[^\]]*\]` removed everywhere.
fn drop_ccs(s: &str) -> String {
    let mut out = s.to_string();
    while let Some(i) = out.find("[CCS=") {
        let Some(j) = out[i..].find(']') else {
            break;
        };
        let head = out[..i].trim_end().to_string();
        out = format!("{head}{}", &out[i + j + 1..]);
    }
    out
}

/// `\s*CollisionEnergy:?\S*` (any case) removed everywhere.
fn drop_collision_energy(s: &str) -> String {
    let mut out = s.to_string();
    while let Some(i) = ascii_ci_find(&out, "collisionenergy") {
        let head = out[..i].trim_end().to_string();
        let tail = &out[i + "collisionenergy".len()..];
        let tail = tail.strip_prefix(':').unwrap_or(tail);
        let rest = tail.trim_start_matches(|c: char| !c.is_whitespace());
        out = format!("{head}{rest}");
    }
    out
}

/// Whether a token is `[M+H]`, `M-H`, `[M+H]+`, `M+H-` ... (`\[?M[+-]H\]?[+-]?`).
fn is_adduct_token(t: &str) -> bool {
    let t = t.strip_prefix('[').unwrap_or(t);
    let Some(t) = t.strip_prefix('M') else {
        return false;
    };
    let mut c = t.chars();
    if !matches!(c.next(), Some('+' | '-')) || c.next() != Some('H') {
        return false;
    }
    let rest: String = c.collect();
    let rest = rest.strip_prefix(']').unwrap_or(&rest);
    rest.is_empty() || rest == "+" || rest == "-"
}

/// The adduct suffix, collision energy and CCS removed from a raw library name.
pub fn clean_library_name(name: &str) -> String {
    let s = drop_collision_energy(&drop_ccs(name));
    let t = s.trim_end();
    if let Some(k) = t.rfind(char::is_whitespace) {
        if is_adduct_token(&t[k..].trim_start()) && !t[..k].is_empty() {
            return t[..k].trim().to_string();
        }
    }
    s.trim().to_string()
}

fn ambiguous_code() -> &'static Pattern {
    static P: std::sync::OnceLock<Pattern> = std::sync::OnceLock::new();
    P.get_or_init(|| {
        Pattern::new(
            r"^(oprea|acmc|ncgc|tocris|bmse|acon|spa-|compound\b|compound\d|epitope|zinc|cid[\s:]|np-\d|chebi|smr|stk|mls|cas-|z\d{6,}|.*\(tentative\)|.*\(sample\))",
        )
    })
}

/// A confident non-contaminant name is `natural`, or `ambiguous` when it is a library code (Oprea, ZINC, NCGC ...). A string heuristic, not a taxonomy.
pub fn classify_noncec(name: Option<&str>) -> &'static str {
    match name.map(str::trim) {
        Some(n) if !n.is_empty() && !ambiguous_code().search(&n.to_lowercase()) => "natural",
        _ => "ambiguous",
    }
}

fn blank(s: Option<&str>) -> bool {
    s.is_none_or(|s| s.trim().is_empty())
}

/// The curation read from the tables of a campaign.
#[derive(Clone, Debug, Default)]
pub struct TableCuration {
    /// `(pattern, canonical name, category)`, in file order.
    name_map: Vec<(Pattern, String, String)>,
    /// `(substring, canonical name, category)`, in file order.
    category_override: Vec<(String, String, String)>,
    formula_override: HashMap<String, String>,
}

fn rows3(text: Option<&str>, what: &str, key: &str) -> Result<Vec<(String, String, String)>> {
    let Some(text) = text else {
        return Ok(Vec::new());
    };
    let f = Frame::parse(text, ';')?;
    let col = |n: &str| {
        f.col(n)
            .ok_or_else(|| CampaignError::campaign(format!("{what}: the column '{n}' is missing")))
    };
    let (p, l, c) = (col(key)?, col("canonical_name")?, col("category")?);
    Ok((0..f.len())
        .map(|r| {
            (
                f.cell(r, p).to_string(),
                f.cell(r, l).to_string(),
                f.cell(r, c).to_string(),
            )
        })
        .collect())
}

impl TableCuration {
    /// From the text of the three tables (`None`: the table is not there).
    pub fn from_texts(
        name_map: Option<&str>,
        category_override: Option<&str>,
        formula_override: Option<&str>,
    ) -> Result<TableCuration> {
        let name_map = rows3(name_map, "name_map", "pattern")?
            .into_iter()
            .map(|(p, l, c)| (Pattern::new(&p), l, c))
            .collect();
        let category_override = rows3(category_override, "category_override", "pattern")?;
        let mut formula = HashMap::new();
        if let Some(t) = formula_override {
            let f = Frame::parse(t, ';')?;
            let (k, v) = (f.col("identity_key"), f.col("corrected_formula"));
            let (Some(k), Some(v)) = (k, v) else {
                return Err(CampaignError::campaign("identity_formula_override: the columns 'identity_key' and 'corrected_formula' are required"));
            };
            for r in 0..f.len() {
                formula.insert(f.cell(r, k).to_string(), f.cell(r, v).to_string());
            }
        }
        Ok(TableCuration {
            name_map,
            category_override,
            formula_override: formula,
        })
    }

    /// The tables named in `[curation]` of the campaign, read through `io`; a table that is not named or not there is empty.
    pub fn load(c: &Campaign, io: &Io) -> Result<TableCuration> {
        let text = |i: usize| c.curation_csv[i].as_ref().and_then(|p| io.text(p));
        Self::from_texts(text(0).as_deref(), text(1).as_deref(), text(2).as_deref())
    }

    /// Whether any table has a row.
    pub fn is_empty(&self) -> bool {
        self.name_map.is_empty()
            && self.category_override.is_empty()
            && self.formula_override.is_empty()
    }

    /// `(label, category)` of the first name that matches a row of the category override (a plain substring of the lowercase name).
    pub fn match_noncec_override(&self, name: Option<&str>) -> Option<(String, String)> {
        let low = name?.to_lowercase();
        self.category_override
            .iter()
            .find(|(k, _, _)| low.contains(k.as_str()))
            .map(|(_, l, c)| (l.clone(), c.clone()))
    }

    /// Static audit of the name map: duplicate patterns, patterns outside the dialect, and order conflicts (an EARLIER pattern that also matches the trigger
    /// of a LATER pattern mapping to another contaminant, which then never fires).
    pub fn audit_name_map(&self) -> NameMapAudit {
        let mut a = NameMapAudit::default();
        let mut seen: Vec<&str> = Vec::new();
        for (p, _, _) in &self.name_map {
            if seen.contains(&p.text()) {
                a.duplicates.push(p.text().to_string());
            }
            seen.push(p.text());
            if !p.is_valid() {
                a.invalid.push(p.text().to_string());
            }
        }
        for (j, (pj, nj, _)) in self.name_map.iter().enumerate() {
            let hay = pj.text().replace('\\', "").to_lowercase();
            for (pi, ni, _) in &self.name_map[..j] {
                if ni != nj && pi.search(&hay) {
                    a.shadows.push((
                        pi.text().to_string(),
                        ni.clone(),
                        pj.text().to_string(),
                        nj.clone(),
                    ));
                }
            }
        }
        a
    }
}

/// What `TableCuration::audit_name_map` finds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NameMapAudit {
    pub duplicates: Vec<String>,
    pub invalid: Vec<String>,
    /// `(earlier pattern, its contaminant, later pattern, its contaminant)`.
    pub shadows: Vec<(String, String, String, String)>,
}

impl Curation for TableCuration {
    fn match_cec(&self, names: &[Option<&str>]) -> Option<Cec> {
        for nm in names.iter().flatten() {
            let low = nm.to_lowercase();
            if let Some((_, l, c)) = self.name_map.iter().find(|(p, _, _)| p.search(&low)) {
                return Some((l.clone(), c.clone()));
            }
        }
        None
    }
    fn is_headline(&self, label: &str) -> bool {
        HEADLINE.contains(&label)
    }
    fn clean_library_name(&self, name: &str) -> String {
        clean_library_name(name)
    }
    fn curate_identity(
        &self,
        cec: Option<&Cec>,
        disp_name: Option<&str>,
        formula: Option<&str>,
    ) -> (String, String) {
        let found = cec
            .filter(|c| !c.0.trim().is_empty())
            .cloned()
            .or_else(|| self.match_cec(&[disp_name]));
        if let Some(c) = found.filter(|c| !c.0.trim().is_empty()) {
            return c;
        }
        if let Some(o) = self.match_noncec_override(disp_name) {
            return o;
        }
        if blank(disp_name) {
            return unnamed(formula);
        }
        let n = disp_name.unwrap_or_default();
        (n.to_string(), classify_noncec(Some(n)).to_string())
    }
    fn canonical_name(&self, identity: &str, formula: Option<&str>) -> String {
        names::canonical_name(identity, formula)
    }
    fn formula_override(&self, key: &str) -> Option<String> {
        self.formula_override.get(key).cloned()
    }
}

/// The placeholder identity of a feature without a name.
pub fn unnamed(formula: Option<&str>) -> (String, String) {
    match formula.filter(|f| !f.trim().is_empty()) {
        Some(f) => (
            format!("Unidentified (CSI:FingerID structure, formula {f})"),
            "ambiguous".into(),
        ),
        None => (
            "Unidentified (no name available)".into(),
            "ambiguous".into(),
        ),
    }
}
