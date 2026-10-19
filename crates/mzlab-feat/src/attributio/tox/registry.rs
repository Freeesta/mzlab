// Ported from Attributio chem/registry.py, chem/pubchem.py, chem/pubchemlite.py, chem/norman.py.
//! The pure rules of the compound registry: the representative CID of an InChIKey first block, CAS/DTXSID from PubChem synonyms, aquatic H-codes from a PUG-View
//! answer, PubChemLite categories, the predicted acute category of a NORMAN SusDat row and the registry summary. The downloads that feed them live outside.

use super::snapshot::RegistryRow;
use std::collections::BTreeSet;

/// PubChemLite annotation categories, in file order.
pub const PCL_CATEGORIES: [&str; 10] = [
    "AgroChemInfo",
    "BioPathway",
    "DrugMedicInfo",
    "FoodRelated",
    "PharmacoInfo",
    "SafetyInfo",
    "ToxicityInfo",
    "KnownUse",
    "DisorderDisease",
    "Identification",
];

/// SusDat predicted-toxicity columns (µg/L) and the endpoint they stand for.
pub const NORMAN_TOX_COLUMNS: [(&str, &str); 3] = [
    ("LC50_48_hr_ug/L", "Daphnia 48 h LC50"),
    ("EC50_72_hr_ug/L", "algae 72 h EC50"),
    ("LC50_96_hr_ug/L", "fish 96 h LC50"),
];

/// One representative CID per first block: among the candidates whose InChIKey starts with `block`, the one equal to `prefer_key`, else the neutral stereo-free
/// form (`-UHFFFAOYSA-N`), else any neutral form (`-N`), else the lowest CID. `props` are (CID, InChIKey) pairs.
pub fn representative_cid(
    block: &str,
    props: &[(i64, String)],
    prefer_key: Option<&str>,
) -> Option<i64> {
    let mut cand: Vec<&(i64, String)> =
        props.iter().filter(|(_, k)| k.starts_with(block)).collect();
    cand.sort_by_key(|(c, _)| *c);
    let tests: [&dyn Fn(&str) -> bool; 3] = [
        &|k| prefer_key.is_some_and(|p| !p.is_empty() && k == p),
        &|k| k.ends_with("-UHFFFAOYSA-N"),
        &|k| k.ends_with("-N"),
    ];
    for t in tests {
        if let Some((c, _)) = cand.iter().find(|(_, k)| t(k)) {
            return Some(*c);
        }
    }
    cand.first().map(|(c, _)| *c)
}

/// The first synonym that is a DTXSID (`DTXSID` + at least six digits), or empty.
pub fn dtxsid_from_synonyms(syn: &[&str]) -> String {
    syn.iter()
        .find(|s| {
            s.strip_prefix("DTXSID")
                .is_some_and(|d| d.len() >= 6 && d.bytes().all(|b| b.is_ascii_digit()))
        })
        .map_or(String::new(), |s| s.to_string())
}

/// The first synonym that looks like a CAS number (`\d{2,7}-\d{2}-\d`), or empty.
pub fn cas_from_synonyms(syn: &[&str]) -> String {
    let ok = |s: &str| {
        let p: Vec<&str> = s.split('-').collect();
        let digits = |t: &str, a: usize, b: usize| {
            (a..=b).contains(&t.len()) && t.bytes().all(|c| c.is_ascii_digit())
        };
        p.len() == 3 && digits(p[0], 2, 7) && digits(p[1], 2, 2) && digits(p[2], 1, 1)
    };
    syn.iter()
        .find(|s| ok(s))
        .map_or(String::new(), |s| s.to_string())
}

/// Aquatic H-codes (H400-H413) found in the `String` values of a PUG-View GHS answer (the caller collects them from the JSON), sorted
/// (`\bH4(00|01|02|10|11|12|13)\b`).
pub fn h_aquatic_codes(strings: &[&str]) -> Vec<String> {
    let mut codes = BTreeSet::new();
    for s in strings {
        let b = s.as_bytes();
        let word = |i: usize| {
            b.get(i)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        };
        for i in 0..b.len().saturating_sub(3) {
            if b[i] == b'H' && (i == 0 || !word(i - 1)) && !word(i + 4) && s.is_char_boundary(i + 4)
            {
                let code = &s[i..i + 4];
                if ["H400", "H401", "H402", "H410", "H411", "H412", "H413"].contains(&code) {
                    codes.insert(code.to_string());
                }
            }
        }
    }
    codes.into_iter().collect()
}

/// «AgroChemInfo|ToxicityInfo|…»: the PubChemLite categories whose count is > 0 (`counts` in `PCL_CATEGORIES` order; missing or non-numeric counts are 0).
pub fn pcl_categories(counts: &[Option<f64>; 10]) -> String {
    PCL_CATEGORIES
        .iter()
        .zip(counts)
        .filter(|(_, n)| n.is_some_and(|v| v > 0.0))
        .map(|(c, _)| *c)
        .collect::<Vec<_>>()
        .join("|")
}

/// (lowest predicted L(E)C50 in mg/L, endpoint) from the three SusDat columns in µg/L (`None` for a missing value); the first column wins a tie.
pub fn predicted_lowest_lc50_mg_l(ug_l: &[Option<f64>; 3]) -> (Option<f64>, &'static str) {
    let (mut best, mut which) = (None, "");
    for ((_, label), v) in NORMAN_TOX_COLUMNS.iter().zip(ug_l) {
        if let Some(v) = v.filter(|v| !v.is_nan()).map(|v| v / 1000.0) {
            if best.is_none_or(|b| v < b) {
                (best, which) = (Some(v), *label);
            }
        }
    }
    (best, which)
}

/// GHS acute aquatic category of the lowest predicted L(E)C50; empty without a value.
pub fn ghs_acute_category(lc50_mg_l: Option<f64>) -> &'static str {
    match lc50_mg_l {
        None => "",
        Some(v) if v <= 1.0 => "Acute 1",
        Some(v) if v <= 10.0 => "Acute 2",
        Some(v) if v <= 100.0 => "Acute 3",
        Some(_) => "Not classified",
    }
}

/// Counts of the registry (`registry.summary`, registry side): compounds, with CID, in PubChemLite, in SusDat, with DTXSID, with XLogP.
pub fn summary(reg: &[RegistryRow]) -> Vec<(&'static str, usize)> {
    let n = |f: &dyn Fn(&RegistryRow) -> bool| reg.iter().filter(|r| f(r)).count();
    vec![
        ("compounds (first blocks)", reg.len()),
        ("with PubChem CID", n(&|r| !r.pubchem_cid.is_empty())),
        ("in PubChemLite", n(&|r| r.in_pubchemlite == "True")),
        ("in NORMAN SusDat", n(&|r| !r.norman_ids.is_empty())),
        ("with DTXSID", n(&|r| !r.dtxsid.is_empty())),
        ("with XLogP", n(&|r| r.xlogp.is_some())),
    ]
}
