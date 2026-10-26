//! Wildman-Crippen atom typing (1999 paper, contributions in `crippen_tab.rs`): each atom takes the first type whose rule fits, in the order of
//! the paper's table, and every hydrogen (implicit or an explicit `[H]` atom) takes the type given by the atom it sits on. Rules are written as
//! graph tests on the record, with the aromaticity perceived by `Mol::aromatic_bonds`; bond tests mean a plain single bond unless stated.

use crate::crippen_tab::*;
use crate::descr::Ctx;

/// (logP, MR) summed over all atoms and hydrogens.
pub(crate) fn contributions(c: &Ctx) -> (f64, f64) {
    let (mut lp, mut mr) = (0.0, 0.0);
    let mut add = |t: Ty, times: u32| {
        lp += t.0 * times as f64;
        mr += t.1 * times as f64;
    };
    for i in 0..c.m.atoms.len() {
        if c.sym(i) == "H" {
            if let Some(b) =
                c.m.bonds
                    .iter()
                    .find(|b| b.a as usize == i || b.b as usize == i)
            {
                let p = if b.a as usize == i { b.b } else { b.a } as usize;
                add(hydrogen(c, p), 1);
            } else {
                add(HS, 1);
            }
            continue;
        }
        add(heavy(c, i), 1);
        if c.m.atoms[i].h > 0 {
            add(hydrogen(c, i), c.m.atoms[i].h as u32);
        }
    }
    (lp, mr)
}

fn heavy(c: &Ctx, i: usize) -> Ty {
    let q = c.chg(i);
    match c.sym(i) {
        "C" if c.ar[i] => aromatic_carbon(c, i),
        "C" => carbon(c, i),
        "N" => nitrogen(c, i),
        "O" => oxygen(c, i),
        "F" | "Cl" | "Br" | "I" if q != 0 => HAL,
        "F" => F,
        "Cl" => CL,
        "Br" => BR,
        "I" => I,
        "P" => P,
        "S" if c.ar[i] => S3,
        "S" => {
            let hyper = c
                .doubles(i)
                .iter()
                .any(|&j| c.ali(j) && matches!(c.sym(j), "N" | "O" | "P" | "S"));
            if q != 0 || hyper {
                S2
            } else {
                S1
            }
        }
        "Li" | "Na" | "K" | "Rb" | "Cs" if q > 0 => HAL,
        "Li" | "Be" | "B" | "Na" | "Mg" | "Al" | "Si" | "K" | "Ca" | "Ga" | "Ge" | "As" | "Rb"
        | "Sr" | "In" | "Sn" | "Sb" | "Se" | "Te" | "Cs" | "Ba" | "Tl" | "Pb" | "Bi" => ME1,
        "Sc" | "Ti" | "V" | "Cr" | "Mn" | "Fe" | "Co" | "Ni" | "Cu" | "Zn" | "Y" | "Zr" | "Nb"
        | "Mo" | "Tc" | "Ru" | "Rh" | "Pd" | "Ag" | "Cd" | "Hf" | "Ta" | "W" | "Re" | "Os"
        | "Ir" | "Pt" | "Au" | "Hg" => ME2,
        _ => (0.0, 0.0),
    }
}

/// True when some two distinct atoms of `v` satisfy `f(first, second)`.
fn pair(v: &[usize], f: impl Fn(usize, usize) -> bool) -> bool {
    v.iter()
        .enumerate()
        .any(|(p, &a)| v.iter().enumerate().any(|(q, &b)| p != q && f(a, b)))
}

fn hydrogen(c: &Ctx, p: usize) -> Ty {
    let s = c.sym(p);
    if s == "C" || s == "H" {
        return H1;
    }
    if s == "O" && !c.ar[p] {
        let Some(&j) = c.singles(p).first().or(c.arom_nb(p).first()) else {
            return HS;
        };
        if (c.is_c(j) && c.x(j) == 4) || (c.ar[j] && c.sym(j) == "C") {
            return H2;
        }
        if c.ar[j] || !matches!(c.sym(j), "C" | "N" | "O" | "S") {
            return H2;
        }
        if c.sym(j) == "N" {
            return H3;
        }
        let dbl_to_x = c.is_c(j)
            && c.doubles(j)
                .iter()
                .any(|&d| matches!(c.sym(d), "C" | "N" | "O" | "S"));
        if dbl_to_x || matches!(c.sym(j), "O" | "S") {
            return H4;
        }
        return HS;
    }
    if s == "N" {
        return H3;
    }
    if s == "O" {
        return HS;
    }
    H2
}

fn carbon(c: &Ctx, i: usize) -> Ty {
    let (h, x, n) = (c.hs[i], c.x(i), c.nb[i].len());
    let (s, d, t) = (c.singles(i), c.doubles(i), c.triples(i));
    let count = |f: &dyn Fn(usize) -> bool| s.iter().filter(|&&j| f(j)).count();
    let (sc, sa, sh) = (
        count(&|j| c.is_c(j)),
        count(&|j| c.ali(j)),
        count(&|j| c.hetero(j)),
    );
    let plain = d.is_empty() && t.is_empty();
    if plain && (h == 4 || (h == 3 && n == 1 && sc == 1) || (h == 2 && n == 2 && sc == 2)) {
        return C1;
    }
    if plain && ((h == 1 && n == 3 && sc == 3) || (h == 0 && n == 4 && sc == 4)) {
        return C2;
    }
    if plain
        && ((h == 3 && n == 1 && sh == 1) || (h == 2 && x == 4 && n == 2 && sh >= 1 && sa == 2))
    {
        return C3;
    }
    if plain
        && sh >= 1
        && x == 4
        && ((h == 1 && n == 3 && sa == 3) || (h == 0 && n == 4 && sa == 4))
    {
        return C4;
    }
    if d.iter().any(|&j| c.ali(j) && c.sym(j) != "C") {
        return C5;
    }
    let dc = d.iter().filter(|&&j| c.is_c(j)).count();
    if (h == 2 && n == 1 && dc == 1)
        || (h == 1 && n == 2 && dc == 1 && sa == 1)
        || (h == 0 && n == 3 && dc == 1 && sa == 2)
        || dc == 2
    {
        return C6;
    }
    if x == 2 && t.iter().any(|&j| c.ali(j)) {
        return C7;
    }
    let arom_single: Vec<usize> = s.iter().copied().filter(|&j| c.ar[j]).collect();
    if plain && !arom_single.is_empty() {
        let aromatic_c = arom_single.iter().any(|&j| c.sym(j) == "C");
        match (h, x) {
            (3, _) if aromatic_c => return C8,
            (3, _) => return C9,
            (2, 4) => return C10,
            (1, 4) => return C11,
            (0, 4) => return C12,
            _ => {}
        }
    }
    // C=C with an aromatic neighbour: [C](=C)(a)A, [C](=C)(c)a and [CH](=C)a
    if dc == 1
        && ((h == 1 && !arom_single.is_empty())
            || pair(&s, |a, b| c.ar[a] && (c.ali(b) || c.ar[b])))
    {
        return C26;
    }
    if d.iter().any(|&j| c.ar[j] && c.sym(j) == "C") {
        return C26;
    }
    if x == 4
        && s.iter().any(|&j| {
            c.ali(j)
                && !matches!(
                    c.sym(j),
                    "C" | "N" | "O" | "P" | "S" | "F" | "Cl" | "Br" | "I"
                )
        })
    {
        return C27;
    }
    CS
}

fn aromatic_carbon(c: &Ctx, i: usize) -> Ty {
    let h = c.hs[i];
    let s = c.singles(i);
    if h == 0
        && s.iter().any(|&j| {
            c.ali(j) && !matches!(c.sym(j), "C" | "N" | "O" | "S" | "F" | "Cl" | "Br" | "I")
        })
    {
        return C13;
    }
    for (el, ty) in [("F", C14), ("Cl", C15), ("Br", C16), ("I", C17)] {
        if c.nb[i]
            .iter()
            .any(|b| (b.o == 1 || b.ab) && c.sym(b.j) == el)
        {
            return ty;
        }
    }
    if h > 0 {
        return C18;
    }
    if c.arom_nb(i).len() == 3 {
        return C19;
    }
    if c.arom_nb(i).len() == 2 {
        if s.iter().any(|&j| c.ar[j]) {
            return C20;
        }
        for (el, ty) in [("C", C21), ("N", C22), ("O", C23), ("S", C24)] {
            if s.iter().any(|&j| c.ali(j) && c.sym(j) == el) {
                return ty;
            }
        }
        if c.doubles(i)
            .iter()
            .any(|&j| c.ali(j) && matches!(c.sym(j), "C" | "N" | "O"))
        {
            return C25;
        }
    }
    CS
}

fn nitrogen(c: &Ctx, i: usize) -> Ty {
    let (h, n, q) = (c.hs[i], c.nb[i].len(), c.chg(i));
    if c.ar[i] {
        return if q == 0 { N11 } else { N12 };
    }
    let (s, d, t) = (c.singles(i), c.doubles(i), c.triples(i));
    let all_ali = s.iter().all(|&j| c.ali(j));
    let n_ar = s.iter().filter(|&&j| c.ar[j]).count();
    let plain = d.is_empty() && t.is_empty();
    if q == 0 {
        if plain && h == 2 && n == 1 && all_ali {
            return N1;
        }
        if plain && h == 1 && n == 2 && all_ali {
            return N2;
        }
        if plain && h == 2 && n == 1 && n_ar == 1 {
            return N3;
        }
        if plain && h == 1 && n == 2 && n_ar >= 1 {
            return N4;
        }
        if h == 1 && n == 1 && d.len() == 1 {
            return N5;
        }
        if h == 0 && d.len() == 1 && s.len() == 1 {
            return N6;
        }
        if plain && h == 0 && n == 3 && all_ali {
            return N7;
        }
        if plain && h == 0 && n == 3 && n_ar >= 1 && (n_ar == 3 || s.iter().any(|&j| c.ali(j))) {
            return N8;
        }
        if t.iter().any(|&j| c.ali(j)) {
            return N9;
        }
        if q == 0 {
            return NS;
        }
    }
    if q > 0 && (1..=3).contains(&h) {
        return N10;
    }
    if q > 0 && h == 0 {
        let a4 = plain && n == 4 && all_ali;
        let a3 = d.len() == 1 && c.ali(d[0]) && s.len() == 2 && s.iter().any(|&j| c.ali(j));
        let c2 = d.len() == 2
            && d.iter().any(|&j| c.sym(j) == "C")
            && d.iter().any(|&j| c.sym(j) == "N");
        if a4 || a3 || c2 {
            return N13;
        }
        if t.iter().any(|&j| c.ali(j))
            || (d.len() == 2 && d.iter().any(|&j| c.sym(j) == "N" && c.chg(j) < 0))
        {
            return N14;
        }
    }
    if q < 0 {
        return N14;
    }
    NS
}

fn oxygen(c: &Ctx, i: usize) -> Ty {
    let (h, n, q) = (c.hs[i], c.nb[i].len(), c.chg(i));
    if c.ar[i] {
        return O1;
    }
    let (s, d) = (c.singles(i), c.doubles(i));
    if (1..=2).contains(&h) {
        return O2;
    }
    if n == 2 && c.nb[i].iter().all(|b| b.o == 1 && c.ali(b.j)) {
        return O3;
    }
    if n == 2 && c.nb[i].iter().any(|b| c.ar[b.j]) {
        return O4;
    }
    if d.iter().any(|&j| matches!(c.sym(j), "N" | "O"))
        || (q < 0 && n == 1 && s.len() == 1 && c.sym(s[0]) == "N")
    {
        return O5;
    }
    if (q < 0 && n == 1 && s.len() == 1 && c.sym(s[0]) == "S")
        || (q == 0 && d.iter().any(|&j| c.sym(j) == "S" && c.chg(j) == 0))
    {
        return O6;
    }
    if q < 0
        && n == 1
        && s.len() == 1
        && c.is_c(s[0])
        && c.doubles(s[0]).iter().any(|&j| c.sym(j) == "O")
    {
        return O12;
    }
    if q < 0 && n == 1 && !matches!(c.sym(c.nb[i][0].j), "N" | "S") {
        return O7;
    }
    if d.iter().any(|&j| c.ar[j] && c.sym(j) == "C") {
        return O8;
    }
    match d.iter().find(|&&j| c.is_c(j)) {
        Some(&cc) => carbonyl(c, i, cc),
        None => OS,
    }
}

/// Carbonyl oxygen `i` double-bonded to the aliphatic carbon `cc`: O9 (aliphatic neighbours), O10 (aromatic neighbour), O11 (two heteroatoms).
fn carbonyl(c: &Ctx, i: usize, cc: usize) -> Ty {
    let hc = c.hs[cc];
    let o: Vec<usize> = c.singles(cc).into_iter().filter(|&j| j != i).collect();
    let co2 = c.x(cc) == 2 && c.doubles(cc).iter().filter(|&&j| c.sym(j) == "O").count() == 2;
    let aliph_c = |j: usize| c.is_c(j);
    if (hc == 1 && o.len() == 1 && aliph_c(o[0]))
        || pair(&o, |a, b| aliph_c(a) && c.ali(b))
        || (hc == 1 && o.len() == 1 && c.ali(o[0]) && matches!(c.sym(o[0]), "N" | "O"))
        || hc == 2
        || co2
    {
        return O9;
    }
    let arom_c = |j: usize| c.ar[j] && c.sym(j) == "C";
    if (hc == 1 && o.len() == 1 && arom_c(o[0]))
        || pair(&o, |a, b| (aliph_c(a) || arom_c(a)) && c.ar[b])
        || pair(&o, |a, b| arom_c(a) && c.ali(b))
    {
        return O10;
    }
    if pair(&o, |a, b| c.sym(a) != "C" && c.sym(b) != "C") {
        return O11;
    }
    OS
}
