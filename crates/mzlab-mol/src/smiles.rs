//! SMILES (and the enhanced-stereo groups of CXSMILES) into `mol/1`. Atoms keep the order of the text; aromatic bonds are kekulised;
//! hydrogens of atoms outside brackets come from the valence rules; `@`, `@@`, `/` and `\` become `defined` stereo elements.
//! Not read: whether an unmarked atom or double bond could be stereogenic (a later package decides), so no `unknown` element is made here.

use crate::codec::MolError;
use crate::cxsmiles;
use crate::kekule::kekulize;
use crate::model::{Atom, Bond, Mol, Status, Stereo, StereoGroup};
use crate::smiles_parse::{parse, Raw, RawBond, Sym};
use crate::smiles_stereo::{double, tetra};
use crate::valence::{needs_double, shortfall};

impl Mol {
    /// Reads a SMILES with an optional CXSMILES tail (`CC[C@H](C)O |&1:2|`). Explicit `[H]` atoms stay atoms.
    pub fn from_smiles(text: &str) -> Result<Mol, MolError> {
        let (smiles, tail) = cxsmiles::split(text);
        let raw = parse(smiles)?;
        let (orders, h) = resolve(&raw)?;
        let atoms = atoms(&raw, &orders, &h);
        let mut bonds: Vec<Bond> = raw
            .bonds
            .iter()
            .zip(&orders)
            .map(|(e, &o)| Bond {
                a: e.from.min(e.to) as u32,
                b: e.from.max(e.to) as u32,
                o,
            })
            .collect();
        bonds.sort_by_key(|b| (b.a, b.b));
        let mut stereo = tetra(&raw, &h);
        stereo.extend(double(&raw, &orders));
        let groups = groups(cxsmiles::groups(tail), &stereo, atoms.len())?;
        let m = Mol::new(atoms, bonds, stereo, groups);
        m.validate()?;
        Ok(m)
    }
}

/// Bond order as written; 0 stands for an aromatic bond that still has to be kekulised.
fn written_order(raw: &Raw, e: &RawBond) -> u8 {
    match e.sym {
        Sym::Double => 2,
        Sym::Triple => 3,
        Sym::Single | Sym::Up | Sym::Down => 1,
        Sym::Aromatic => 0,
        Sym::None => u8::from(!(raw.atoms[e.from].aromatic && raw.atoms[e.to].aromatic)),
    }
}

/// Final bond orders (aromatic bonds kekulised) and the hydrogens of every atom.
fn resolve(raw: &Raw) -> Result<(Vec<u8>, Vec<u8>), MolError> {
    let n = raw.atoms.len();
    let mut orders: Vec<u8> = raw.bonds.iter().map(|e| written_order(raw, e)).collect();
    let aromatic: Vec<usize> = (0..orders.len()).filter(|&i| orders[i] == 0).collect();
    if !aromatic.is_empty() {
        let mut total = vec![0u32; n];
        let mut in_ring = vec![false; n];
        for (i, e) in raw.bonds.iter().enumerate() {
            let o = u32::from(orders[i].max(1));
            total[e.from] += o;
            total[e.to] += o;
            if orders[i] == 0 {
                in_ring[e.from] = true;
                in_ring[e.to] = true;
            }
        }
        let needy: Vec<bool> = (0..n)
            .map(|i| {
                let a = &raw.atoms[i];
                in_ring[i] && needs_double(&a.el, i32::from(a.chg), total[i] + u32::from(a.h))
            })
            .collect();
        let pairs: Vec<(usize, usize)> = aromatic
            .iter()
            .map(|&i| (raw.bonds[i].from, raw.bonds[i].to))
            .collect();
        let Some(doubles) = kekulize(n, &pairs, &needy) else {
            return Err(MolError::Smiles(
                "the aromatic system cannot be kekulised".into(),
            ));
        };
        for &i in &aromatic {
            let (f, t) = (raw.bonds[i].from, raw.bonds[i].to);
            let double = doubles.contains(&(f.min(t), f.max(t)));
            orders[i] = if double { 2 } else { 1 };
        }
    }
    let mut sum = vec![0u32; n];
    for (e, &o) in raw.bonds.iter().zip(&orders) {
        sum[e.from] += u32::from(o);
        sum[e.to] += u32::from(o);
    }
    let h = (0..n)
        .map(|i| {
            let a = &raw.atoms[i];
            if a.bracket {
                a.h
            } else {
                shortfall(&a.el, 0, sum[i]) as u8
            }
        })
        .collect();
    Ok((orders, h))
}

fn atoms(raw: &Raw, orders: &[u8], h: &[u8]) -> Vec<Atom> {
    let mut sum = vec![0u32; raw.atoms.len()];
    for (e, &o) in raw.bonds.iter().zip(orders) {
        sum[e.from] += u32::from(o);
        sum[e.to] += u32::from(o);
    }
    raw.atoms
        .iter()
        .enumerate()
        .map(|(i, a)| Atom {
            el: a.el.clone(),
            iso: a.iso,
            chg: a.chg,
            // a bracket atom with fewer bonds than its valence allows is a radical; outside brackets hydrogens fill the valence
            rad: if a.bracket {
                shortfall(&a.el, i32::from(a.chg), sum[i] + u32::from(h[i])) as u8
            } else {
                0
            },
            h: h[i],
        })
        .collect()
}

/// Enhanced-stereo groups of the tail: every listed atom must carry a defined tetrahedral mark and sit in one group only.
fn groups(
    listed: Vec<cxsmiles::CxGroup>,
    stereo: &[Stereo],
    n: usize,
) -> Result<Vec<StereoGroup>, MolError> {
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for (kind, _, mut atoms) in listed {
        atoms.sort_unstable();
        atoms.dedup();
        for &a in &atoms {
            let defined = stereo
                .iter()
                .any(|s| matches!(s, Stereo::Tetra { c, status: Status::Defined, .. } if *c == a));
            if a as usize >= n || !defined {
                return Err(MolError::Smiles(format!(
                    "stereo group lists atom {a}, which is not a marked tetrahedral centre"
                )));
            }
            if seen.contains(&a) {
                return Err(MolError::Smiles(format!(
                    "atom {a} is in two stereo groups"
                )));
            }
            seen.push(a);
        }
        if !atoms.is_empty() {
            out.push(StereoGroup { kind, atoms });
        }
    }
    Ok(out)
}
