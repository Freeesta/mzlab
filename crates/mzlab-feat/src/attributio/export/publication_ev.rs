// Ported from Attributio diagnostics/publication_table.py.
//! The evidence, the ion and the structure cells of a row of the publication table.

use super::publication_ids::{Dd, PFeat};
use crate::attributio::annotate::Feature;
use crate::attributio::curation::cec_category_label;
use crate::attributio::formula::{
    best_common_adduct, common_adducts, formula_mass, ion_mz, neutral_equivalent, parse_adduct, ppm,
};
use crate::attributio::frame::Cell;
use crate::attributio::readers::standards::py_round;
use crate::sortc::SortC;
use std::collections::BTreeSet;

pub(super) const LEVEL_LABEL: [&str; 3] = ["1", "2a", "3"];
pub(super) const SOURCE_LABEL: [(&str, &str); 6] = [
    ("reference-std", "reference standard"),
    ("SIRIUS-spectral", "SIRIUS spectral match"),
    ("GNPS2", "GNPS2"),
    ("mzMine-library", "MZmine library"),
    ("CSI:FingerID", "CSI:FingerID"),
    ("reference-RT", "reference standard (mass + RT only)"),
];
/// Contaminant classes in the wording of a publication (keys: curation category).
pub(super) const CLASS_LABEL: [(&str, &str); 10] = [
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
pub(super) const LIBRARY_L3: [&str; 4] = [
    "name conflict",
    "expert review",
    "isomers not discriminated",
    "RT isomer",
];
pub(super) const STD_COLUMNS: [(&str, &str); 8] = [
    ("mz_MH_pos", "[M+H]+"),
    ("mz_MNa_pos", "[M+Na]+"),
    ("mz_MNH4_pos", "[M+NH4]+"),
    ("mz_MK_pos", "[M+K]+"),
    ("mz_MH_neg", "[M-H]-"),
    ("mz_MHCOO_neg", "[M+HCOO]-"),
    ("mz_MCl_neg", "[M+Cl]-"),
    ("mz_M2H_pos2", "[M+2H]2+"),
];

pub(super) fn text(s: impl Into<String>) -> Cell {
    Cell::Str(s.into())
}

pub(super) fn rounded(v: Option<f64>, n: usize) -> Cell {
    Cell::num(v.filter(|x| !x.is_nan()).map(|x| py_round(x, n)))
}

pub(super) fn yes_no(b: bool) -> Cell {
    text(if b { "yes" } else { "no" })
}

pub(super) fn int_of(v: Option<f64>) -> Cell {
    v.filter(|x| x.is_finite())
        .map_or(Cell::Null, |x| Cell::Int(x as i64))
}

// ── the evidence of one row ──────────────────────────────────────────────

/// `mzspec:` USI of the mzMine `usi` field (a JSON-like list or a plain text): the first one, else the text itself.
pub(super) fn first_usi(v: &str) -> Option<String> {
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
pub(super) fn spec_reference(v: Option<&str>) -> (Option<String>, Option<String>) {
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
pub(super) struct SrcEv {
    pub(super) lib: Option<String>,
    pub(super) reference: Option<String>,
    pub(super) cos: Option<f64>,
    pub(super) ions: Option<f64>,
}

pub(super) fn source_evidence(p: &PFeat, src: &str) -> SrcEv {
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

pub(super) fn has_source(f: &Feature, s: &str) -> bool {
    f.sources.contains(&s)
}

/// The score that decided the level of a feature, for choosing the representative.
pub(super) fn score(p: &PFeat) -> f64 {
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

pub(super) fn largest(p: &PFeat) -> f64 {
    p.max_area
        .iter()
        .copied()
        .filter(|a| !a.is_nan())
        .fold(f64::NEG_INFINITY, f64::max)
}

/// The best-evidence feature at the best level of the row (score, then largest area; the first of equals).
pub(super) fn representative<'a, 'b>(sub: &'b [&'a PFeat<'a>]) -> &'b PFeat<'a> {
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
pub(super) fn best_source(p: &PFeat) -> Option<&'static str> {
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
pub(super) fn adduct_in_name(name: &str) -> Option<String> {
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
pub(super) fn reported_adduct(p: &PFeat, lv: i32) -> Option<String> {
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
pub(super) fn common_adduct(adduct: &str, pol: &str) -> Option<String> {
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
pub(super) struct IonFit {
    pub(super) adduct: Option<String>,
    pub(super) err: Option<f64>,
    pub(super) mono: Option<f64>,
    pub(super) inferred: bool,
}

pub(super) fn ion_fit(rep: &PFeat, lv: i32, formula: Option<&str>) -> IonFit {
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
pub(super) fn structure(rep: &PFeat) -> (Cell, Cell, Cell) {
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

pub(super) fn evidence_cells(best_feats: &[&PFeat], rep: &PFeat, lv: i32) -> Vec<Cell> {
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

pub(super) fn std_cells(rep: &PFeat, lv: i32) -> Vec<Cell> {
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

pub(super) fn feature_ids(sub: &[&PFeat]) -> String {
    let mut ids: Vec<(bool, i64, &str)> = sub
        .iter()
        .map(|p| (p.f().pol != "POS", p.f().id, p.f().pol.as_str()))
        .collect();
    ids.sortc();
    ids.iter()
        .map(|(_, i, p)| format!("{p}:{i}"))
        .collect::<Vec<_>>()
        .join("; ")
}

pub(super) fn rt_range(sub: &[&PFeat]) -> String {
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

pub(super) fn class_label(it: &Dd) -> String {
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

/// The first cells of a row: the placeholder of the number, the name, the level, the anthropogenic flag and the class.
pub(super) fn head_cells(it: &Dd, lv: i32, lv_sub: &str) -> Vec<Cell> {
    vec![
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
    ]
}
