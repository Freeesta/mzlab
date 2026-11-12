//! `cip/1`: the CIP descriptors of the drawn stereo (R/S of tetrahedral centres, r/s when only R against S tells the ligands apart, E/Z of
//! double bonds), from the molfile of the canvas. Only defined elements on a stereo site get a label; a unit whose ligands the rules cannot
//! tell apart gets none. Descriptors of the other units feed rules 3 to 5 in two passes. Same bytes in the molkit `.wasm`, the app
//! (`feat/grafo`, `kind: "cip"`) and `mzlab-cli mol-cip`.

use mzlab_core::elements;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out};
use mzlab_mol::{atomic_number, Cfg, Mol, Parity, Site, Status, Stereo};

use super::cip_rank::{handed, Centre, Chem, Digraph, NO_ATOM, RULES};

/// Above this many atoms no label is given (small molecules only).
const MAX_ATOMS: usize = 400;

fn chem_of(m: &Mol) -> Chem {
    let n = m.atoms.len();
    let z: Vec<u16> = m
        .atoms
        .iter()
        .map(|a| atomic_number(&a.el).unwrap_or(0))
        .collect();
    let mut sum = vec![0u32; n];
    for b in &m.bonds {
        sum[b.a as usize] += u32::from(b.o);
        sum[b.b as usize] += u32::from(b.o);
    }
    // A multiple bond of P, S, As, Se or Te past its lowest valence (P=O, S=O) is read as single, without duplicates: the ylide form.
    let expanded = |i: usize| match z[i] {
        15 | 33 => sum[i] + u32::from(m.atoms[i].h) > 3,
        16 | 34 | 52 => sum[i] + u32::from(m.atoms[i].h) > 2,
        _ => false,
    };
    let mut adj = vec![Vec::new(); n];
    for b in &m.bonds {
        let (x, y) = (b.a as usize, b.b as usize);
        let o = if expanded(x) || expanded(y) { 1 } else { b.o };
        adj[x].push((y, o));
        adj[y].push((x, o));
    }
    let mass: Vec<u16> = m
        .atoms
        .iter()
        .map(|a| match a.iso {
            0 => elements::nuclide(&a.el, None).map_or(0, |p| 2 * p.0 + 1),
            i => 2 * i,
        })
        .collect();
    Chem {
        manc: mancude(m, &z, &mass),
        z,
        mass,
        h: m.atoms.iter().map(|a| a.h).collect(),
        adj,
        db: Vec::new(),
        tetra: vec![None; n],
    }
}

/// Whether bond `k` (atoms `x`, `y`) lies in a ring: `y` is still reached from `x` without it.
fn in_ring(m: &Mol, k: usize, x: usize, y: usize) -> bool {
    let mut seen = vec![false; m.atoms.len()];
    let mut stack = vec![x];
    seen[x] = true;
    while let Some(a) = stack.pop() {
        for (j, b) in m.bonds.iter().enumerate() {
            let (p, q) = (b.a as usize, b.b as usize);
            let o = if p == a {
                q
            } else if q == a {
                p
            } else {
                continue;
            };
            if j != k && !seen[o] {
                seen[o] = true;
                stack.push(o);
            }
        }
    }
    seen[y]
}

/// Mancude rings (every ring atom with one double bond in the ring): for such an atom with at least two ring neighbours of the same kind,
/// its double-bond partner among them, the mean atomic number (times 12) of those neighbours and their common mass (0 when they differ),
/// what the duplicate of its double bond carries whichever Kekulé form was drawn.
fn mancude(m: &Mol, z: &[u16], mass: &[u16]) -> Vec<Option<(u16, u16)>> {
    let n = m.atoms.len();
    let ring: Vec<bool> = m
        .bonds
        .iter()
        .enumerate()
        .map(|(k, b)| in_ring(m, k, b.a as usize, b.b as usize))
        .collect();
    let (mut doubles, mut ring_double) = (vec![0u8; n], vec![false; n]);
    for (b, &r) in m.bonds.iter().zip(&ring) {
        if b.o == 2 {
            for x in [b.a as usize, b.b as usize] {
                doubles[x] += 1;
                ring_double[x] |= r;
            }
        }
    }
    let ok: Vec<bool> = (0..n).map(|x| doubles[x] == 1 && ring_double[x]).collect();
    let (mut partners, mut double_to) = (vec![Vec::new(); n], vec![usize::MAX; n]);
    for (b, &r) in m.bonds.iter().zip(&ring) {
        let (x, y) = (b.a as usize, b.b as usize);
        if r && ok[x] && ok[y] {
            partners[x].push(y);
            partners[y].push(x);
            if b.o == 2 {
                (double_to[x], double_to[y]) = (y, x);
            }
        }
    }
    partners
        .iter()
        .enumerate()
        .map(|(x, p)| {
            if p.len() < 2 || !p.contains(&double_to[x]) {
                return None;
            }
            let sum: usize = p.iter().map(|&y| usize::from(z[y])).sum();
            let same = p.iter().all(|&y| mass[y] == mass[p[0]]);
            Some((
                (12 * sum / p.len()) as u16,
                if same { mass[p[0]] } else { 0 },
            ))
        })
        .collect()
}

/// The centre as the rules read it, from a defined tetrahedral element.
fn centre_of(s: &Stereo) -> Option<(usize, Centre)> {
    let Stereo::Tetra {
        c,
        refs: Some(refs),
        implicit_h,
        par: Some(par),
        ..
    } = s
    else {
        return None;
    };
    let refs: Vec<usize> = refs.iter().map(|&r| r as usize).collect();
    let extra = (refs.len() == 3).then_some(u16::from(*implicit_h == Some(true)));
    Some((
        *c as usize,
        Centre {
            refs,
            extra,
            cw: *par == Parity::Cw,
        },
    ))
}

/// Ligands ranked highest first, and whether rule 5 was needed to part two of them; `None` when two stay tied or the digraph grew too big.
fn rank(g: &mut Digraph, lig: &[usize], upto: usize) -> Option<(Vec<usize>, bool)> {
    let mut sorted = lig.to_vec();
    if !g.sort_under(0, &mut sorted, upto, 0) || g.over {
        return None;
    }
    let pseudo = upto >= RULES - 1 && !g.sort_under(0, &mut sorted.clone(), RULES - 2, 0);
    Some((
        sorted
            .iter()
            .map(|k| lig.iter().position(|l| l == k).unwrap_or(0))
            .collect(),
        pseudo,
    ))
}

/// R/S (or r/s) of a defined centre: 2 for R or r, 1 for S or s, with whether it is the lowercase kind.
fn centre_label(c: &Chem, at: usize, upto: usize) -> Option<(u8, bool)> {
    let cen = c.tetra[at].as_ref()?;
    let mut g = Digraph::new(c, at);
    let lig = g.ligands(&cen.refs, cen.extra);
    let (order, pseudo) = rank(&mut g, &lig, upto)?;
    Some((handed(&order, cen.cw), pseudo))
}

/// The highest-ranked substituent of `end` other than `other` (`NO_ATOM` for its hydrogen), `None` when its two are tied.
fn top_of(c: &Chem, end: usize, other: usize, upto: usize) -> Option<usize> {
    let refs: Vec<usize> = c.adj[end]
        .iter()
        .map(|p| p.0)
        .filter(|&y| y != other)
        .collect();
    let extra = (c.h[end] > 0).then_some(1);
    if refs.len() + usize::from(extra.is_some()) < 2 {
        return refs.first().copied();
    }
    let mut g = Digraph::new(c, end);
    let lig = g.ligands(&refs, extra);
    let (order, _) = rank(&mut g, &lig, upto)?;
    Some(refs.get(order[0]).copied().unwrap_or(NO_ATOM))
}

/// E/Z of a defined double bond: 2 for Z, 1 for E.
fn bond_label(c: &Chem, s: &Stereo, upto: usize) -> Option<u8> {
    let Stereo::Db {
        a,
        b,
        refs: Some([ra, rb]),
        cfg: Some(cfg),
        ..
    } = s
    else {
        return None;
    };
    let (a, b) = (*a as usize, *b as usize);
    let ta = top_of(c, a, b, upto)?;
    let tb = top_of(c, b, a, upto)?;
    let same = (ta == *ra as usize) == (tb == *rb as usize);
    let cis = (*cfg == Cfg::Cis) == same;
    Some(if cis { 2 } else { 1 })
}

/// Labels of every defined unit: centres as (atom, value, lowercase), bonds as (a, b, value). The E/Z of the double bonds feed rule 3
/// (found first by rules 1a to 2, then again with rule 3); auxiliary R/S for rule 5 come from each digraph itself.
type Labels = (Vec<(u32, u8, bool)>, Vec<(usize, usize, u8)>);

fn labels(m: &Mol, units: &[&Stereo]) -> Labels {
    let mut c = chem_of(m);
    for s in units {
        if let Some((at, cen)) = centre_of(s) {
            c.tetra[at] = Some(cen);
        }
    }
    let bonds = |c: &Chem, upto: usize| -> Vec<(usize, usize, u8)> {
        units
            .iter()
            .filter_map(|s| match s {
                Stereo::Db { a, b, .. } => {
                    bond_label(c, s, upto).map(|v| (*a as usize, *b as usize, v))
                }
                Stereo::Tetra { .. } => None,
            })
            .collect()
    };
    for upto in [2, 3] {
        c.db = bonds(&c, upto);
    }
    let centres = units
        .iter()
        .filter_map(|s| match s {
            Stereo::Tetra { c: at, .. } => {
                centre_label(&c, *at as usize, RULES - 1).map(|(v, p)| (*at, v, p))
            }
            Stereo::Db { .. } => None,
        })
        .collect();
    (centres, bonds(&c, RULES - 1))
}

/// CIP descriptors of a record: `R`/`S`/`r`/`s` per centre and `E`/`Z` per double bond, for the defined elements that sit on one of `sites`.
/// Every defined element feeds the rules, as the record gives it.
pub struct CipLabels {
    pub atoms: Vec<(u32, &'static str)>,
    pub bonds: Vec<(u32, u32, &'static str)>,
}

pub fn cip_labels(m: &Mol, sites: &[Site]) -> CipLabels {
    let units: Vec<&Stereo> = m
        .stereo
        .iter()
        .filter(|s| {
            matches!(
                s,
                Stereo::Tetra {
                    status: Status::Defined,
                    ..
                } | Stereo::Db {
                    status: Status::Defined,
                    ..
                }
            )
        })
        .collect();
    let (centres, bonds) = labels(m, &units);
    // by value (1 S or E, 2 R or Z) and case
    const NAMES: [&str; 6] = ["S", "R", "s", "r", "E", "Z"];
    let mut l = CipLabels {
        atoms: Vec::new(),
        bonds: Vec::new(),
    };
    for (at, v, low) in centres {
        if sites.contains(&Site::Tetra(at)) {
            l.atoms
                .push((at, NAMES[usize::from(v - 1) + 2 * usize::from(low)]));
        }
    }
    for (a, b, v) in bonds {
        let (a, b) = (a as u32, b as u32);
        if sites.contains(&Site::Db(a, b)) {
            l.bonds.push((a, b, NAMES[usize::from(v + 3)]));
        }
    }
    l
}

/// `cip/1` of a request `{"molfile": "..."}`: `skip` (`query`, `kekule`, `tooBig`) says why a drawing gets no labels at all.
pub fn cip_json(request: &str) -> Result<String> {
    let v = json::parse(request.as_bytes()).map_err(|_| Error::new("error.cip.request"))?;
    let mf = v
        .get("molfile")
        .and_then(json::Value::str)
        .ok_or_else(|| Error::new("error.cip.request"))?;
    let d = Mol::drawing_from_molfile(mf).map_err(|_| Error::new("error.cip.molfile"))?;
    let skip = if d.mol.atoms.len() > MAX_ATOMS {
        Some("tooBig")
    } else if !d.query_atoms.is_empty() || !d.query_bonds.is_empty() {
        Some("query")
    } else if !d.kekule {
        Some("kekule")
    } else {
        None
    };
    let l = match skip {
        None => cip_labels(&d.mol, &d.sites),
        Some(_) => CipLabels {
            atoms: Vec::new(),
            bonds: Vec::new(),
        },
    };
    let mut o = Out::default();
    o.raw("{\"format\":\"cip/1\",\"atoms\":[");
    for (i, (at, label)) in l.atoms.iter().enumerate() {
        o.raw(if i > 0 { ",{\"atom\":" } else { "{\"atom\":" });
        o.int(i64::from(*at)).raw(",\"label\":").str(label).raw("}");
    }
    o.raw("],\"bonds\":[");
    for (i, (a, b, label)) in l.bonds.iter().enumerate() {
        o.raw(if i > 0 { ",{\"a\":" } else { "{\"a\":" });
        o.int(i64::from(*a)).raw(",\"b\":").int(i64::from(*b));
        o.raw(",\"label\":").str(label).raw("}");
    }
    o.raw("]");
    if let Some(s) = skip {
        o.raw(",\"skip\":").str(s);
    }
    o.raw("}");
    Ok(o.0)
}
