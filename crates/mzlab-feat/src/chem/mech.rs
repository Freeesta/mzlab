//! `mecc/1`: the curved arrows of a mechanism drawn on a structure, checked for form only. A full-head arrow moves an electron pair, a half-head arrow
//! (fishhook) one electron; each must start at a lone pair, a bond or a radical and end at an atom or a bond, and the drawn lone pairs must fit the
//! electrons the atom has. The check never says which mechanism is right: the student draws it, this says whether every arrow is well formed.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out, Value};

use super::graph::{props, Mol};
use super::labels::{unpaired_of, z_of};

/// Where an arrow starts or ends: atoms are 0-based positions of the molfile.
#[derive(Clone, Copy, PartialEq)]
enum End {
    Lone(usize),
    Radical(usize),
    Atom(usize),
    Bond(usize, usize),
}

impl End {
    fn name(self) -> &'static str {
        match self {
            End::Lone(_) => "lone",
            End::Radical(_) => "radical",
            End::Atom(_) => "atom",
            End::Bond(..) => "bond",
        }
    }
}

struct Arrow {
    pair: bool,
    from: End,
    to: End,
}

/// The request: `{"molfile", "arrows": [{"kind": "pair"|"single", "from": {"lone"|"radical": i} | {"bond": [a, b]}, "to": {"atom": i} | {"bond": [a, b]}}],
/// "lone": [[atom, pairs], ..]}` (the lone pairs drawn on the atoms).
struct Request {
    mf: String,
    arrows: Vec<Arrow>,
    lone: Vec<(usize, u32)>,
}

fn request(text: &str) -> Result<Request> {
    let bad = || Error::new("error.grafo.mech.request");
    let v = json::parse(text.as_bytes()).map_err(|_| bad())?;
    let idx = |x: Option<&Value>| {
        x.and_then(Value::f64)
            .filter(|f| *f >= 0.0 && f.fract() == 0.0)
            .map(|f| f as usize)
            .ok_or_else(bad)
    };
    let end = |e: Option<&Value>, starts: bool| -> Result<End> {
        let e = e.ok_or_else(bad)?;
        let [(k, x)] = e.entries() else {
            return Err(bad());
        };
        Ok(match (k.as_str(), starts) {
            ("lone", true) => End::Lone(idx(Some(x))?),
            ("radical", true) => End::Radical(idx(Some(x))?),
            ("atom", false) => End::Atom(idx(Some(x))?),
            ("bond", _) => End::Bond(idx(x.at(0))?, idx(x.at(1))?),
            _ => return Err(bad()),
        })
    };
    let mut arrows = vec![];
    for a in v.get("arrows").map_or(&[][..], Value::arr) {
        let pair = match a.get("kind").and_then(Value::str) {
            Some("pair") => true,
            Some("single") => false,
            _ => return Err(bad()),
        };
        arrows.push(Arrow {
            pair,
            from: end(a.get("from"), true)?,
            to: end(a.get("to"), false)?,
        });
    }
    let mut lone = vec![];
    for l in v.get("lone").map_or(&[][..], Value::arr) {
        lone.push((idx(l.at(0))?, idx(l.at(1))? as u32));
    }
    Ok(Request {
        mf: v
            .get("molfile")
            .and_then(Value::str)
            .unwrap_or("")
            .to_string(),
        arrows,
        lone,
    })
}

/// Valence electrons of a main-group element (none for a transition metal or an unknown symbol: its lone pairs are not checked).
fn valence(el: &str) -> Option<i64> {
    let z = z_of(el);
    Some(match z {
        1..=2 => z,
        3..=10 => z - 2,
        11..=18 => z - 10,
        19..=20 => z - 18,
        31..=36 => z - 28,
        37..=38 => z - 36,
        49..=54 => z - 46,
        _ => return None,
    })
}

/// The atoms of the drawing as the arrows see them: every bond (explicit H included), unpaired electrons and the lone pairs each atom can hold
/// (None for an element that is not checked).
struct Drawn {
    bonds: Vec<(usize, usize)>,
    unp: Vec<u64>,
    pairs: Vec<Option<u32>>,
}

/// The molfile without its explicit H atoms (the graph reader fills them back by valence on their heavy atom), with every bond of the drawing,
/// the radical codes and, for each drawn atom, its position in the new molfile (None for an H).
struct Heavy {
    mf: String,
    bonds: Vec<(usize, usize)>,
    rad: Vec<(usize, i32)>,
    at: Vec<Option<usize>>,
}

fn heavy(mf: &str) -> Result<Heavy> {
    let bad = || Error::new("error.grafo.molfile");
    let lines: Vec<&str> = mf.lines().collect();
    let num =
        |s: &str, a: usize, b: usize| s.get(a..b).and_then(|t| t.trim().parse::<usize>().ok());
    let counts = lines.get(3).ok_or_else(bad)?;
    let (na, nb) = (
        num(counts, 0, 3).ok_or_else(bad)?,
        num(counts, 3, 6).ok_or_else(bad)?,
    );
    if lines.len() < 4 + na + nb {
        return Err(bad());
    }
    let mut at = vec![None; na];
    let mut n = 0;
    for (i, l) in lines[4..4 + na].iter().enumerate() {
        if l.get(31..34).map(str::trim) != Some("H") {
            at[i] = Some(n);
            n += 1;
        }
    }
    let mut bonds = vec![];
    let mut kept = vec![];
    for l in &lines[4 + na..4 + na + nb] {
        let (a, b) = (num(l, 0, 3).ok_or_else(bad)?, num(l, 3, 6).ok_or_else(bad)?);
        if a < 1 || b < 1 || a > na || b > na {
            return Err(bad());
        }
        bonds.push((a - 1, b - 1));
        if let (Some(x), Some(y)) = (at[a - 1], at[b - 1]) {
            kept.push(format!(
                "{:>3}{:>3}{}",
                x + 1,
                y + 1,
                l.get(6..).unwrap_or("")
            ));
        }
    }
    let tail = &lines[4 + na + nb..];
    let rad = props(tail, "M  RAD", na);
    let mut out = lines[..3].join("\n")
        + &format!(
            "\n{:>3}{:>3}{}\n",
            n,
            kept.len(),
            counts.get(6..).unwrap_or("")
        );
    for i in (0..na).filter(|i| at[*i].is_some()) {
        out += lines[4 + i];
        out += "\n";
    }
    for l in &kept {
        out += l;
        out += "\n";
    }
    for (key, list) in [
        ("M  CHG", props(tail, "M  CHG", na)),
        ("M  RAD", rad.clone()),
        ("M  ISO", props(tail, "M  ISO", na)),
    ] {
        for (i, v) in list {
            if let Some(x) = at[i] {
                out += &format!("{key}  1 {:>3} {v:>3}\n", x + 1);
            }
        }
    }
    Ok(Heavy {
        mf: out + "M  END",
        bonds,
        rad,
        at,
    })
}

/// Reads the molfile (explicit H atoms, such as the γ-H of a McLafferty rearrangement, keep their own place for the arrows and hold no lone pair).
fn drawn(mf: &str) -> Result<Drawn> {
    let h = heavy(mf)?;
    let m = Mol::from_molfile(&h.mf)?;
    let mut unp = vec![0u64; h.at.len()];
    for &(i, c) in &h.rad {
        unp[i] = unpaired_of(c as i64);
    }
    let mut order = vec![0f32; m.atoms.len()];
    for b in &m.bonds {
        order[b.a] += b.order;
        order[b.b] += b.order;
    }
    let pairs = (0..h.at.len())
        .map(|i| match h.at[i] {
            None => Some(0),
            Some(x) => {
                let a = &m.atoms[x];
                valence(&a.el).map(|v| {
                    let nb =
                        v - a.charge as i64 - order[x].round() as i64 - a.h as i64 - unp[i] as i64;
                    (nb.max(0) / 2) as u32
                })
            }
        })
        .collect();
    Ok(Drawn {
        bonds: h.bonds,
        unp,
        pairs,
    })
}

/// The first reason an arrow is not well formed, or None.
fn problem(d: &Drawn, a: &Arrow) -> Option<&'static str> {
    let bond = |x: usize, y: usize| d.bonds.iter().any(|b| *b == (x, y) || *b == (y, x));
    match a.from {
        End::Lone(i) if d.pairs[i] == Some(0) => return Some("noLonePair"),
        End::Radical(i) if d.unp[i] == 0 => return Some("noRadical"),
        End::Radical(_) if a.pair => return Some("pairFromRadical"),
        End::Bond(x, y) if !bond(x, y) => return Some("noBond"),
        _ => {}
    }
    match (a.from, a.to) {
        (_, End::Bond(x, y)) if !bond(x, y) => Some("noBond"),
        (End::Lone(i) | End::Radical(i), End::Atom(j)) if i == j => Some("same"),
        (End::Bond(a1, b1), End::Bond(a2, b2)) if (a1, b1) == (a2, b2) || (a1, b1) == (b2, a2) => {
            Some("same")
        }
        _ => None,
    }
}

/// `mecc/1` of a request (see `Request`): `{"format","arrows":[{kind, from, to, electrons, ok, problem?}],"lone":[{atom, pairs, max?, ok}],"ok"}`. `from`
/// and `to` name what the arrow starts and ends at; `problem` is a code (noLonePair, noRadical, pairFromRadical, noBond, same); `max` is the number of
/// lone pairs the atom can hold, absent for an element that is not checked. Form only: no arrow is said to be the right one.
pub fn mech_json(text: &str) -> Result<String> {
    let r = request(text)?;
    let d = drawn(&r.mf)?;
    let n = d.unp.len();
    let atoms = |e: End| match e {
        End::Bond(x, y) => vec![x, y],
        End::Lone(i) | End::Radical(i) | End::Atom(i) => vec![i],
    };
    if r.arrows
        .iter()
        .flat_map(|a| [atoms(a.from), atoms(a.to)].concat())
        .chain(r.lone.iter().map(|l| l.0))
        .any(|i| i >= n)
    {
        return Err(Error::new("error.grafo.mech.atom"));
    }
    let mut ok = true;
    let mut o = Out::default();
    o.raw("{\"format\":\"mecc/1\",\"arrows\":[");
    for (k, a) in r.arrows.iter().enumerate() {
        let p = problem(&d, a);
        ok &= p.is_none();
        o.raw(if k > 0 { ",{\"kind\":" } else { "{\"kind\":" })
            .str(if a.pair { "pair" } else { "single" })
            .raw(",\"from\":")
            .str(a.from.name())
            .raw(",\"to\":")
            .str(a.to.name())
            .raw(",\"electrons\":")
            .int(if a.pair { 2 } else { 1 })
            .raw(",\"ok\":")
            .raw(if p.is_none() { "true" } else { "false" });
        if let Some(p) = p {
            o.raw(",\"problem\":").str(p);
        }
        o.raw("}");
    }
    o.raw("],\"lone\":[");
    for (k, &(i, drawn)) in r.lone.iter().enumerate() {
        let fits = d.pairs[i].is_none_or(|mx| drawn <= mx);
        ok &= fits;
        o.raw(if k > 0 { ",{\"atom\":" } else { "{\"atom\":" })
            .int(i as i64)
            .raw(",\"pairs\":")
            .int(drawn as i64);
        if let Some(mx) = d.pairs[i] {
            o.raw(",\"max\":").int(mx as i64);
        }
        o.raw(",\"ok\":")
            .raw(if fits { "true" } else { "false" })
            .raw("}");
    }
    o.raw("],\"ok\":")
        .raw(if ok { "true" } else { "false" })
        .raw("}");
    Ok(o.0)
}

#[cfg(test)]
#[path = "mech_tests.rs"]
mod tests;
