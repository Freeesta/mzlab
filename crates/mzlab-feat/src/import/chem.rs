//! The chemistry the readers use: the formula of a library entry must explain the precursor of the entry (`formulaMass`, adducts, `checkStructure`),
//! and the names of the library entries are cleaned with the profile «attributio» of `livelli/1` (`cleanName`).

use super::delim::{jstrim, txt};
use mzlab_core::elements::{mass_of, ELECTRON};
use mzlab_core::json::{self, Value};
use std::sync::OnceLock;

/// The elements of the old table that `mzlab-core` does not have.
fn mono(el: &str) -> Option<f64> {
    match el {
        "B" => Some(11.0093054),
        "Fe" => Some(55.9349375),
        _ => mass_of(el),
    }
}

/// Monoisotopic mass of a formula written as plain element counts («C10H17NO2»); `None` for anything else.
pub fn formula_mass(f: &str) -> Option<f64> {
    let b = txt(f).as_bytes();
    let (mut i, mut m) = (0, 0.0);
    if b.is_empty() {
        return None;
    }
    while i < b.len() {
        if !b[i].is_ascii_uppercase() {
            return None;
        }
        let mut j = i + 1;
        if j < b.len() && b[j].is_ascii_lowercase() {
            j += 1;
        }
        let el = std::str::from_utf8(&b[i..j]).ok()?;
        let k = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        let n: f64 = if j > k {
            std::str::from_utf8(&b[k..j]).ok()?.parse().ok()?
        } else {
            1.0
        };
        m += mono(el)? * n;
        i = j;
    }
    Some(m)
}

fn proton() -> f64 {
    mass_of("H").unwrap() - ELECTRON
}

/// m/z of the ion = M + delta (single charge), for the adducts the check knows.
pub fn adduct_delta(a: &str) -> Option<f64> {
    let (h, o, c) = (mass_of("H")?, mass_of("O")?, mass_of("C")?);
    Some(match a {
        "[M+H]+" => proton(),
        "[M+Na]+" => mass_of("Na")? - ELECTRON,
        "[M+K]+" => mass_of("K")? - ELECTRON,
        "[M+NH4]+" => mass_of("N")? + 4.0 * h - ELECTRON,
        "[M-H2O+H]+" => proton() - (2.0 * h + o),
        "[M-H]-" => -proton(),
        "[M+Cl]-" => mass_of("Cl")? + ELECTRON,
        "[M+HCOO]-" => c + h + 2.0 * o + ELECTRON,
        "[M+CH3COO]-" => 2.0 * c + 3.0 * h + 2.0 * o + ELECTRON,
        "[M-H2O-H]-" => -proton() - (2.0 * h + o),
        _ => return None,
    })
}

const COMMON_POS: [&str; 5] = ["[M+H]+", "[M+Na]+", "[M+NH4]+", "[M+K]+", "[M-H2O+H]+"];
const COMMON_NEG: [&str; 5] = [
    "[M-H]-",
    "[M+Cl]-",
    "[M+HCOO]-",
    "[M+CH3COO]-",
    "[M-H2O-H]-",
];

/// «M+H» → «[M+H]+»; an unknown adduct stays as written.
pub fn norm_adduct(a: &str) -> String {
    let s: String = txt(a).chars().filter(|c| !c.is_whitespace()).collect();
    if s.is_empty() || s.starts_with('[') {
        return s;
    }
    let neg = s.ends_with('-')
        || (s.starts_with("M-H") && !s.starts_with("M-H2O"))
        || ["M+Cl", "M+HCOO", "M+CH3COO"]
            .iter()
            .any(|p| s.starts_with(p));
    let core = s.strip_suffix(['+', '-']).unwrap_or(&s);
    format!("[{core}]{}", if neg { "-" } else { "+" })
}

pub fn ppm_of(a: f64, b: f64) -> Option<f64> {
    (b != 0.0).then(|| (a - b) / b * 1e6)
}

/// The answer of `check_structure`: `ok` is `None` when there is nothing to check.
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub ok: Option<bool>,
    pub ppm: Option<f64>,
    pub adduct: Option<String>,
}

/// Does the library structure of a match agree with the precursor of its entry (declared adduct first, then the common ones of the polarity)?
pub fn check_structure(
    formula: &str,
    precursor: Option<f64>,
    adduct: &str,
    pol: i32,
    tol_ppm: f64,
) -> Check {
    let none = Check {
        ok: None,
        ppm: None,
        adduct: None,
    };
    let (Some(m), Some(p)) = (formula_mass(formula), precursor.filter(|p| *p > 0.0)) else {
        return none;
    };
    let decl = norm_adduct(adduct);
    let common: &[&str] = if pol < 0 { &COMMON_NEG } else { &COMMON_POS };
    let mut list: Vec<&str> = vec![];
    if !decl.is_empty() && adduct_delta(&decl).is_some() {
        list.push(&decl);
    }
    list.extend(common.iter().copied().filter(|a| *a != decl));
    let mut best: Option<(f64, &str)> = None;
    for a in list {
        let ppm = ppm_of(m + adduct_delta(a).unwrap_or(0.0), p).unwrap_or(0.0);
        if best.is_none_or(|(b, _)| ppm.abs() < b.abs()) {
            best = Some((ppm, a));
        }
        if a == decl && ppm.abs() <= tol_ppm {
            return Check {
                ok: Some(true),
                ppm: Some(ppm),
                adduct: Some(a.into()),
            };
        }
    }
    match best {
        Some((ppm, a)) => Check {
            ok: Some(ppm.abs() <= tol_ppm),
            ppm: Some(ppm),
            adduct: Some(a.into()),
        },
        None => Check {
            ok: Some(false),
            ppm: None,
            adduct: None,
        },
    }
}

/// GNPS2 propagated names («Candidate …», «(known structural isomers», «(delta mass:»): never a source.
pub fn is_propagated(n: &str) -> bool {
    let low = n.to_lowercase();
    let cand = txt(&low).strip_prefix("candidate").is_some_and(|r| {
        !r.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    });
    cand || low.contains("(known structural isomers") || low.contains("(delta mass:")
}

struct Cleaning {
    prefixes: Vec<String>,
    suffixes: Vec<String>,
}

/// The cleaning rules of the profile «attributio» (`pulizia_nomi` of the bundled `livelli/1`).
fn cleaning() -> &'static Cleaning {
    static C: OnceLock<Cleaning> = OnceLock::new();
    C.get_or_init(|| {
        let list = |v: Option<&Value>| {
            v.map(|v| {
                v.arr()
                    .iter()
                    .filter_map(|x| x.str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
        };
        let v = json::parse(crate::evid::points::BUNDLED.as_bytes()).ok();
        let p = v
            .as_ref()
            .and_then(|v| v.get("profili"))
            .and_then(|p| p.get("attributio"))
            .and_then(|p| p.get("pulizia_nomi"));
        Cleaning {
            prefixes: list(p.and_then(|p| p.get("prefissi"))),
            suffixes: list(p.and_then(|p| p.get("suffissi"))),
        }
    })
}

/// `^(?:[A-Z]{2,3}\d{5,}|\d{5,})\s+`: the length of a leading accession number with the white space after it.
fn accession_len(s: &str) -> usize {
    let b = s.as_bytes();
    let letters = b.iter().take_while(|c| c.is_ascii_uppercase()).count();
    let start = if (2..=3).contains(&letters) {
        letters
    } else {
        0
    };
    let digits = b[start..].iter().take_while(|c| c.is_ascii_digit()).count();
    if digits < 5 {
        return 0;
    }
    let ws = s[start + digits..]
        .chars()
        .take_while(|c| c.is_whitespace() || *c == '\u{feff}')
        .map(char::len_utf8)
        .sum::<usize>();
    if ws == 0 {
        0
    } else {
        start + digits + ws
    }
}

/// `_CE\d+(?:\.\d+)?$`: where a trailing collision energy starts.
fn energy_start(s: &str) -> Option<usize> {
    s.match_indices("_CE").map(|(i, _)| i).find(|&i| {
        let r = &s.as_bytes()[i + 3..];
        let d = r.iter().take_while(|c| c.is_ascii_digit()).count();
        let rest = &r[d..];
        d > 0
            && (rest.is_empty()
                || (rest[0] == b'.'
                    && rest.len() > 1
                    && rest[1..].iter().all(|c| c.is_ascii_digit())))
    })
}

/// The name of a library entry without prefix, «|…», suffix, leading accession and trailing energy.
pub fn clean_name(name: &str) -> String {
    let mut s = txt(name).to_string();
    if s.is_empty() {
        return s;
    }
    let c = cleaning();
    let low = s.to_lowercase();
    if let Some(p) = c
        .prefixes
        .iter()
        .find(|p| low.starts_with(&p.to_lowercase()))
    {
        s = s.chars().skip(p.chars().count()).collect();
    }
    if let Some(i) = s.find('|') {
        s.truncate(i);
    }
    let mut s = jstrim(&s).to_string();
    let low = s.to_lowercase();
    if let Some(x) = c.suffixes.iter().find(|x| low.ends_with(&x.to_lowercase())) {
        let keep = s.chars().count().saturating_sub(x.chars().count());
        s = s.chars().take(keep).collect();
    }
    s.drain(..accession_len(&s));
    if let Some(i) = energy_start(&s) {
        s.truncate(i);
    }
    jstrim(&s).to_string()
}
