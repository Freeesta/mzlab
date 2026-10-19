//! Tokeniser of SMILES text into atoms, bonds and, per atom, the bonds in the order they are written (what `@`/`@@` and `/`/`\` refer to).
//! Nothing chemical is decided here: aromatic bonds, hydrogens and stereo are resolved by `smiles.rs`.

use crate::codec::MolError;
use crate::elements::is_element;
use std::collections::BTreeMap;

/// Bond symbol as written; `Up`/`Down` are `/` and `\`, read from the first atom of the bond to the second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sym {
    None,
    Single,
    Double,
    Triple,
    Aromatic,
    Up,
    Down,
}

pub(crate) struct RawAtom {
    pub el: String,
    pub aromatic: bool,
    pub bracket: bool,
    pub iso: u16,
    pub chg: i8,
    pub h: u8,
    /// `Some(true)` for `@@`, `Some(false)` for `@`.
    pub chiral: Option<bool>,
    /// Written after another atom of the same component (so that atom is its first neighbour).
    pub has_parent: bool,
}

pub(crate) struct RawBond {
    pub from: usize,
    pub to: usize,
    pub sym: Sym,
}

pub(crate) struct Raw {
    pub atoms: Vec<RawAtom>,
    pub bonds: Vec<RawBond>,
    /// Per atom, the ids of its bonds in the order of the text: incoming bond, ring digits, then branches and the chain.
    pub slots: Vec<Vec<usize>>,
}

impl Raw {
    /// The atom at the other end of bond `b` seen from `x`.
    pub fn other(&self, b: usize, x: usize) -> usize {
        let e = &self.bonds[b];
        if e.from == x {
            e.to
        } else {
            e.from
        }
    }
}

fn err<T>(msg: impl Into<String>) -> Result<T, MolError> {
    Err(MolError::Smiles(msg.into()))
}

struct Parser {
    s: Vec<char>,
    i: usize,
    raw: Raw,
    prev: Option<usize>,
    stack: Vec<Option<usize>>,
    sym: Sym,
    open: BTreeMap<u32, usize>,
}

/// Parses the SMILES part (no CXSMILES tail).
pub(crate) fn parse(text: &str) -> Result<Raw, MolError> {
    let mut p = Parser {
        s: text.chars().collect(),
        i: 0,
        raw: Raw {
            atoms: Vec::new(),
            bonds: Vec::new(),
            slots: Vec::new(),
        },
        prev: None,
        stack: Vec::new(),
        sym: Sym::None,
        open: BTreeMap::new(),
    };
    while p.i < p.s.len() {
        p.step()?;
    }
    if !p.stack.is_empty() {
        return err("unclosed branch '('");
    }
    if let Some(n) = p.open.keys().next() {
        return err(format!("ring closure {n} is never closed"));
    }
    if p.sym != Sym::None {
        return err("bond symbol at the end of the string");
    }
    if p.raw.atoms.is_empty() {
        return err("no atoms");
    }
    Ok(p.raw)
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    fn step(&mut self) -> Result<(), MolError> {
        let c = self.s[self.i];
        match c {
            '(' => {
                let Some(p) = self.prev else {
                    return err("branch '(' before any atom");
                };
                self.stack.push(Some(p));
                self.i += 1;
            }
            ')' => {
                let Some(top) = self.stack.pop() else {
                    return err("unmatched ')'");
                };
                self.prev = top;
                self.sym = Sym::None;
                self.i += 1;
            }
            '.' => {
                self.prev = None;
                self.sym = Sym::None;
                self.i += 1;
            }
            '-' | '=' | '#' | ':' | '/' | '\\' => {
                self.set_sym(Self::sym_of(c))?;
                self.i += 1;
            }
            '0'..='9' | '%' => {
                let n = self.ring_number()?;
                self.ring(n)?;
            }
            '[' => {
                let a = self.bracket()?;
                self.add_atom(a)?;
            }
            _ => {
                let a = self.organic()?;
                self.add_atom(a)?;
            }
        }
        Ok(())
    }

    fn sym_of(c: char) -> Sym {
        match c {
            '-' => Sym::Single,
            '=' => Sym::Double,
            '#' => Sym::Triple,
            ':' => Sym::Aromatic,
            '/' => Sym::Up,
            _ => Sym::Down,
        }
    }

    fn set_sym(&mut self, s: Sym) -> Result<(), MolError> {
        if self.sym != Sym::None {
            return err("two bond symbols in a row");
        }
        self.sym = s;
        Ok(())
    }

    fn add_atom(&mut self, mut a: RawAtom) -> Result<(), MolError> {
        let id = self.raw.atoms.len();
        a.has_parent = self.prev.is_some();
        self.raw.atoms.push(a);
        self.raw.slots.push(Vec::new());
        if let Some(p) = self.prev {
            let b = self.new_bond(p, id, self.sym);
            self.raw.slots[p].push(b);
            self.raw.slots[id].push(b);
        } else if self.sym != Sym::None {
            return err("bond symbol with no atom before it");
        }
        self.sym = Sym::None;
        self.prev = Some(id);
        Ok(())
    }

    fn new_bond(&mut self, from: usize, to: usize, sym: Sym) -> usize {
        self.raw.bonds.push(RawBond { from, to, sym });
        self.raw.bonds.len() - 1
    }

    fn ring_number(&mut self) -> Result<u32, MolError> {
        if self.s[self.i] != '%' {
            let n = self.s[self.i].to_digit(10).unwrap_or(0);
            self.i += 1;
            return Ok(n);
        }
        let digits: String = self.s[self.i + 1..].iter().take(2).collect();
        match digits.parse::<u32>() {
            Ok(n) if digits.len() == 2 && digits.chars().all(|c| c.is_ascii_digit()) => {
                self.i += 3;
                Ok(n)
            }
            _ => err("'%' needs two digits"),
        }
    }

    /// A ring digit after the current atom: opens a bond, or closes the open one with the same number.
    fn ring(&mut self, n: u32) -> Result<(), MolError> {
        let Some(cur) = self.prev else {
            return err("ring digit before any atom");
        };
        let sym = std::mem::replace(&mut self.sym, Sym::None);
        let Some(b) = self.open.remove(&n) else {
            let id = self.new_bond(cur, usize::MAX, sym);
            self.raw.slots[cur].push(id);
            self.open.insert(n, id);
            return Ok(());
        };
        let from = self.raw.bonds[b].from;
        if from == cur {
            return err(format!("ring closure {n} joins an atom to itself"));
        }
        if self.raw.slots[cur]
            .iter()
            .any(|&x| self.raw.other(x, cur) == from)
        {
            return err("two bonds between the same atoms");
        }
        // a direction written on the closing digit reads from the closing atom to the partner
        let closing = match sym {
            Sym::Up => Sym::Down,
            Sym::Down => Sym::Up,
            s => s,
        };
        let e = &mut self.raw.bonds[b];
        e.to = cur;
        if e.sym == Sym::None {
            e.sym = closing;
        } else if closing != Sym::None && closing != e.sym {
            return err(format!("ring closure {n} has two different bond symbols"));
        }
        self.raw.slots[cur].push(b);
        Ok(())
    }

    /// An atom outside brackets: B C N O P S F Cl Br I and the aromatic b c n o p s.
    fn organic(&mut self) -> Result<RawAtom, MolError> {
        let c = self.s[self.i];
        let two: String = self.s[self.i..].iter().take(2).collect();
        let (el, len, aromatic) = if two == "Cl" || two == "Br" {
            (two, 2, false)
        } else if "BCNOPSFI".contains(c) {
            (c.to_string(), 1, false)
        } else if "bcnops".contains(c) {
            (c.to_ascii_uppercase().to_string(), 1, true)
        } else {
            return err(format!("unexpected {c:?} at position {}", self.i));
        };
        self.i += len;
        Ok(atom(el, aromatic, false))
    }

    fn bracket(&mut self) -> Result<RawAtom, MolError> {
        self.i += 1;
        let mut iso = 0u32;
        while let Some(d) = self.peek().and_then(|c| c.to_digit(10)) {
            iso = iso * 10 + d;
            self.i += 1;
            if iso > u16::MAX as u32 {
                return err("isotope too large");
            }
        }
        let (el, aromatic) = self.bracket_symbol()?;
        let mut a = atom(el, aromatic, true);
        a.iso = iso as u16;
        if self.peek() == Some('@') {
            self.i += 1;
            let double = self.peek() == Some('@');
            self.i += usize::from(double);
            a.chiral = Some(double);
            if self.peek().is_some_and(|c| c.is_ascii_uppercase()) && self.peek() != Some('H') {
                return err("only @ and @@ are supported as chirality marks");
            }
        }
        if self.peek() == Some('H') {
            self.i += 1;
            a.h = self.small_number(1)? as u8;
        }
        a.chg = self.charge()?;
        if self.peek() == Some(':') {
            self.i += 1;
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.i += 1;
            }
        }
        if self.peek() != Some(']') {
            return err(format!("bracket atom not closed at position {}", self.i));
        }
        self.i += 1;
        Ok(a)
    }

    fn bracket_symbol(&mut self) -> Result<(String, bool), MolError> {
        let Some(c) = self.peek() else {
            return err("bracket atom without a symbol");
        };
        let next = self.s.get(self.i + 1).copied();
        let aromatic = c.is_ascii_lowercase();
        let one = c.to_ascii_uppercase().to_string();
        let two = next
            .filter(|n| n.is_ascii_lowercase())
            .map(|n| format!("{one}{n}"));
        let pick = match two {
            // aromatic two-letter symbols are se, as, si, te; an element like Co or Cl is never aromatic
            Some(t)
                if is_element(&t)
                    && (!aromatic || ["Se", "As", "Si", "Te"].contains(&t.as_str())) =>
            {
                t
            }
            _ => one,
        };
        if !is_element(&pick) {
            return err(format!("{pick:?} is not an element symbol"));
        }
        if aromatic
            && !["B", "C", "N", "O", "P", "S", "Se", "As", "Si", "Te"].contains(&pick.as_str())
        {
            return err(format!("{pick:?} cannot be aromatic"));
        }
        self.i += pick.len();
        Ok((pick, aromatic))
    }

    /// Digits at the cursor as a number, `default` when there are none.
    fn small_number(&mut self, default: u32) -> Result<u32, MolError> {
        let mut n = None::<u32>;
        while let Some(d) = self.peek().and_then(|c| c.to_digit(10)) {
            n = Some(n.unwrap_or(0) * 10 + d);
            self.i += 1;
            if n > Some(100) {
                return err("number too large");
            }
        }
        Ok(n.unwrap_or(default))
    }

    fn charge(&mut self) -> Result<i8, MolError> {
        let Some(sign) = self.peek().filter(|c| *c == '+' || *c == '-') else {
            return Ok(0);
        };
        self.i += 1;
        let mut n = 1u32;
        if self.peek().is_some_and(|c| c.is_ascii_digit()) {
            n = self.small_number(1)?;
        } else {
            while self.peek() == Some(sign) {
                n += 1;
                self.i += 1;
            }
        }
        let v = i8::try_from(n).map_err(|_| MolError::Smiles("charge too large".into()))?;
        Ok(if sign == '+' { v } else { -v })
    }
}

fn atom(el: String, aromatic: bool, bracket: bool) -> RawAtom {
    RawAtom {
        el,
        aromatic,
        bracket,
        iso: 0,
        chg: 0,
        h: 0,
        chiral: None,
        has_parent: false,
    }
}
