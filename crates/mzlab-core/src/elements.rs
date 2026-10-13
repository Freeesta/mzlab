//! Monoisotopic masses, formulas and adducts: the rules of `mzlab/chem/elements.py` (same masses, same reading of lower case, same
//! rounding). Errors carry the i18n keys that Python's `UserError` carries.

use crate::error::{Error, Result};
use crate::json::{round, Out};

pub const MASS: [(&str, f64); 14] = [
    ("H", 1.00782503223),
    ("C", 12.0),
    ("N", 14.00307400443),
    ("O", 15.99491461957),
    ("F", 18.99840316273),
    ("P", 30.97376199842),
    ("S", 31.9720711744),
    ("Cl", 34.968852682),
    ("Br", 78.9183376),
    ("I", 126.9044719),
    ("Na", 22.989769282),
    ("K", 38.9637064),
    ("Si", 27.9769265350),
    ("Se", 79.9165218),
];
pub const ELECTRON: f64 = 0.000548579909;
const COMMON2: [&str; 16] = [
    "Cl", "Br", "Si", "Na", "Se", "Li", "Mg", "Ca", "Al", "Fe", "Zn", "Cu", "Mn", "As", "Hg", "Cd",
];

pub fn mass_of(el: &str) -> Option<f64> {
    MASS.iter().find(|(e, _)| *e == el).map(|(_, m)| *m)
}

/// Ion m/z = neutral mass + shift; the order is the one of `ADDUCT_SHIFT`.
pub fn adducts() -> [(&'static str, f64); 7] {
    let (h, n, o, c) = (
        mass_of("H").unwrap(),
        mass_of("N").unwrap(),
        mass_of("O").unwrap(),
        mass_of("C").unwrap(),
    );
    [
        ("[M+H]+", h - ELECTRON),
        ("[M+NH4]+", n + 4.0 * h - ELECTRON),
        ("[M+Na]+", mass_of("Na").unwrap() - ELECTRON),
        ("[M+K]+", mass_of("K").unwrap() - ELECTRON),
        ("[M-H]-", -h + ELECTRON),
        ("[M+Cl]-", mass_of("Cl").unwrap() + ELECTRON),
        ("[M+HCOO]-", c + h + 2.0 * o + ELECTRON),
    ]
}

/// A parsed formula: element and count, in the order they were first written (the sums run in that order, as Python's dict does).
pub type Formula = Vec<(String, u64)>;

fn add(f: &mut Formula, el: &str, n: u64) {
    match f.iter_mut().find(|(e, _)| e == el) {
        Some(x) => x.1 += n,
        None => f.push((el.to_string(), n)),
    }
}

/// Fix the capitals of a formula typed in lower case ('c9h10cl2n2o' -> 'C9H10Cl2N2O').
pub fn normalize_formula(s: &str) -> String {
    let c: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        let (ch, nx) = (c[i], c.get(i + 1).copied());
        let pair = |a: char, b: char| format!("{a}{b}");
        if !ch.is_alphabetic() {
            out.push(ch);
            i += 1;
        } else if let (true, Some(n)) = (ch.is_uppercase(), nx) {
            if n.is_lowercase() && mass_of(&pair(ch, n)).is_some() {
                out.push(ch);
                out.push(n);
                i += 2;
            } else {
                out.push(ch);
                i += 1;
            }
        } else if let (true, Some(n)) = (ch.is_lowercase(), nx) {
            let up: String = ch.to_uppercase().collect();
            if n.is_lowercase() && COMMON2.contains(&format!("{up}{n}").as_str()) {
                out.push_str(&up);
                out.push(n);
                i += 2;
            } else {
                out.push_str(&up);
                i += 1;
            }
        } else {
            out.extend(ch.to_uppercase());
            i += 1;
        }
    }
    out
}

fn num_at(c: &[char], i: &mut usize) -> Option<u64> {
    let a = *i;
    while *i < c.len() && c[*i].is_ascii_digit() {
        *i += 1;
    }
    (*i > a).then(|| {
        c[a..*i]
            .iter()
            .collect::<String>()
            .parse()
            .unwrap_or(u64::MAX / 4)
    })
}

/// 'C15H12N2O' -> [("C",15),("H",12),("N",2),("O",1)]. Parentheses are allowed: 'C(OH)2'.
pub fn parse_formula(s: &str) -> Result<Formula> {
    let s = normalize_formula(s);
    if s.is_empty() {
        return Err(Error::new("err.formula.empty"));
    }
    let c: Vec<char> = s.chars().collect();
    let unreadable = || Error::new("err.formula.unreadable").with("formula", &s);
    let mut stack: Vec<Formula> = vec![vec![]];
    let mut i = 0;
    while i < c.len() {
        if c[i].is_ascii_uppercase() {
            let mut el = c[i].to_string();
            i += 1;
            if i < c.len() && c[i].is_ascii_lowercase() {
                el.push(c[i]);
                i += 1;
            }
            let n = num_at(&c, &mut i).unwrap_or(1);
            if mass_of(&el).is_none() {
                return Err(Error::new("err.formula.element")
                    .with("element", el)
                    .with("formula", &s));
            }
            add(stack.last_mut().unwrap(), &el, n);
        } else if c[i] == '(' {
            stack.push(vec![]);
            i += 1;
        } else if c[i] == ')' {
            i += 1;
            let k = num_at(&c, &mut i).unwrap_or(1);
            if stack.len() < 2 {
                return Err(Error::new("err.formula.parens").with("formula", &s));
            }
            for (el, n) in stack.pop().unwrap() {
                add(stack.last_mut().unwrap(), &el, n * k);
            }
        } else {
            return Err(unreadable());
        }
    }
    if stack.len() != 1 || stack[0].is_empty() {
        return Err(unreadable());
    }
    Ok(stack.pop().unwrap())
}

/// Round half up (not banker's rounding): 364.05 -> 364.1, 200.5 -> 201. Works on the decimal text of `round(x, 9)`, like `Decimal(repr(...))`.
pub fn round_half_up(x: f64, digits: usize) -> f64 {
    let r = round(x, 9);
    let neg = r < 0.0;
    let s = format!("{}", r.abs());
    let (ip, fp) = s.split_once('.').unwrap_or((&s, ""));
    let mut d: Vec<u8> = ip
        .bytes()
        .chain(fp.bytes().chain(std::iter::repeat(b'0')).take(digits + 1))
        .map(|b| b - b'0')
        .collect();
    let keep = ip.len() + digits;
    let up = d.get(keep).copied().unwrap_or(0) >= 5;
    d.truncate(keep);
    if up {
        let mut i = d.len();
        loop {
            if i == 0 {
                d.insert(0, 1);
                break;
            }
            i -= 1;
            if d[i] == 9 {
                d[i] = 0;
            } else {
                d[i] += 1;
                break;
            }
        }
    }
    let int_len = d.len() - digits;
    let t: String = d[..int_len]
        .iter()
        .map(|v| (v + b'0') as char)
        .collect::<String>()
        + "."
        + &d[int_len..]
            .iter()
            .map(|v| (v + b'0') as char)
            .collect::<String>();
    let v: f64 = t.parse().unwrap_or(0.0);
    if neg {
        -v
    } else {
        v
    }
}

pub fn mass(f: &Formula) -> f64 {
    crate::pysum::pysum(f.iter().map(|(e, n)| mass_of(e).unwrap_or(0.0) * *n as f64))
}

/// Hill order: C, H, then alphabetical.
pub fn fmt(f: &Formula) -> String {
    let get = |e: &str| f.iter().find(|(x, _)| x == e).map(|(_, n)| *n);
    let mut order: Vec<&str> = ["C", "H"]
        .into_iter()
        .filter(|e| get(e).is_some())
        .collect();
    let mut rest: Vec<&str> = f
        .iter()
        .map(|(e, _)| e.as_str())
        .filter(|e| *e != "C" && *e != "H")
        .collect();
    crate::sortc::ord(&mut rest);
    order.extend(rest);
    order
        .into_iter()
        .filter_map(|e| {
            get(e).filter(|n| *n != 0).map(|n| {
                if n != 1 {
                    format!("{e}{n}")
                } else {
                    e.to_string()
                }
            })
        })
        .collect()
}

pub fn ion_mz(neutral: f64, adduct: &str) -> Option<f64> {
    adducts()
        .iter()
        .find(|(a, _)| *a == adduct)
        .map(|(_, s)| neutral + s)
}

/// `/api/formula`: neutral mass and ion m/z of every adduct, exact, at 1 decimal and nominal.
pub fn formula_mz(formula: &str, adduct: Option<&str>) -> Result<String> {
    let f = parse_formula(formula)?;
    let m = mass(&f);
    let mut o = Out::default();
    o.raw("{\"formula\":")
        .str(&fmt(&f))
        .raw(",\"neutral\":")
        .num(round(m, 4))
        .raw(",\"nominal_neutral\":")
        .int(round_half_up(m, 0) as i64)
        .raw(",\"adducts\":{");
    let row = |o: &mut Out, z: f64| {
        o.raw("\"mz\":")
            .num(round(z, 4))
            .raw(",\"mz5\":")
            .num(round(z, 5))
            .raw(",\"mz1\":")
            .num(round_half_up(z, 1))
            .raw(",\"nominal\":")
            .int(round_half_up(z, 0) as i64);
    };
    for (i, (a, s)) in adducts().iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(a).raw(":{");
        row(&mut o, m + s);
        o.raw("}");
    }
    o.raw("}");
    if let Some(a) = adduct {
        let z = ion_mz(m, a).ok_or_else(|| Error::new("err.adduct.unknown").with("adduct", a))?;
        o.raw(",\"adduct\":").str(a).raw(",");
        row(&mut o, z);
    }
    o.raw("}");
    Ok(o.0)
}
