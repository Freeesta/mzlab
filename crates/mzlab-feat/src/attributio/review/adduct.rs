// Ported from Attributio src/attributio/chem/formula.py (MASS, formula_mass, parse_adduct) and review/libraries.py (adduct_key).
//
// Only what the library index needs: a comparable form of an adduct string.

pub const ELECTRON: f64 = 0.000548579909;

const MASS: [(&str, f64); 36] = [
    ("H", 1.00782503207),
    ("D", 2.0141017778),
    ("C", 12.0),
    ("N", 14.0030740048),
    ("O", 15.99491461956),
    ("F", 18.99840322),
    ("Na", 22.9897692809),
    ("Mg", 23.985041700),
    ("Al", 26.98153863),
    ("Si", 27.9769265325),
    ("P", 30.97376163),
    ("S", 31.97207100),
    ("Cl", 34.96885268),
    ("K", 38.96370668),
    ("Ca", 39.96259098),
    ("Fe", 55.9349375),
    ("Cu", 62.9295975),
    ("Zn", 63.9291422),
    ("As", 74.9215965),
    ("Se", 79.9165213),
    ("Br", 78.9183371),
    ("Sn", 119.9021947),
    ("I", 126.904473),
    ("Hg", 201.970643),
    ("B", 11.0093054),
    ("Li", 7.01600455),
    ("Co", 58.933195),
    ("Ni", 57.9353429),
    ("Mn", 54.9380451),
    ("Cr", 51.9405075),
    ("Pt", 194.9647911),
    ("Gd", 157.9241039),
    ("Ti", 47.9479463),
    ("Sb", 120.9038157),
    ("Bi", 208.9803987),
    ("Pb", 207.9766521),
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

const ALIASES: [(&str, &str); 9] = [
    ("HCOO", "CHO2"),
    ("CHOO", "CHO2"),
    ("FA", "CHO2"),
    ("ACN", "C2H3N"),
    ("AcO", "C2H3O2"),
    ("CH3COO", "C2H3O2"),
    ("NH4", "NH4"),
    ("Hac", "C2H4O2"),
    ("HAc", "C2H4O2"),
];

/// The monoisotopic mass of an element of the table of Attributio (36 elements); the formulas of `formula.rs` read it too.
pub fn mass_of(el: &str) -> Option<f64> {
    MASS.iter().find(|(e, _)| *e == el).map(|(_, m)| *m)
}

/// Element tokens `[A-Z][a-z]?\d*` that cover the whole text, or `None`.
fn tokens(body: &str) -> Option<Vec<(String, i64)>> {
    let b = body.as_bytes();
    let (mut i, mut out) = (0, Vec::new());
    while i < b.len() {
        if !b[i].is_ascii_uppercase() {
            return None;
        }
        let mut j = i + 1;
        if j < b.len() && b[j].is_ascii_lowercase() {
            j += 1;
        }
        let k = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        let n = if j > k { body[k..j].parse().ok()? } else { 1 };
        out.push((body[i..k].to_string(), n));
        i = j;
    }
    Some(out)
}

/// Monoisotopic neutral mass of a formula (`C10H12N2O`, `C9H8O4.Na`); `None` for an unknown element or an empty text.
pub fn formula_mass(formula: &str) -> Option<f64> {
    let f = formula.trim().replace(' ', "");
    let f = f
        .trim_start_matches('[')
        .trim_end_matches(['+', '-'])
        .trim_end_matches(']')
        .to_string();
    if f.is_empty() {
        return None;
    }
    let mut tot = 0.0;
    for part in f.split('.') {
        let d = part.bytes().take_while(|c| c.is_ascii_digit()).count();
        let mult: f64 = if d > 0 { part[..d].parse().ok()? } else { 1.0 };
        let body = &part[d..];
        if body.is_empty() {
            return None;
        }
        let mut s = 0.0;
        for (el, n) in tokens(body)? {
            s += mass_of(&el)? * n as f64;
        }
        tot += mult * s;
    }
    Some(tot)
}

fn term_mass(term: &str) -> Option<f64> {
    if term == "e" {
        return Some(ELECTRON);
    }
    let d = term.bytes().take_while(|c| c.is_ascii_digit()).count();
    let (k, body) = if d > 0 && d < term.len() {
        (term[..d].parse::<f64>().ok()?, &term[d..])
    } else {
        (1.0, term)
    };
    let body = ALIASES
        .iter()
        .find(|(a, _)| *a == body)
        .map_or(body, |(_, v)| v);
    formula_mass(body).map(|m| k * m)
}

/// `(n molecules, mass added to n*M, charge with sign)` of an adduct, or `None` if unreadable. Electrons are accounted for.
pub fn parse_adduct(adduct: &str) -> Option<(i64, f64, i64)> {
    let a = adduct.trim();
    if a.is_empty() {
        return None;
    }
    let a: String = STD_COLUMNS
        .iter()
        .find(|(k, _)| *k == a)
        .map_or(a, |(_, v)| v)
        .replace(' ', "");
    let b = a.as_bytes();
    let mut i = usize::from(b.first() == Some(&b'['));
    let d0 = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let n: i64 = if i > d0 { a[d0..i].parse().ok()? } else { 1 };
    if b.get(i) != Some(&b'M') {
        return None;
    }
    i += 1;
    let mut terms: Vec<(u8, &str)> = Vec::new();
    while i + 1 < b.len() && (b[i] == b'+' || b[i] == b'-') && b[i + 1].is_ascii_alphanumeric() {
        let s = i + 1;
        i = s;
        while i < b.len() && b[i].is_ascii_alphanumeric() {
            i += 1;
        }
        terms.push((b[s - 1], &a[s..i]));
    }
    if b.get(i) == Some(&b']') {
        i += 1;
    }
    let z0 = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let z: i64 = if i > z0 { a[z0..i].parse().ok()? } else { 1 };
    let sign: i64 = match (b.get(i), i + 1 == b.len()) {
        (Some(b'+'), true) => 1,
        (Some(b'-'), true) => -1,
        _ => return None,
    };
    let mut delta = 0.0;
    for (s, t) in &terms {
        let m = term_mass(t)?;
        delta += if *s == b'+' { m } else { -m };
    }
    if !terms.iter().any(|(_, t)| *t == "e") {
        delta -= (sign * z) as f64 * ELECTRON;
    }
    Some((n, delta, sign * z))
}

/// A comparable form of an adduct string (`[M+H]+`, `M+H`, `[M + H]+`), or `None`: `(n, delta rounded to 3 decimals, charge)`.
pub fn adduct_key(adduct: &str) -> Option<(i64, f64, i64)> {
    let s = adduct.trim();
    if s.is_empty() {
        return None;
    }
    let s = if s.starts_with('[') {
        s.to_string()
    } else {
        format!(
            "[{}]{}",
            s.trim_end_matches(['+', '-']),
            if s.ends_with('-') { "-" } else { "+" }
        )
    };
    let (n, delta, charge) = parse_adduct(&s)?;
    Some((n, (delta * 1000.0).round_ties_even() / 1000.0, charge))
}
