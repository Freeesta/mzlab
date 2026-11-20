//! The end of an edit patch: the record the slots make with the stereo that survived, hydrogens counted again, the atoms without a position placed
//! around the fixed ones, and the wedges of the patch read back by the molfile reader.

use mzlab_core::error::Result;
use mzlab_mol::{allowed_valences, Coords, Mol, Stereo, StereoGroup};
use std::collections::{BTreeMap, BTreeSet};

use super::draw::{note, too_big, Drawn};
use super::edit::Ed;
use super::edit_op::fail;

/// Bonds and atoms a V2000 molfile can count, the only form a wedge round trip is done in.
const V2000_MAX: usize = 999;

/// The stereo elements that survive the patch, renumbered, and the ids of the ones dropped.
fn keep_stereo(d: &Drawn, ed: &Ed, at: &BTreeMap<u32, u32>) -> (Vec<Stereo>, Vec<String>) {
    let (mut dropped, mut stereo) = (Vec::new(), Vec::new());
    let hit = |x: u32| ed.sets.shape.contains(&x) || !at.contains_key(&x);
    for s in &d.mol.stereo {
        let map = |x: &u32| at.get(x).copied();
        match s {
            Stereo::Tetra {
                c,
                refs,
                implicit_h,
                par,
                status,
            } => {
                let refs2: Option<Option<Vec<u32>>> =
                    refs.as_ref().map(|r| r.iter().map(map).collect());
                match (hit(*c), refs2) {
                    (false, Some(None)) | (true, _) => dropped.push(c.to_string()),
                    (false, r) => stereo.push(Stereo::Tetra {
                        c: at[c],
                        refs: r.flatten(),
                        implicit_h: *implicit_h,
                        par: *par,
                        status: *status,
                    }),
                }
            }
            Stereo::Db {
                a,
                b,
                refs,
                cfg,
                status,
            } => {
                let refs2 = refs.map(|r| (map(&r[0]), map(&r[1])));
                match (hit(*a) || hit(*b), refs2) {
                    (false, Some((Some(x), Some(y)))) => stereo.push(Stereo::Db {
                        a: at[a],
                        b: at[b],
                        refs: Some([x, y]),
                        cfg: *cfg,
                        status: *status,
                    }),
                    (false, None) => stereo.push(Stereo::Db {
                        a: at[a],
                        b: at[b],
                        refs: None,
                        cfg: *cfg,
                        status: *status,
                    }),
                    _ => dropped.push(format!("{a}-{b}")),
                }
            }
        }
    }
    (stereo, dropped)
}

/// The record the slots and bonds make, with the stereo that survived; the second list is what was dropped.
fn rebuild(d: &Drawn, ed: &Ed) -> (Mol, Vec<String>) {
    let at: BTreeMap<u32, u32> = ed
        .slots
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id, i as u32))
        .collect();
    let bonds = ed
        .bonds
        .iter()
        .map(|&(a, b, o)| {
            let (x, y) = (at[&a], at[&b]);
            mzlab_mol::Bond {
                a: x.min(y),
                b: x.max(y),
                o,
            }
        })
        .collect();
    let (stereo, dropped) = keep_stereo(d, ed, &at);
    let kept: BTreeSet<u32> = stereo
        .iter()
        .filter_map(|s| {
            if let Stereo::Tetra { c, .. } = s {
                Some(*c)
            } else {
                None
            }
        })
        .collect();
    let groups = d
        .mol
        .groups
        .iter()
        .filter_map(|g| {
            let atoms: Vec<u32> = g
                .atoms
                .iter()
                .filter_map(|x| at.get(x))
                .copied()
                .filter(|x| kept.contains(x))
                .collect();
            (!atoms.is_empty()).then_some(StereoGroup {
                kind: g.kind,
                atoms,
            })
        })
        .collect();
    let mut m = Mol::new(
        ed.slots.iter().map(|s| s.atom.clone()).collect(),
        bonds,
        stereo,
        groups,
    );
    m.coords = None;
    (m, dropped)
}

/// Hydrogens counted again, by the lowest allowed valence, for the atoms the patch changed or added and did not give a count for.
fn count_h(ed: &mut Ed) {
    for k in 0..ed.slots.len() {
        let id = ed.slots[k].id;
        let new = ed.added.contains(&id);
        let dirty = ed.sets.shape.contains(&id) || ed.sets.atom.contains(&id) || new;
        if !dirty || ed.sets.h_given.contains(&id) {
            continue;
        }
        let a = &ed.slots[k].atom;
        let total: u32 = ed
            .bonds
            .iter()
            .filter(|b| b.0 == id || b.1 == id)
            .map(|b| u32::from(b.2))
            .sum::<u32>()
            + u32::from(a.rad);
        let h = allowed_valences(&a.el, i32::from(a.chg))
            .into_iter()
            .find(|&v| v >= total)
            .map_or(0, |v| v - total);
        ed.slots[k].atom.h = h as u8;
    }
}

pub(super) fn finish(d: &mut Drawn, mut ed: Ed, n_ops: usize) -> Result<()> {
    count_h(&mut ed);
    let (mut m, dropped) = rebuild(d, &ed);
    // positions: the ones the drawing had or the patch gave stay; the atoms without one are placed around them
    let mut xy: Vec<Option<[f64; 2]>> = ed.slots.iter().map(|s| s.xy).collect();
    let placed: Vec<u32> = ed
        .slots
        .iter()
        .zip(&xy)
        .filter(|(_, p)| p.is_none())
        .map(|(s, _)| s.id)
        .collect();
    if !placed.is_empty() {
        let lay = m.layout_2d_fixed(&xy);
        for (p, r) in xy.iter_mut().zip(&lay.xy) {
            p.get_or_insert([r[0], r[1]]);
        }
    }
    let from = d
        .mol
        .coords
        .as_ref()
        .map_or("layout", |c| c.from.as_str())
        .to_string();
    m.coords = Some(Coords {
        dim: 2,
        xy: xy
            .iter()
            .map(|p| p.map_or(vec![0.0, 0.0], |p| vec![p[0], p[1]]))
            .collect(),
        from,
    });
    if !ed.wedges.is_empty() {
        m = read_wedges(m, &ed, d)?;
    }
    if ed.slots.iter().any(|s| s.label.is_some())
        && (m.atoms.len() > V2000_MAX || m.bonds.len() > V2000_MAX || !m.groups.is_empty())
    {
        d.warnings.push(note(
            "label_not_in_molfile",
            "labels are written to a V2000 molfile only",
        ));
    }
    d.labels = ed
        .slots
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.label.clone().map(|l| (i as u32, l)))
        .collect();
    d.mol = m;
    d.choices
        .push(note("edit", format!("{n_ops} operations applied")));
    let list = |v: &[u32]| v.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    if !ed.added.is_empty() {
        d.choices
            .push(note("edit_added", format!("atom ids {}", list(&ed.added))));
    }
    if !placed.is_empty() {
        d.choices.push(note(
            "edit_placed",
            format!("atom ids {} laid out around the fixed atoms", list(&placed)),
        ));
    }
    if !dropped.is_empty() {
        d.choices.push(note(
            "edit_stereo_dropped",
            format!("stereo of {} dropped: its bonds changed", dropped.join(",")),
        ));
    }
    too_big(&d.mol)
}

/// The record with the wedges of the patch: written as a V2000 molfile with those marks, read back by the reader, which takes the stereo
/// from the marks and the coordinates. Atoms and their order stay.
fn read_wedges(m: Mol, ed: &Ed, d: &Drawn) -> Result<Mol> {
    let at: BTreeMap<u32, u32> = ed
        .slots
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id, i as u32))
        .collect();
    let last = ed.wedges.last().map_or(0, |w| w.3);
    let bad = |reason, detail: String| fail(last, "wedge", reason, detail);
    if m.atoms.len() > V2000_MAX || m.bonds.len() > V2000_MAX || !m.groups.is_empty() {
        return Err(bad(
            "wedge_size",
            "a wedge is set on a V2000 molfile: at most 999 atoms and bonds, no stereo groups"
                .into(),
        ));
    }
    let marks: Vec<(usize, usize, i8)> = ed
        .wedges
        .iter()
        .map(|w| (at[&w.0] as usize + 1, at[&w.1] as usize + 1, w.2))
        .collect();
    let text = m.to_molfile(&d.title);
    let lines: Vec<&str> = text.lines().collect();
    let n: usize = lines
        .get(3)
        .and_then(|l| l.get(0..3))
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    for l in out.iter_mut().skip(4 + n).take(m.bonds.len()) {
        let f = |r: std::ops::Range<usize>| {
            l.get(r)
                .and_then(|s| s.trim().parse::<usize>().ok())
                .unwrap_or(0)
        };
        let (a, b) = (f(0..3), f(3..6));
        let mine = marks
            .iter()
            .find(|w| (w.0 == a && w.1 == b) || (w.0 == b && w.1 == a));
        let (a, b, w) = match mine {
            Some(w) => (w.0, w.1, w.2),
            // another mark that starts at an atom the patch sets a wedge on goes: one wedge per set
            None if marks.iter().any(|w| w.0 == a) => (a, b, 0),
            None => continue,
        };
        let code = match w {
            1 => 1,
            -1 => 6,
            2 => 4,
            _ => 0,
        };
        *l = format!(
            "{a:>3}{b:>3}{:>3}{code:>3}{}",
            f(6..9),
            l.get(12..).unwrap_or("")
        );
    }
    let mut back =
        Mol::from_molfile(&(out.join("\n") + "\n")).map_err(|e| bad("molfile", e.to_string()))?;
    if back.atoms.len() != m.atoms.len()
        || back.atoms.iter().zip(&m.atoms).any(|(x, y)| x.el != y.el)
    {
        return Err(bad(
            "molfile",
            "the atoms changed when the wedges were read".into(),
        ));
    }
    back.coords = m.coords;
    Ok(back)
}
