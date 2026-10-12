//! The reading of a SMILES: atoms and bonds as written (organic subset, brackets, lower-case aromatic atoms, ring closures `1`-`9` and `%nn`), then the
//! implicit hydrogens by valence (as `molgraph.py`: an aromatic C or N gives one electron to the ring; hypervalent N, P, S take the next valence).

use mzlab_core::error::{Error, Result};

use super::grafo::{Atom, Bond};

pub(super) fn bad(smiles: &str) -> Error {
    Error::new("error.grafo.smiles").with("smiles", smiles)
}

pub(super) fn normal_valence(el: &str) -> Option<u32> {
    Some(match el {
        "B" => 3,
        "C" => 4,
        "N" => 3,
        "O" => 2,
        "P" => 3,
        "S" => 2,
        "F" | "Cl" | "Br" | "I" => 1,
        _ => return None,
    })
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
        .unwrap_or_default()
}

pub(super) struct RawAtom {
    pub el: String,
    pub arom: bool,
    pub h: Option<u32>,
    pub charge: i32,
}

/// Digits at `b[*i..]` as a number (None when there are none).
fn number(b: &[char], i: &mut usize, smiles: &str) -> Result<Option<i32>> {
    let s = *i;
    while *i < b.len() && b[*i].is_ascii_digit() {
        *i += 1;
    }
    if s == *i {
        return Ok(None);
    }
    let n = b[s..*i]
        .iter()
        .collect::<String>()
        .parse()
        .map_err(|_| bad(smiles))?;
    Ok(Some(n))
}

/// `H` or `Hn` of a bracket atom (0 when absent).
fn hcount(b: &[char], i: &mut usize, smiles: &str) -> Result<u32> {
    if *i < b.len() && b[*i] == 'H' {
        *i += 1;
        return Ok(number(b, i, smiles)?.map_or(1, |n| n as u32));
    }
    Ok(0)
}

/// `+`, `++`, `+2`, `-`, `-3` of a bracket atom (0 when absent).
fn charge_of(b: &[char], i: &mut usize, smiles: &str) -> Result<i32> {
    if *i >= b.len() || (b[*i] != '+' && b[*i] != '-') {
        return Ok(0);
    }
    let c0 = b[*i];
    let sign = if c0 == '+' { 1 } else { -1 };
    *i += 1;
    if let Some(n) = number(b, i, smiles)? {
        return Ok(sign * n);
    }
    let mut n = 1;
    while *i < b.len() && b[*i] == c0 {
        n += 1;
        *i += 1;
    }
    Ok(sign * n)
}

/// `[13CH3+]`, `[nH]`, `[O-]`: isotope, element, chirality, hydrogens, charge, atom class.
fn bracket(t: &str, smiles: &str) -> Result<RawAtom> {
    let b: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let start = i;
    if i < b.len() && b[i].is_ascii_uppercase() {
        i += 1;
        if i < b.len() && b[i].is_ascii_lowercase() {
            i += 1;
        }
    } else {
        while i < b.len() && b[i].is_ascii_lowercase() && i - start < 2 {
            i += 1;
        }
    }
    let name: String = b[start..i].iter().collect();
    if name.is_empty() {
        return Err(bad(smiles));
    }
    let arom = name.chars().next().is_some_and(|c| c.is_ascii_lowercase());
    while i < b.len() && b[i] == '@' {
        i += 1;
    }
    let h = hcount(&b, &mut i, smiles)?;
    let charge = charge_of(&b, &mut i, smiles)?;
    if i < b.len() && b[i] == ':' {
        i = b.len();
    }
    if i != b.len() {
        return Err(bad(smiles));
    }
    Ok(RawAtom {
        el: cap(&name),
        arom,
        h: Some(h),
        charge,
    })
}

/// (atom, atom, order, written explicitly)
pub(super) type RawBond = (usize, usize, f32, bool);

/// Atom outside brackets at `s[*p]`, if there is one (`Cl`, `Br`, `BCNOPSFI`, `bcnops`).
fn organic(s: &[char], p: &mut usize) -> Option<RawAtom> {
    let c = s[*p];
    let two: String = s[*p..(*p + 2).min(s.len())].iter().collect();
    let (el, arom, n) = if two == "Cl" || two == "Br" {
        (two, false, 2)
    } else if "BCNOPSFI".contains(c) {
        (c.to_string(), false, 1)
    } else if "bcnops".contains(c) {
        (cap(&c.to_string()), true, 1)
    } else {
        return None;
    };
    *p += n;
    Some(RawAtom {
        el,
        arom,
        h: None,
        charge: 0,
    })
}

#[derive(Default)]
struct Reader {
    raw: Vec<RawAtom>,
    bonds: Vec<RawBond>,
    stack: Vec<usize>,
    open: Vec<(String, usize, f32, bool)>,
    prev: Option<usize>,
    order: f32,
    explicit: bool,
}

impl Reader {
    fn atom(&mut self, a: RawAtom) {
        self.raw.push(a);
        let i = self.raw.len() - 1;
        if let Some(pr) = self.prev {
            self.bonds.push((pr, i, self.order, self.explicit));
        }
        self.prev = Some(i);
        (self.order, self.explicit) = (1.0, false);
    }

    /// Ring closure `key`: opens it, or closes the one open.
    fn ring(&mut self, key: String, smiles: &str) -> Result<()> {
        let here = self.prev.ok_or_else(|| bad(smiles))?;
        if let Some(k) = self.open.iter().position(|o| o.0 == key) {
            let (_, j, o, ex) = self.open.remove(k);
            self.bonds
                .push((j, here, o.max(self.order), ex || self.explicit));
        } else {
            self.open.push((key, here, self.order, self.explicit));
        }
        (self.order, self.explicit) = (1.0, false);
        Ok(())
    }

    /// Everything that is not an atom: branches, bond symbols, ring closures.
    fn symbol(&mut self, s: &[char], p: &mut usize, smiles: &str) -> Result<()> {
        let c = s[*p];
        *p += 1;
        match c {
            '(' => self.stack.push(self.prev.ok_or_else(|| bad(smiles))?),
            ')' => self.prev = Some(self.stack.pop().ok_or_else(|| bad(smiles))?),
            '=' => (self.order, self.explicit) = (2.0, true),
            '#' => (self.order, self.explicit) = (3.0, true),
            '-' => (self.order, self.explicit) = (1.0, true),
            ':' => (self.order, self.explicit) = (1.5, true),
            '/' | '\\' => {}
            '.' => return Err(Error::new("error.grafo.mixture")),
            '%' => {
                let key: String = s[*p..(*p + 2).min(s.len())].iter().collect();
                *p += 2;
                self.ring(key, smiles)?;
            }
            '0'..='9' => self.ring(c.to_string(), smiles)?,
            _ => return Err(bad(smiles)),
        }
        Ok(())
    }
}

/// The atoms and bonds of one SMILES, in the order written.
pub(super) fn read(smiles: &str) -> Result<(Vec<RawAtom>, Vec<RawBond>)> {
    let s: Vec<char> = smiles.trim().chars().collect();
    let mut r = Reader {
        order: 1.0,
        ..Reader::default()
    };
    let mut p = 0;
    while p < s.len() {
        if s[p] == '[' {
            let end = s[p..]
                .iter()
                .position(|x| *x == ']')
                .ok_or_else(|| bad(smiles))?
                + p;
            let a = bracket(&s[p + 1..end].iter().collect::<String>(), smiles)?;
            p = end + 1;
            r.atom(a);
        } else if let Some(a) = organic(&s, &mut p) {
            r.atom(a);
        } else {
            r.symbol(&s, &mut p, smiles)?;
        }
    }
    if !r.open.is_empty() || !r.stack.is_empty() || r.raw.is_empty() {
        return Err(bad(smiles));
    }
    Ok((r.raw, r.bonds))
}

/// Atoms with their hydrogens: those written in brackets, the others by valence.
pub(super) fn fill(raw: &[RawAtom], bonds: &[Bond]) -> Vec<Atom> {
    let mut used = vec![0f32; raw.len()];
    for b in bonds {
        let v = if b.order == 1.5 { 1.0 } else { b.order };
        used[b.a] += v;
        used[b.b] += v;
    }
    raw.iter()
        .enumerate()
        .map(|(i, r)| {
            let h = r.h.unwrap_or_else(|| {
                let extra = if r.arom && (r.el == "C" || r.el == "N") {
                    1.0
                } else {
                    0.0
                };
                let u = used[i] + extra;
                let normal = normal_valence(&r.el).unwrap_or(0) as f32;
                let free = if u > normal && matches!(r.el.as_str(), "N" | "P" | "S") {
                    let ladder: &[f32] = if r.el == "S" { &[4.0, 6.0] } else { &[5.0] };
                    ladder.iter().find(|v| **v >= u).map_or(0.0, |v| v - u)
                } else {
                    normal - u
                };
                free.round().max(0.0) as u32
            });
            Atom {
                el: r.el.clone(),
                arom: r.arom,
                h,
                charge: r.charge,
            }
        })
        .collect()
}
