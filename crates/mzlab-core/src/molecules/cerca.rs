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
