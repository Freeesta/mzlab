//! The one search of the molecules: a number is a mass (within `ppm`), a valid formula is the same formula in Hill order and then also a text, anything else is
//! a text over the name and the synonyms (the ranking of the Tabui name box: equal, starts with, starts a word, contains).

use super::Molecola;
use crate::elements::{fmt, parse_formula};

/// Text score of `w` (lower case) in one name, as the Tabui box gave it: 0 equal, 1 starts with, 2 starts a word, 3 contains; None when absent.
fn score(name: &str, w: &str) -> Option<u8> {
    let i = name.find(w)?;
    Some(if name == w {
        0
    } else if i == 0 {
        1
    } else if name[..i]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_whitespace() || "-(,".contains(c))
    {
        2
    } else {
        3
    })
}

fn by_text(ms: &[Molecola], w: &str) -> Vec<(usize, u8)> {
    let mut hits: Vec<(usize, u8, usize)> = Vec::new();
    for (k, m) in ms.iter().enumerate() {
        let best = std::iter::once(&m.nome)
            .chain(&m.sinonimi)
            .filter_map(|n| score(&n.to_lowercase(), w))
            .min();
        if let Some(b) = best {
            hits.push((k, b, m.nome.encode_utf16().count()));
        }
    }
    crate::sortc::slice_by_key(&mut hits, |h| (h.1, h.2));
    hits.into_iter().map(|h| (h.0, h.1)).collect()
}

fn by_mass(ms: &[Molecola], q: f64, ppm: f64) -> Vec<(usize, f64)> {
    let mut hits: Vec<(usize, f64)> = ms
        .iter()
        .enumerate()
        .filter(|(_, m)| (q - m.massa).abs() <= m.massa * ppm * 1e-6)
        .map(|(k, m)| (k, (q - m.massa).abs() / m.massa * 1e6))
        .collect();
    crate::sortc::slice_by(&mut hits, |a, b| a.1.total_cmp(&b.1));
    hits
}

/// The Hill form of `q` when it reads as a formula: it needs a carbon or two elements, so that a short word is not taken for one.
fn formula_of(q: &str) -> Option<String> {
    let f = parse_formula(q).ok()?;
    (f.iter().any(|(e, _)| e == "C") || f.len() > 1).then(|| fmt(&f))
}

/// The indices (in `ms`) and scores of the best `limite` molecules for `q`: the error in ppm for a mass, 0 for the same formula, 0..3 for a text. Fewer than
/// two characters of text give nothing.
pub fn cerca(ms: &[Molecola], q: &str, limite: usize, ppm: f64) -> Vec<(usize, f64)> {
    let q = q.trim();
    let mut hits: Vec<(usize, f64)> = Vec::new();
    if let Some(mass) = q.parse::<f64>().ok().filter(|x| x.is_finite() && *x > 0.0) {
        hits = by_mass(ms, mass, ppm);
    } else {
        if let Some(h) = formula_of(q) {
            hits.extend(
                ms.iter()
                    .enumerate()
                    .filter(|(_, m)| m.formula == h)
                    .map(|(k, _)| (k, 0.0)),
            );
        }
        let w = q.to_lowercase();
        if w.chars().count() >= 2 {
            let seen: Vec<usize> = hits.iter().map(|h| h.0).collect();
            hits.extend(
                by_text(ms, &w)
                    .into_iter()
                    .filter(|h| !seen.contains(&h.0))
                    .map(|(k, s)| (k, f64::from(s))),
            );
        }
    }
    hits.truncate(limite);
    hits
}

/// True when `a` and `b` differ by at most one inserted, deleted or replaced character.
fn within_one(a: &[char], b: &[char]) -> bool {
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if long.len() - short.len() > 1 {
        return false;
    }
    let i = short.iter().zip(long).take_while(|(x, y)| x == y).count();
    if short.len() == long.len() {
        short[i + 1..] == long[i + 1..] || i == short.len()
    } else {
        short[i..] == long[i + 1..]
    }
}

/// Molecules whose name or synonym is one typing slip away from `w` (lower case), as a whole name (4) or as the start of one (5): "bentazan" finds "bentazone".
fn by_typo(ms: &[Molecola], w: &str) -> Vec<(usize, u8)> {
    let q: Vec<char> = w.chars().collect();
    let mut hits: Vec<(usize, u8, usize)> = Vec::new();
    for (k, m) in ms.iter().enumerate() {
        let best = std::iter::once(&m.nome)
            .chain(&m.sinonimi)
            .filter_map(|n| {
                let c: Vec<char> = n.to_lowercase().chars().collect();
                if within_one(&q, &c) {
                    return Some(4);
                }
                (q.len().saturating_sub(1)..=q.len() + 1)
                    .any(|l| l <= c.len() && l < c.len() && within_one(&q, &c[..l]))
                    .then_some(5)
            })
            .min();
        if let Some(b) = best {
            hits.push((k, b, m.nome.encode_utf16().count()));
        }
    }
    crate::sortc::slice_by_key(&mut hits, |h| (h.1, h.2));
    hits.into_iter().map(|h| (h.0, h.1)).collect()
}

/// Molecules whose InChIKey is `q` (27 characters) or starts with it when `q` is the 14-character first block, in any case.
fn by_inchikey(ms: &[Molecola], q: &str) -> Vec<usize> {
    let q = q.to_ascii_uppercase();
    let full = q.len() == 27 && q.as_bytes()[14] == b'-';
    if !(full || q.len() == 14) || !q.bytes().all(|b| b.is_ascii_uppercase() || b == b'-') {
        return Vec::new();
    }
    (0..ms.len())
        .filter(|&k| {
            ms[k]
                .inchikey
                .as_deref()
                .is_some_and(|i| if full { i == q } else { i.starts_with(&q) })
        })
        .collect()
}

/// `cerca` for a box that picks a molecule: the same hits first, then the molecules of an InChIKey (score 0), and, only when nothing was found, the names one
/// typing slip away (score 4 whole name, 5 start of a name, from four characters on).
pub fn cerca_ampia(ms: &[Molecola], q: &str, limite: usize, ppm: f64) -> Vec<(usize, f64)> {
    let q = q.trim();
    let mut hits = cerca(ms, q, usize::MAX, ppm);
    for k in by_inchikey(ms, q) {
        if !hits.iter().any(|h| h.0 == k) {
            hits.insert(0, (k, 0.0));
        }
    }
    let w = q.to_lowercase();
    if hits.is_empty() && w.chars().count() >= 4 && q.parse::<f64>().is_err() {
        hits = by_typo(ms, &w)
            .into_iter()
            .map(|(k, s)| (k, f64::from(s)))
            .collect();
    }
    hits.truncate(limite);
    hits
}
