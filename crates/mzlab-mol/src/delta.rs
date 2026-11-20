//! Formula changes written as signed element counts (`+O`, `-CH2`, `-Cl+OH`): parsed, added and printed in one canonical spelling, so the delta of a
//! chain of transformations can be computed. Only element symbols and counts; no charges.

use crate::codec::MolError;
use crate::sortm::SortM;

/// Net change of every element in element-name order, zero entries removed.
pub type Counts = Vec<(String, i64)>;

/// Adds `n` to the count of `el`, keeping the list sorted by element.
fn bump(counts: &mut Counts, el: String, n: i64) {
    match counts.binary_search_by(|e| e.0.cmp(&el)) {
        Ok(i) => counts[i].1 += n,
        Err(i) => counts.insert(i, (el, n)),
    }
}

/// Reads `+O`, `-H2+O`, `+NO-H`; an empty text is no change.
pub fn parse(text: &str) -> Result<Counts, MolError> {
    let bad = || MolError::Invalid(format!("delta `{text}`"));
    let mut out = Counts::new();
    let mut sign = 0i64;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '+' => sign = 1,
            '-' => sign = -1,
            c if c.is_ascii_uppercase() && sign != 0 => {
                let mut el = c.to_string();
                if let Some(&l) = chars.peek().filter(|l| l.is_ascii_lowercase()) {
                    el.push(l);
                    chars.next();
                }
                let mut digits = String::new();
                while let Some(&d) = chars.peek().filter(|d| d.is_ascii_digit()) {
                    digits.push(d);
                    chars.next();
                }
                let n: i64 = if digits.is_empty() {
                    1
                } else {
                    digits.parse().map_err(|_| bad())?
                };
                bump(&mut out, el, sign * n);
            }
            _ => return Err(bad()),
        }
    }
    out.retain(|e| e.1 != 0);
    Ok(out)
}

/// Gains first (`+O2`), then losses (`-H2`), each in Hill order (C, H, then alphabetical); empty for no net change.
pub fn print(counts: &Counts) -> String {
    let hill = |el: &str| (el != "C", el != "H", el.to_string());
    let part = |gain: bool| {
        let mut els: Vec<(&String, i64)> = counts
            .iter()
            .filter(|&&(_, n)| (n > 0) == gain && n != 0)
            .map(|(e, n)| (e, n.abs()))
            .collect();
        els.sortm_by_key(|(e, _)| hill(e));
        let body: String = els
            .iter()
            .map(|(e, n)| {
                if *n == 1 {
                    (*e).clone()
                } else {
                    format!("{e}{n}")
                }
            })
            .collect();
        if body.is_empty() {
            body
        } else {
            format!("{}{body}", if gain { '+' } else { '-' })
        }
    };
    format!("{}{}", part(true), part(false))
}

/// The sum of two deltas in the canonical spelling.
pub fn add(a: &str, b: &str) -> Result<String, MolError> {
    let mut sum = parse(a)?;
    for (el, n) in parse(b)? {
        bump(&mut sum, el, n);
    }
    sum.retain(|e| e.1 != 0);
    Ok(print(&sum))
}
