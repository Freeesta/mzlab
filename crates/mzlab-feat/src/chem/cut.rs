//! `cut/1`: a cut of a drawn structure across one or more bonds (a cut line, or selected bonds), the way a student works a fragmentation out on paper.
//! Homolytic (each atom keeps one electron per bond order: radical marks), heterolytic (the pair goes to one side: a cation and an anion) or the «+H»
//! shorthand (each side closes with one H per bond order). Every atom keeps its hydrogens; the pieces come with the numbers of the Draw labels
//! (`labels::shape`) and a shift that moves them apart along the cut bonds. Candidates to judge, never a ranking of cuts.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Out, Value};

use super::graph::{props, Mol};
use super::labels::{shape, shape_json, unpaired_of, AtomIn};

/// How the electrons of a cut bond go.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Homolytic,
    Heterolytic,
    Hydrogen,
}

impl Mode {
    fn parse(s: &str) -> Result<Mode> {
        Ok(match s {
            "" | "homolytic" => Mode::Homolytic,
            "heterolytic" => Mode::Heterolytic,
            "hydrogen" => Mode::Hydrogen,
            o => return Err(Error::new("error.grafo.cut.mode").with("mode", o)),
        })
    }

    fn name(self) -> &'static str {
        match self {
            Mode::Homolytic => "homolytic",
            Mode::Heterolytic => "heterolytic",
            Mode::Hydrogen => "hydrogen",
        }
    }
}

/// An atom after the cut: charge, radical code (`M  RAD`) and hydrogens.
#[derive(Clone, Copy, PartialEq)]
struct State {
    q: i32,
    rad: i64,
    h: u32,
}

/// Coordinates of the atoms of a V2000 molfile (x, y as written: y up).
fn coords(mf: &str, na: usize) -> Vec<(f64, f64)> {
    let num = |l: &str, a: usize, b: usize| {
        l.get(a..b)
            .and_then(|t| t.trim().parse::<f64>().ok())
            .unwrap_or(0.0)
    };
    mf.lines()
        .skip(4)
        .take(na)
        .map(|l| (num(l, 0, 10), num(l, 10, 20)))
        .collect()
}

/// Radical codes of the atoms (`M  RAD`), 0 = none.
fn radicals(mf: &str, na: usize) -> Vec<i64> {
    let lines: Vec<&str> = mf.lines().collect();
    let mut r = vec![0; na];
    for (i, c) in props(&lines, "M  RAD", na) {
        r[i] = c as i64;
    }
    r
}

/// The request: `{"molfile", "bonds": [[a, b], ..], "mode", "cation": [atom, ..], "gap"}`; atoms are 0-based positions of the file.
struct Request {
    mf: String,
    bonds: Vec<(usize, usize)>,
    mode: Mode,
    cation: Vec<usize>,
    gap: f64,
}

fn request(text: &str) -> Result<Request> {
    let bad = || Error::new("error.grafo.cut.request");
    let v = json::parse(text.as_bytes()).map_err(|_| bad())?;
    let idx = |x: &Value| {
        x.f64()
            .filter(|f| *f >= 0.0 && f.fract() == 0.0)
            .map(|f| f as usize)
            .ok_or_else(bad)
    };
    let mut bonds = vec![];
    for p in v.get("bonds").map_or(&[][..], Value::arr) {
        bonds.push((
            idx(p.at(0).ok_or_else(bad)?)?,
            idx(p.at(1).ok_or_else(bad)?)?,
        ));
    }
    let cation = v
        .get("cation")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(idx)
        .collect::<Result<Vec<_>>>()?;
    Ok(Request {
        mf: v
            .get("molfile")
            .and_then(Value::str)
            .unwrap_or("")
            .to_string(),
        bonds,
        mode: Mode::parse(v.get("mode").and_then(Value::str).unwrap_or(""))?,
        cation,
        gap: v
            .get("gap")
            .and_then(Value::f64)
            .unwrap_or(1.0)
            .clamp(0.0, 10.0),
    })
}

/// The bond index of each requested pair; an atom pair that is no bond, a repeated or an aromatic bond is refused.
fn bond_ids(m: &Mol, pairs: &[(usize, usize)]) -> Result<Vec<usize>> {
    if pairs.is_empty() {
        return Err(Error::new("error.grafo.cut.none"));
    }
    let mut ks = vec![];
    for &(a, b) in pairs {
        let k = (0..m.bonds.len())
            .find(|k| {
                let bd = &m.bonds[*k];
                (bd.a, bd.b) == (a, b) || (bd.a, bd.b) == (b, a)
            })
            .ok_or_else(|| Error::new("error.grafo.molfile"))?;
        if m.bonds[k].order == 1.5 {
            return Err(Error::new("error.grafo.cut.aromatic"));
        }
        if ks.contains(&k) {
            return Err(Error::new("error.grafo.cut.request"));
        }
        ks.push(k);
    }
    Ok(ks)
}

/// The atoms after the cut: homolytic adds one unpaired electron per bond order on each end, heterolytic gives + to the `cation` end and − to the
/// other (single bonds only), «+H» one H per bond order; more than two unpaired electrons on an atom, or a mark on a singlet atom, is refused.
fn after(m: &Mol, rad: &[i64], ks: &[usize], r: &Request) -> Result<Vec<State>> {
    let mut st: Vec<State> = m
        .atoms
        .iter()
        .zip(rad)
        .map(|(a, &rad)| State {
            q: a.charge,
            rad,
            h: a.h,
        })
        .collect();
    let mut unp: Vec<u64> = rad.iter().map(|c| unpaired_of(*c)).collect();
    for (n, &k) in ks.iter().enumerate() {
        let bd = m.bonds[k];
        let o = bd.order as u32;
        match r.mode {
            Mode::Homolytic => {
                for x in [bd.a, bd.b] {
                    if rad[x] == 1 {
                        return Err(Error::new("error.grafo.cut.radical").with("atom", x + 1));
                    }
                    unp[x] += o as u64;
                }
            }
            Mode::Heterolytic => {
                if o != 1 {
                    return Err(Error::new("error.grafo.cut.order"));
                }
                let c = r.cation.get(n).copied().unwrap_or(bd.a);
                let an = if c == bd.a {
                    bd.b
                } else if c == bd.b {
                    bd.a
                } else {
                    return Err(Error::new("error.grafo.cut.request"));
                };
                st[c].q += 1;
                st[an].q -= 1;
            }
            Mode::Hydrogen => {
                st[bd.a].h += o;
                st[bd.b].h += o;
            }
        }
    }
    for (i, u) in unp.iter().enumerate() {
        st[i].rad = match u {
            0 => st[i].rad,
            1 => 2,
            2 => 3,
            _ => return Err(Error::new("error.grafo.cut.radical").with("atom", i + 1)),
        };
    }
    Ok(st)
}

/// Hydrogens that a reader filling valences (the molfile reader here, Ketcher on the page) gives each atom after the cut: the file without the cut
/// bonds and with the new charges and radical marks, read again. Where it differs from the hydrogens kept, the page pins the count.
fn by_valence(mf: &str, n: usize, ks: &[usize], st: &[State]) -> Result<Vec<u32>> {
    let lines: Vec<&str> = mf.lines().collect();
    let nb = lines
        .get(3)
        .and_then(|l| l.get(3..6))
        .and_then(|t| t.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let kept: Vec<&str> = (0..nb)
        .filter(|k| !ks.contains(k))
        .filter_map(|k| lines.get(4 + n + k).copied())
        .collect();
    let rest = lines.get(3).and_then(|l| l.get(6..)).unwrap_or("");
    let mut out = lines[..3].join("\n") + &format!("\n{n:>3}{:>3}{rest}\n", kept.len());
    for l in lines[4..4 + n].iter().chain(kept.iter()) {
        out += l;
        out += "\n";
    }
    for l in lines[4 + n + nb..]
        .iter()
        .filter(|l| l.starts_with("M  ISO"))
    {
        out += l;
        out += "\n";
    }
    for (i, x) in st.iter().enumerate() {
        if x.q != 0 {
            out += &format!("M  CHG  1 {:>3} {:>3}\n", i + 1, x.q);
        }
        if x.rad != 0 {
            out += &format!("M  RAD  1 {:>3} {:>3}\n", i + 1, x.rad);
        }
    }
    Ok(Mol::from_molfile(&(out + "M  END"))?
        .atoms
        .iter()
        .map(|a| a.h)
        .collect())
}

/// Connected pieces of the molecule without the cut bonds, each a list of atoms in file order; pieces in the order of their first atom.
fn pieces(m: &Mol, ks: &[usize]) -> Vec<Vec<usize>> {
    let mut piece = vec![usize::MAX; m.atoms.len()];
    let mut out: Vec<Vec<usize>> = vec![];
    for s in 0..m.atoms.len() {
        if piece[s] != usize::MAX {
            continue;
        }
        let p = out.len();
        piece[s] = p;
        let (mut stack, mut atoms) = (vec![s], vec![]);
        while let Some(u) = stack.pop() {
            atoms.push(u);
            for (v, k) in &m.adj[u] {
                if !ks.contains(k) && piece[*v] == usize::MAX {
                    piece[*v] = p;
                    stack.push(*v);
                }
            }
        }
        atoms.sort_unstable();
        out.push(atoms);
    }
    out
}

/// Shift of each piece: half the gap away from the other side of its cut bonds, along their mean direction (none for a ring opened in one place).
fn shifts(
    m: &Mol,
    ks: &[usize],
    ps: &[Vec<usize>],
    xy: &[(f64, f64)],
    gap: f64,
) -> Vec<(f64, f64)> {
    let of = |i: usize| ps.iter().position(|p| p.contains(&i)).unwrap_or(0);
    let mut acc = vec![(0.0, 0.0); ps.len()];
    for &k in ks {
        let (a, b) = (m.bonds[k].a, m.bonds[k].b);
        let (pa, pb) = (of(a), of(b));
        let (dx, dy) = (xy[a].0 - xy[b].0, xy[a].1 - xy[b].1);
        let len = (dx * dx + dy * dy).sqrt();
        if pa == pb || len == 0.0 {
            continue;
        }
        acc[pa] = (acc[pa].0 + dx / len, acc[pa].1 + dy / len);
        acc[pb] = (acc[pb].0 - dx / len, acc[pb].1 - dy / len);
    }
    acc.into_iter()
        .map(|(x, y)| {
            let len = (x * x + y * y).sqrt();
            if len < 1e-9 {
                (0.0, 0.0)
            } else {
                (round(x / len * gap / 2.0, 4), round(y / len * gap / 2.0, 4))
            }
        })
        .collect()
}

/// `cut/1` of a request (see `Request`): `{"format","mode","atoms":[{i, charge, radical, h, pin}],"pieces":[{atoms, shift:[dx,dy], formula, text, charge,
/// mass, nominal, mz, unpaired, electrons, odd, kind}]}`. `atoms` lists the atoms the cut changes (the page sets them on its drawing; `pin`: the valence rule would give another H count, so `h` is written on the atom); `shift` is in
/// the units and axes of the molfile (y up). Pieces are candidates: no cut is preferred to another.
pub fn cut_json(text: &str) -> Result<String> {
    let r = request(text)?;
    let m = Mol::from_molfile(&r.mf)?;
    let n = m.atoms.len();
    let (xy, rad) = (coords(&r.mf, n), radicals(&r.mf, n));
    let ks = bond_ids(&m, &r.bonds)?;
    let st = after(&m, &rad, &ks, &r)?;
    let ps = pieces(&m, &ks);
    let sh = shifts(&m, &ks, &ps, &xy, r.gap);
    let fill = by_valence(&r.mf, n, &ks, &st)?;
    let mut o = Out::default();
    o.raw("{\"format\":\"cut/1\",\"mode\":")
        .str(r.mode.name())
        .raw(",\"atoms\":[");
    let mut first = true;
    for (i, s) in st.iter().enumerate() {
        let was = State {
            q: m.atoms[i].charge,
            rad: rad[i],
            h: m.atoms[i].h,
        };
        if *s == was {
            continue;
        }
        o.raw(if first { "" } else { "," });
        first = false;
        o.raw("{\"i\":")
            .int(i as i64)
            .raw(",\"charge\":")
            .int(s.q as i64);
        o.raw(",\"radical\":")
            .int(s.rad)
            .raw(",\"h\":")
            .int(s.h as i64);
        o.raw(",\"pin\":")
            .raw(if fill[i] != s.h { "true" } else { "false" })
            .raw("}");
    }
    o.raw("],\"pieces\":[");
    for (p, atoms) in ps.iter().enumerate() {
        let ins: Vec<AtomIn> = atoms
            .iter()
            .map(|&i| {
                let a = &m.atoms[i];
                AtomIn {
                    el: &a.el,
                    iso: a.iso as f64,
                    h: st[i].h as f64,
                    q: st[i].q as f64,
                    r: st[i].rad as f64,
                }
            })
            .collect();
        let s = shape(&ins)?;
        o.raw(if p > 0 {
            ",{\"atoms\":["
        } else {
            "{\"atoms\":["
        });
        for (j, i) in atoms.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).int(*i as i64);
        }
        o.raw("],\"shift\":[")
            .num(sh[p].0)
            .raw(",")
            .num(sh[p].1)
            .raw("],\"label\":");
        shape_json(&mut o, &s);
        o.raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// V2000 molfile of atoms `(el, x, y)` with bonds `(a, b, type)` (1-based) and property lines.
    pub(crate) fn molfile(
        atoms: &[(&str, f64, f64)],
        bonds: &[(usize, usize, u8)],
        props: &[&str],
    ) -> String {
        let mut s = format!(
            "\n  qqq\n\n{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
            atoms.len(),
            bonds.len()
        );
        for (e, x, y) in atoms {
            s += &format!(
                "{x:>10.4}{y:>10.4}{:>10.4} {e:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n",
                0.0
            );
        }
        for (a, b, o) in bonds {
            s += &format!("{a:>3}{b:>3}{o:>3}  0  0  0  0\n");
        }
        for p in props {
            s += &format!("{p}\n");
        }
        s + "M  END"
    }

    fn req(mf: &str, bonds: &str, mode: &str, cation: &str) -> String {
        format!(
            "{{\"molfile\":{},\"bonds\":{bonds},\"mode\":\"{mode}\",\"cation\":{cation}}}",
            {
                let mut o = Out::default();
                o.str(mf);
                o.0
            }
        )
    }

    fn ethanol() -> String {
        molfile(
            &[("C", 0.0, 0.0), ("C", 1.0, 0.0), ("O", 2.0, 0.0)],
            &[(1, 2, 1), (2, 3, 1)],
            &[],
        )
    }

    #[test]
    fn homolytic_gives_two_radicals_moved_apart() {
        let j = cut_json(&req(&ethanol(), "[[0,1]]", "homolytic", "[]")).unwrap();
        assert!(
            j.contains("{\"i\":0,\"charge\":0,\"radical\":2,\"h\":3,\"pin\":false}"),
            "{j}"
        );
        assert!(
            j.contains("\"formula\":\"CH3\"") && j.contains("\"formula\":\"CH3O\""),
            "{j}"
        );
        assert!(
            j.contains("\"kind\":\"radical\"") && !j.contains("\"kind\":\"neutral\""),
            "{j}"
        );
        assert!(
            j.contains("\"shift\":[-0.5,0]") && j.contains("\"shift\":[0.5,0]"),
            "{j}"
        );
        assert!(
            j.contains("\"unpaired\":1,\"electrons\":9,\"odd\":true"),
            "{j}"
        );
    }

    #[test]
    fn heterolytic_puts_the_pair_on_the_anion() {
        let j = cut_json(&req(&ethanol(), "[[1,2]]", "heterolytic", "[1]")).unwrap();
        assert!(
            j.contains("\"text\":\"C2H5+\"") && j.contains("\"kind\":\"cation\""),
            "{j}"
        );
        assert!(
            j.contains("\"text\":\"HO-\"") && j.contains("\"kind\":\"anion\""),
            "{j}"
        );
        assert!(
            j.contains("{\"i\":2,\"charge\":-1,\"radical\":0,\"h\":1,\"pin\":false}"),
            "{j}"
        );
        assert_eq!(
            cut_json(&req(&ethanol(), "[[1,2]]", "heterolytic", "[0]"))
                .unwrap_err()
                .key,
            "error.grafo.cut.request"
        );
    }

    #[test]
    fn a_nitrenium_keeps_its_hydrogens() {
        let mf = molfile(&[("C", 0.0, 0.0), ("N", 1.0, 0.0)], &[(1, 2, 1)], &[]);
        let j = cut_json(&req(&mf, "[[0,1]]", "heterolytic", "[1]")).unwrap();
        assert!(
            j.contains("{\"i\":1,\"charge\":1,\"radical\":0,\"h\":2,\"pin\":true}"),
            "{j}"
        );
        assert!(
            j.contains("\"text\":\"H2N+\"") && j.contains("\"text\":\"CH3-\""),
            "{j}"
        );
    }

    #[test]
    fn hydrogen_shorthand_and_ring_cuts() {
        let j = cut_json(&req(&ethanol(), "[[1,2]]", "hydrogen", "[]")).unwrap();
        assert!(
            j.contains("\"formula\":\"C2H6\"")
                && j.contains("\"formula\":\"H2O\"")
                && j.contains("\"mz\":null"),
            "{j}"
        );
        let ring = molfile(
            &[
                ("C", 0.0, 0.0),
                ("C", 1.0, 0.0),
                ("C", 1.5, 0.8),
                ("C", 1.0, 1.6),
                ("C", 0.0, 1.6),
                ("C", -0.5, 0.8),
            ],
            &[
                (1, 2, 1),
                (2, 3, 1),
                (3, 4, 1),
                (4, 5, 1),
                (5, 6, 1),
                (6, 1, 1),
            ],
            &[],
        );
        let one = cut_json(&req(&ring, "[[0,1]]", "homolytic", "[]")).unwrap();
        assert!(
            one.contains("\"formula\":\"C6H12\"")
                && one.contains("\"unpaired\":2")
                && one.contains("\"shift\":[0,0]"),
            "{one}"
        );
        let two = cut_json(&req(&ring, "[[0,1],[3,4]]", "homolytic", "[]")).unwrap();
        assert_eq!(two.matches("\"formula\":\"C3H6\"").count(), 2, "{two}");
    }

    #[test]
    fn existing_radical_and_charge_are_counted() {
        let mf = molfile(
            &[("C", 0.0, 0.0), ("N", 1.0, 0.0)],
            &[(1, 2, 1)],
            &["M  CHG  1   2   1", "M  RAD  1   2   2"],
        );
        let j = cut_json(&req(&mf, "[[0,1]]", "hydrogen", "[]")).unwrap();
        assert!(
            j.contains("\"text\":\"NH3+\"") && j.contains("\"kind\":\"radicalCation\""),
            "{j}"
        );
        let bad = |b: &str, m: &str| cut_json(&req(&mf, b, m, "[]")).unwrap_err().key;
        assert_eq!(bad("[]", "homolytic"), "error.grafo.cut.none");
        assert_eq!(bad("[[0,1],[1,0]]", "homolytic"), "error.grafo.cut.request");
        assert_eq!(bad("[[0,1]]", "sideways"), "error.grafo.cut.mode");
        let ene = molfile(&[("C", 0.0, 0.0), ("C", 1.0, 0.0)], &[(1, 2, 2)], &[]);
        assert_eq!(
            cut_json(&req(&ene, "[[0,1]]", "heterolytic", "[]"))
                .unwrap_err()
                .key,
            "error.grafo.cut.order"
        );
        assert!(cut_json(&req(&ene, "[[0,1]]", "homolytic", "[]"))
            .unwrap()
            .contains("\"radical\":3"));
    }
}
