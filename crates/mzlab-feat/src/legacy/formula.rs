//! `api.formula_info`: the molecular formula of the parent from a formula or a SMILES (`TP_Mine/py/tpmine/chem.py::neutral_formula`,
//! `smiles_formula`), its neutral mass and the m/z of the ion. The error texts are those of Python (`str(e)`), since the page shows them.

use mzlab_core::elements::{fmt, ion_mz, mass, mass_of, parse_formula, Formula};
use mzlab_core::error::Error;
use mzlab_core::json::{round, Out};

/// Normal valences of the organic subset (`_ORGANIC`).
fn valences(el: &str) -> &'static [u32] {
    match el {
        "B" => &[3],
        "C" => &[4],
        "N" | "P" => &[3, 5],
        "O" => &[2],
        "S" => &[2, 4, 6],
        "F" | "Cl" | "Br" | "I" => &[1],
        _ => &[0],
    }
}

/// Python's `repr` of a string (quotes and the usual escapes; enough for the texts of the formulas).
pub fn pyrepr(s: &str) -> String {
    let q = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut o = String::from(q);
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            '\r' => o.push_str("\\r"),
            c if c == q => {
                o.push('\\');
                o.push(c);
            }
            c => o.push(c),
        }
    }
    o.push(q);
    o
}

/// `str(e)` of the `UserError` of `elements.parse_formula`.
pub(crate) fn formula_text(e: &Error) -> String {
    let p = |k: &str| {
        e.params
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    match e.key {
        "err.formula.empty" => "empty formula".into(),
        "err.formula.element" => format!(
            "unknown element {} in {}",
            pyrepr(&p("element")),
            pyrepr(&p("formula"))
        ),
        "err.formula.parens" => format!("unbalanced parentheses in {}", pyrepr(&p("formula"))),
        _ => format!("cannot read the formula {}", pyrepr(&p("formula"))),
    }
}

struct Atom {
    el: String,
    bonds: f64,
    arom: bool,
    h: Option<u32>,
}

/// One token of `_ATOM` at `i`: a bracket atom, an atom of the organic subset or a bond/branch/ring token; its end.
enum Tok {
    Bracket(String),
    Organic(String),
    Other(String),
}

fn token(c: &[char], i: usize) -> Option<(Tok, usize)> {
    let s = |a: usize, b: usize| c[a..b].iter().collect::<String>();
    if c[i] == '[' {
        let j = (i + 1..c.len()).find(|&j| c[j] == ']')?;
        return (j > i + 1).then(|| (Tok::Bracket(s(i + 1, j)), j + 1));
    }
    if i + 1 < c.len() && ((c[i] == 'C' && c[i + 1] == 'l') || (c[i] == 'B' && c[i + 1] == 'r')) {
        return Some((Tok::Organic(s(i, i + 2)), i + 2));
    }
    if "BCNOPSFIbcnops".contains(c[i]) {
        return Some((Tok::Organic(c[i].to_string()), i + 1));
    }
    if c[i] == '%' && i + 2 < c.len() && c[i + 1].is_ascii_digit() && c[i + 2].is_ascii_digit() {
        return Some((Tok::Other(s(i, i + 3)), i + 3));
    }
    if "=#:-/\\.()$".contains(c[i]) || c[i].is_ascii_digit() {
        return Some((Tok::Other(c[i].to_string()), i + 1));
    }
    None
}

/// The bracket regex `(?:\d+)?([A-Z][a-z]?|[a-z]{1,2})(?:@+)?(?:H(\d*))?([+-]+\d*)?(?::\d+)?$`: element and explicit hydrogens.
fn bracket(b: &str) -> Option<(String, bool, u32)> {
    let c: Vec<char> = b.chars().collect();
    let mut i = 0;
    while i < c.len() && c[i].is_ascii_digit() {
        i += 1;
    }
    let mut lens = vec![];
    if i < c.len() && c[i].is_ascii_alphabetic() {
        if i + 1 < c.len() && c[i + 1].is_ascii_lowercase() {
            lens.push(2);
        }
        lens.push(1);
    }
    lens.into_iter().find_map(|n| {
        let el: String = c[i..i + n].iter().collect();
        rest(&c, i + n).map(|h| {
            (
                el.clone(),
                el.chars().next().is_some_and(|x| x.is_ascii_lowercase()),
                h,
            )
        })
    })
}

/// After the element: `(?:@+)?(?:H(\d*))?([+-]+\d*)?(?::\d+)?$`; the explicit hydrogens (0 when there is no H).
fn rest(c: &[char], mut i: usize) -> Option<u32> {
    let digits = |mut j: usize| {
        while j < c.len() && c[j].is_ascii_digit() {
            j += 1;
        }
        j
    };
    while i < c.len() && c[i] == '@' {
        i += 1;
    }
    let mut h = 0;
    if i < c.len() && c[i] == 'H' {
        let e = digits(i + 1);
        h = if e > i + 1 {
            c[i + 1..e].iter().collect::<String>().parse().ok()?
        } else {
            1
        };
        i = e;
    }
    if i < c.len() && (c[i] == '+' || c[i] == '-') {
        while i < c.len() && (c[i] == '+' || c[i] == '-') {
            i += 1;
        }
        i = digits(i);
    }
    if i < c.len() && c[i] == ':' && digits(i + 1) > i + 1 {
        i = digits(i + 1);
    }
    (i == c.len()).then_some(h)
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f
            .to_uppercase()
            .chain(c.flat_map(char::to_lowercase))
            .collect(),
        None => String::new(),
    }
}

/// Reads the atoms and bonds of a SMILES (`smiles_formula`, first part).
fn atoms_of(s: &str) -> Result<Vec<Atom>, String> {
    let c: Vec<char> = s.trim().chars().collect();
    let (mut atoms, mut stack) = (Vec::<Atom>::new(), Vec::<Option<usize>>::new());
    let (mut prev, mut order) = (None::<usize>, 1.0f64);
    let mut rings: Vec<(String, Option<usize>, f64)> = vec![];
    let mut i = 0;
    while i < c.len() {
        let Some((tok, end)) = token(&c, i) else {
            let near: String = c[i..(i + 6).min(c.len())].iter().collect();
            return Err(format!("unrecognised SMILES near «{near}»"));
        };
        i = end;
        let atom = match tok {
            Tok::Bracket(b) => {
                let (el, arom, h) =
                    bracket(&b).ok_or_else(|| format!("unrecognised bracket atom: [{b}]"))?;
                Some(Atom {
                    el: capitalize(&el),
                    bonds: 0.0,
                    arom,
                    h: Some(h),
                })
            }
            Tok::Organic(o) => Some(Atom {
                el: capitalize(&o),
                bonds: 0.0,
                arom: o.chars().next().is_some_and(|x| x.is_ascii_lowercase()),
                h: None,
            }),
            Tok::Other(t) => {
                bond_token(
                    &t, &mut atoms, &mut stack, &mut prev, &mut order, &mut rings,
                )?;
                None
            }
        };
        if let Some(mut a) = atom {
            if let Some(p) = prev {
                atoms[p].bonds += order;
                a.bonds += order;
            }
            atoms.push(a);
            prev = Some(atoms.len() - 1);
            order = 1.0;
        }
    }
    if !rings.is_empty() {
        return Err("unclosed ring in the SMILES".into());
    }
    Ok(atoms)
}

fn bond_token(
    t: &str,
    atoms: &mut [Atom],
    stack: &mut Vec<Option<usize>>,
    prev: &mut Option<usize>,
    order: &mut f64,
    rings: &mut Vec<(String, Option<usize>, f64)>,
) -> Result<(), String> {
    match t {
        "(" => stack.push(*prev),
        ")" => *prev = stack.pop().ok_or("pop from empty list")?,
        "." => *prev = None,
        "=" => *order = 2.0,
        "#" => *order = 3.0,
        "-" | ":" | "/" | "\\" => *order = 1.0,
        "$" => {}
        _ => {
            let none = "list indices must be integers or slices, not NoneType";
            if let Some(k) = rings.iter().position(|(key, _, _)| key == t) {
                let (_, j, o) = rings.remove(k);
                let o = o.max(*order);
                atoms[prev.ok_or(none)?].bonds += o;
                atoms[j.ok_or(none)?].bonds += o;
            } else {
                rings.push((t.to_string(), *prev, *order));
            }
            *order = 1.0;
        }
    }
    Ok(())
}

fn bump(f: &mut Formula, el: &str, n: u64) {
    match f.iter_mut().find(|(e, _)| e == el) {
        Some(x) => x.1 += n,
        None => f.push((el.to_string(), n)),
    }
}

/// `smiles_formula`: the molecular formula of a SMILES, implicit hydrogens from the normal valences.
pub fn smiles_formula(s: &str) -> Result<Formula, String> {
    let mut f: Formula = vec![];
    for a in atoms_of(s)? {
        bump(&mut f, &a.el, 1);
        let h = match a.h {
            Some(h) => h,
            None if a.arom && a.el != "C" => 0,
            None => {
                let bonds = a.bonds + if a.arom { 1.0 } else { 0.0 };
                let v = valences(&a.el)
                    .iter()
                    .map(|&v| f64::from(v))
                    .find(|&v| v >= bonds)
                    .map(|v| v - bonds)
                    .unwrap_or(0.0);
                v.round_ties_even().max(0.0) as u32
            }
        };
        if h > 0 {
            bump(&mut f, "H", u64::from(h));
        }
    }
    if let Some((el, _)) = f.iter().find(|(e, _)| mass_of(e).is_none()) {
        return Err(format!("unsupported element: {el}"));
    }
    Ok(f)
}

fn is_formula(t: &str) -> bool {
    let c: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < c.len() {
        if !c[i].is_ascii_uppercase() {
            return false;
        }
        i += 1;
        if i < c.len() && c[i].is_ascii_lowercase() {
            i += 1;
        }
        while i < c.len() && c[i].is_ascii_digit() {
            i += 1;
        }
    }
    !c.is_empty()
}

/// `chem.neutral_formula`: a formula (`C10H12N2O3S`) or a SMILES.
pub fn neutral_formula(text: &str) -> Result<Formula, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("a neutral molecular formula or a SMILES is required".into());
    }
    if is_formula(t) {
        return parse_formula(t).map_err(|e| formula_text(&e));
    }
    smiles_formula(t)
}

/// `api.formula_info(text, adduct)` as JSON.
pub fn formula_info(text: &str, adduct: &str) -> String {
    let mut o = Out::default();
    let f = match neutral_formula(text) {
        Ok(f) => f,
        Err(e) => {
            o.raw("{\"ok\":false,\"error\":").str(&e).raw("}");
            return o.0;
        }
    };
    let m = mass(&f);
    let Some(z) = ion_mz(m, adduct) else {
        o.raw("{\"ok\":false,\"error\":")
            .str(&pyrepr(adduct))
            .raw("}");
        return o.0;
    };
    let t = text.trim();
    let plain: String = t.chars().filter(|&c| c != ' ').collect();
    let from_formula = !plain.is_empty()
        && plain.chars().all(char::is_alphanumeric)
        && t.chars().next().is_some_and(char::is_uppercase)
        && !text.contains('(');
    o.raw("{\"ok\":true,\"formula\":").str(&fmt(&f));
    o.raw(",\"neutral\":")
        .num(round(m, 4))
        .raw(",\"mz\":")
        .num(round(z, 4));
    o.raw(",\"from\":")
        .str(if from_formula { "formula" } else { "SMILES" })
        .raw("}");
    o.0
}
