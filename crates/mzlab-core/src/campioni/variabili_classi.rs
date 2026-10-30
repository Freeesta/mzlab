//! Name tokeniser of `variabili`: level-1 tokens (separators), level-2 atoms inside a token (`MQH2O2T00`), the vocabularies the classes come from, and the
//! comparison helpers (case folding, natural order). Used by `variabili.rs` only.

use super::idcampione::{DARK, IRRADIATED, MATRICES};
use super::parole_bianco::is_blank_name;

/// One piece of a name: the text and its span in characters of the full name (folders included).
#[derive(Clone)]
pub(super) struct Tok {
    pub text: String,
    pub a: usize,
    pub b: usize,
    pub class: &'static str,
}

const ELEMENTS: &str = "H He Li Be B C N O F Ne Na Mg Al Si P S Cl Ar K Ca Sc Ti V Cr Mn Fe Co Ni Cu Zn Ga Ge As Se Br Kr Rb Sr Y Zr Nb Mo Tc Ru Rh Pd Ag Cd In Sn Sb Te I Xe Cs Ba La Ce Pr Nd Pm Sm Eu Gd Tb Dy Ho Er Tm Yb Lu Hf Ta W Re Os Ir Pt Au Hg Tl Pb Bi Po At Rn Fr Ra Ac Th Pa U Np Pu Am Cm Bk Cf Es Fm Md No Lr";

/// Words that name a role, with the digits after them left out (`std5`, `blk2`).
const ROLE_WORDS: [&str; 23] = [
    "blk",
    "blank",
    "bianco",
    "sb",
    "wash",
    "std",
    "standard",
    "cal",
    "calib",
    "calibr",
    "calibrazione",
    "taratura",
    "curva",
    "qc",
    "pool",
    "dark",
    "buio",
    "ctrl",
    "control",
    "infusion",
    "infusione",
    "fia",
    "adsorption",
];

/// Units written after a number: the text and the factor to mg/L (None: not a mass concentration).
pub(super) const UNITS: [(&str, Option<f64>); 14] = [
    ("ppm", Some(1.0)),
    ("ppb", Some(1e-3)),
    ("ppt", Some(1e-6)),
    ("mgl", Some(1.0)),
    ("mg/l", Some(1.0)),
    ("gl", Some(1000.0)),
    ("g/l", Some(1000.0)),
    ("ugl", Some(1e-3)),
    ("ug/l", Some(1e-3)),
    ("ngl", Some(1e-6)),
    ("ng/l", Some(1e-6)),
    ("ngml", Some(1e-3)),
    ("ng/ml", Some(1e-3)),
    ("mm", None),
];

/// Lower case with the micro sign as `u` and the digits after a letter kept (`T00` stays `t00`).
pub(super) fn fold(s: &str) -> String {
    s.chars().flat_map(fold_char).collect()
}

/// One character folded: `µ` is `u`, the rest lower case (may be several characters).
fn fold_char(c: char) -> impl Iterator<Item = char> {
    let c = if c == '\u{b5}' || c == '\u{3bc}' {
        'u'
    } else {
        c
    };
    c.to_lowercase()
}

pub(super) fn is_role_word(low: &str) -> bool {
    ROLE_WORDS.contains(&low.trim_end_matches(|c: char| c.is_ascii_digit()))
}

pub(super) fn is_cond_word(tok: &str) -> bool {
    // the lists are upper case ASCII: for an ASCII token the comparison needs no new string
    if tok.is_ascii() {
        return IRRADIATED
            .iter()
            .chain(&DARK)
            .any(|c| c.eq_ignore_ascii_case(tok));
    }
    let up = tok.to_uppercase();
    IRRADIATED.contains(&up.as_str()) || DARK.contains(&up.as_str())
}

pub(super) fn is_matrix_word(tok: &str) -> bool {
    MATRICES.iter().any(|m| m.eq_ignore_ascii_case(tok))
}

/// A blank word of `nomi-bianco.json` (`wash`, `mq`).
pub(super) fn is_blank_word(low: &str) -> bool {
    is_blank_name(low)
}

fn is_sep_default(s: &[char], i: usize) -> bool {
    match s[i] {
        '_' | '-' | ' ' | '+' | '(' | ')' | '[' | ']' | '\t' => true,
        // a decimal point or comma between digits belongs to the number
        '.' | ',' => {
            !(i > 0 && s[i - 1].is_ascii_digit() && s.get(i + 1).is_some_and(char::is_ascii_digit))
        }
        _ => false,
    }
}

/// The level-1 pieces of `s[from..to]` as `(start, end)`: cut at the separators (`seps` = the characters chosen by the user, None = the default set).
pub(super) fn split(
    s: &[char],
    from: usize,
    to: usize,
    seps: Option<&[char]>,
) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for i in from..to {
        let sep = match seps {
            Some(set) => set.contains(&s[i]),
            None => is_sep_default(s, i),
        };
        match (sep, start) {
            (true, Some(a)) => {
                out.push((a, i));
                start = None;
            }
            (false, None) => start = Some(i),
            _ => {}
        }
    }
    if let Some(a) = start {
        out.push((a, to));
    }
    out
}

fn symbol_len(t: &[char], i: usize) -> Option<usize> {
    let known = |n: usize| {
        let w = &t[i..i + n];
        ELEMENTS
            .split(' ')
            .any(|e| e.len() == n && e.chars().zip(w).all(|(x, y)| x == *y))
    };
    if !t[i].is_ascii_uppercase() {
        return None;
    }
    if i + 2 <= t.len() && t.get(i + 1).is_some_and(char::is_ascii_lowercase) && known(2) {
        return Some(2);
    }
    known(1).then_some(1)
}

/// End of a chemical-looking piece at `i`: element symbols with their counts (`H2O2`, `TiO2`, `Na2SO4`), at least one digit.
fn chem_end(t: &[char], i: usize) -> Option<usize> {
    let mut j = i;
    while j < t.len() {
        let Some(n) = symbol_len(t, j) else { break };
        j += n;
        while j < t.len() && t[j].is_ascii_digit() {
            j += 1;
        }
    }
    (j > i && t[i..j].iter().any(char::is_ascii_digit)).then_some(j)
}

/// End of a time atom at `i`: `T00`, `t2p5`, `T30min`.
fn time_end(t: &[char], i: usize) -> Option<usize> {
    if !matches!(t[i], 't' | 'T') {
        return None;
    }
    let mut j = i + 1;
    let d0 = j;
    while j < t.len() && t[j].is_ascii_digit() {
        j += 1;
    }
    if j == d0 {
        return None;
    }
    if j + 1 < t.len() && matches!(t[j], 'p' | '.') && t[j + 1].is_ascii_digit() {
        j += 1;
        while j < t.len() && t[j].is_ascii_digit() {
            j += 1;
        }
    }
    // the rest of the token is a unit, in any case
    let unit = ["min", "sec", "h", "d", "s"].into_iter().find(|u| {
        t.len() - j == u.len()
            && t[j..]
                .iter()
                .zip(u.bytes())
                .all(|(c, b)| c.to_ascii_lowercase() == char::from(b))
    });
    Some(j + unit.map_or(0, str::len))
}

/// The minutes of a token that is only a time (`T00`, `t2p5`, `T30min`), for the names the parser finds no time in (`MQH2O2T00`).
pub(super) fn time_token(tok: &str) -> Option<f64> {
    let t: Vec<char> = tok.chars().collect();
    if time_end(&t, 0)? != t.len() {
        return None;
    }
    let body: String = t[1..].iter().collect::<String>().to_lowercase();
    let num: String = body
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, 'p' | '.'))
        .map(|c| if c == 'p' { '.' } else { c })
        .collect();
    let unit = &body[num.len()..];
    let f = match unit {
        "s" | "sec" => 1.0 / 60.0,
        "h" => 60.0,
        "d" => 1440.0,
        _ => 1.0,
    };
    num.parse::<f64>().ok().map(|v| v * f)
}

/// End of a known word (matrix, condition, role) at `i`, the longest.
fn word_end(t: &[char], i: usize) -> Option<usize> {
    if t[i..].iter().all(char::is_ascii) {
        // ASCII: one lower-case copy of the first 12 characters, the candidates are slices of it
        let m = (t.len() - i).min(12);
        let tok: String = t[i..i + m].iter().collect();
        let low = tok.to_ascii_lowercase();
        return (2..=m)
            .rev()
            .find(|&n| {
                is_matrix_word(&tok[..n]) || is_cond_word(&tok[..n]) || is_role_word(&low[..n])
            })
            .map(|n| i + n);
    }
    let low: String = fold(&t[i..].iter().collect::<String>());
    let mut best = None;
    for n in (2..=low.chars().count().min(12)).rev() {
        let w: String = low.chars().take(n).collect();
        let tok: String = t[i..i + n].iter().collect();
        if is_matrix_word(&tok) || is_cond_word(&tok) || is_role_word(&w) {
            // a role word swallows the digits after it only as part of the time atom
            best = Some(i + n);
            break;
        }
    }
    best
}

/// The atoms of a token with no separator that mixes kinds (`MQH2O2T00` → `MQ` `H2O2` `T00`); None when the token stays whole.
fn atoms(t: &[char]) -> Option<Vec<(usize, usize)>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < t.len() {
        let e = chem_end(t, i)
            .or_else(|| word_end(t, i))
            .or_else(|| time_end(t, i))?;
        out.push((i, e));
        i = e;
    }
    (out.len() >= 2).then_some(out)
}

/// camelCase parts (`BtzMix` → `Btz` `Mix`), each of two letters at least; None when the token stays whole.
fn camel(t: &[char]) -> Option<Vec<(usize, usize)>> {
    if !t.iter().all(char::is_ascii_alphabetic) {
        return None;
    }
    let mut cuts = vec![0];
    for i in 1..t.len() {
        if t[i - 1].is_ascii_lowercase() && t[i].is_ascii_uppercase() {
            cuts.push(i);
        }
    }
    cuts.push(t.len());
    let parts: Vec<(usize, usize)> = cuts.windows(2).map(|w| (w[0], w[1])).collect();
    (parts.len() >= 2 && parts.iter().all(|p| p.1 - p.0 >= 2)).then_some(parts)
}

/// The tokens of the stem `s[from..to]`: level-1 pieces, each cut into atoms when it holds no separator but several kinds of characters.
pub(super) fn tokenize(s: &[char], from: usize, to: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (a, b) in split(s, from, to, None) {
        let piece = &s[a..b];
        let mixed =
            piece.iter().any(char::is_ascii_digit) && piece.iter().any(char::is_ascii_alphabetic);
        let parts = if mixed { atoms(piece) } else { camel(piece) };
        match parts {
            Some(p) => out.extend(p.into_iter().map(|(x, y)| (a + x, a + y))),
            None => out.push((a, b)),
        }
    }
    out
}

/// Natural order: digit runs by value, the rest without case; the plain order breaks the tie, so the order is total.
pub(super) fn natural(a: &str, b: &str) -> std::cmp::Ordering {
    let (x, y): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let (mut i, mut j) = (0, 0);
    while i < x.len() && j < y.len() {
        if x[i].is_ascii_digit() && y[j].is_ascii_digit() {
            let (si, sj) = (i, j);
            while i < x.len() && x[i].is_ascii_digit() {
                i += 1;
            }
            while j < y.len() && y[j].is_ascii_digit() {
                j += 1;
            }
            let (p, q) = (&x[si..i], &y[sj..j]);
            let zeros = |d: &[char]| d.iter().take_while(|c| **c == '0').count();
            let (p0, q0) = (&p[zeros(p)..], &q[zeros(q)..]);
            let o = p0.len().cmp(&q0.len()).then_with(|| p0.cmp(&q0));
            if o.is_ne() {
                return o;
            }
        } else {
            let o = fold_char(x[i]).cmp(fold_char(y[j]));
            if o.is_ne() {
                return o;
            }
            i += 1;
            j += 1;
        }
    }
    (x.len() - i).cmp(&(y.len() - j)).then_with(|| a.cmp(b))
}
