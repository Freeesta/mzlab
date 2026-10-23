//! Text helpers: a string quoted for the log lines of the agent (`agent`) and the numbers of a sentence (`reading`).

use mzlab_core::json::Out;

pub fn quoted(s: &str) -> String {
    let mut o = Out::default();
    o.str(s);
    o.0
}

/// The numbers of a text as canonical strings (`0,95` and `0.95` meet); digits glued to a letter (`C18`, `H2O`) are part of a name and are skipped, and so
/// is the list marker at the start of a line.
pub fn numbers(text: &str) -> Vec<String> {
    let t = text.trim_start();
    let t = t
        .find(|c: char| !c.is_ascii_digit())
        .filter(|&i| i > 0 && t[i..].starts_with(['.', ')']) && t[i + 1..].starts_with(' '))
        .map_or(t, |i| &t[i + 1..]);
    let c: Vec<char> = t.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if !c[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < c.len() && c[i].is_ascii_digit() {
            i += 1;
        }
        if i + 1 < c.len() && matches!(c[i], '.' | ',') && c[i + 1].is_ascii_digit() {
            i += 1;
            while i < c.len() && c[i].is_ascii_digit() {
                i += 1;
            }
        }
        let glued = start > 0 && c[start - 1].is_alphabetic();
        if !glued {
            let raw: String = c[start..i].iter().collect::<String>().replace(',', ".");
            out.push(raw.parse::<f64>().map_or(raw, |x| x.to_string()));
        }
    }
    out
}
