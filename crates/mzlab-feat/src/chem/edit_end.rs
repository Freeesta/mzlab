//! The end of an edit patch: the record the slots make with the stereo that survived, hydrogens counted again, the atoms without a position placed
//! around the fixed ones, and the wedges of the patch read back by the molfile reader.

use mzlab_core::error::Result;
use mzlab_mol::{allowed_valences, Coords, Mol, Stereo, StereoGroup};

use super::draw::{note, too_big, Drawn};
use super::edit::Ed;
use super::edit_op::{fail, pair};
use super::edit_set::Index;

/// Bonds and atoms a V2000 molfile can count, the only form a wedge round trip is done in.
const V2000_MAX: usize = 999;

/// The stereo elements that survive the patch, renumbered, and the output indices of the ones dropped (`i` for a stereo centre, `i-j` for a
/// double bond) when their atoms are still there.
fn keep_stereo(d: &Drawn, ed: &Ed, ix: &Index) -> (Vec<Stereo>, Vec<String>) {
    let (mut dropped, mut stereo) = (Vec::new(), Vec::new());
    let gone = |x: u32| ed.sets.shape.has(x) || ix.get(x).is_none();
    for s in &d.mol.stereo {
        match s {
            Stereo::Tetra {
                c,
                refs,
                implicit_h,
                par,
                status,
            } => {
                let Some(ci) = ix.get(*c) else { continue };
                let refs2 = refs
                    .as_ref()
                    .map(|r| r.iter().map(|&x| ix.get(x)).collect::<Option<Vec<u32>>>());
                if gone(*c) || matches!(refs2, Some(None)) {
                    dropped.push(ci.to_string());
                    continue;
                }
                stereo.push(Stereo::Tetra {
                    c: ci,
                    refs: refs2.flatten(),
                    implicit_h: *implicit_h,
                    par: *par,
                    status: *status,
                });
            }
            Stereo::Db {
                a,
                b,
                refs,
                cfg,
                status,
            } => {
                let (Some(ia), Some(ib)) = (ix.get(*a), ix.get(*b)) else {
                    continue;
                };
                let refs2 = match refs.map(|r| (ix.get(r[0]), ix.get(r[1]))) {
                    None => Some(None),
                    Some((Some(x), Some(y))) => Some(Some([x, y])),
                    Some(_) => None,
                };
                if gone(*a) || gone(*b) || refs2.is_none() {
                    dropped.push(pair(ia, ib));
                    continue;
                }
                stereo.push(Stereo::Db {
                    a: ia,
                    b: ib,
                    refs: refs2.flatten(),
                    cfg: *cfg,
                    status: *status,
                });
            }
        }
    }
    (stereo, dropped)
}

/// The record the slots and bonds make, with the stereo that survived; the second list is what was dropped.
fn rebuild(d: &Drawn, ed: &Ed, ix: &Index) -> (Mol, Vec<String>) {
    let bonds = ed
        .bonds
        .iter()
        .filter_map(|&(a, b, o)| {
            let (x, y) = (ix.get(a)?, ix.get(b)?);
            Some(mzlab_mol::Bond {
                a: x.min(y),
                b: x.max(y),
                o,
            })
        })
        .collect();
    let (stereo, dropped) = keep_stereo(d, ed, ix);
    let kept = |x: u32| {
        stereo
            .iter()
            .any(|s| matches!(s, Stereo::Tetra { c, .. } if *c == x))
    };
    let groups = d
        .mol
        .groups
        .iter()
        .filter_map(|g| {
            let atoms: Vec<u32> = g
                .atoms
                .iter()
                .filter_map(|&x| ix.get(x))
                .filter(|&x| kept(x))
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

/// Hydrogens counted again, by the lowest allowed valence, for the atoms the patch changed or added and did not give a count for. Returns the
/// ids of the atoms counted again and of the changed atoms whose bonds, radical and hydrogens go over every allowed valence (an element
/// without a valence table is never over).
fn count_h(ed: &mut Ed) -> (Vec<u32>, Vec<u32>) {
    let (mut recounted, mut over) = (Vec::new(), Vec::new());
    for k in 0..ed.slots.len() {
        let id = ed.slots[k].id;
        let new = ed.added.contains(&id);
        if !(new || ed.sets.shape.has(id) || ed.sets.atom.has(id)) {
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
        let allowed = allowed_valences(&a.el, i32::from(a.chg));
        if ed.sets.h_given.has(id) {
            let with_h = total + u32::from(a.h);
            if !allowed.is_empty() && allowed.iter().all(|&v| v < with_h) {
                over.push(id);
            }
            continue;
        }
        let h = match allowed.iter().find(|&&v| v >= total) {
            Some(v) => v - total,
            None => {
                if !allowed.is_empty() {
                    over.push(id);
                }
                0
            }
        };
        ed.slots[k].atom.h = h as u8;
        recounted.push(id);
    }
    (recounted, over)
}

/// The output indices of the atoms `ids` that are still in the record, ascending, as the list a choice carries.
fn out_list(ix: &Index, n: usize, ids: &[u32]) -> String {
    let mut hit = vec![false; n];
    ids.iter().for_each(|&x| {
        if let Some(i) = ix.get(x) {
            hit[i as usize] = true;
        }
    });
    let t: Vec<String> = (0..n).filter(|&i| hit[i]).map(|i| i.to_string()).collect();
    t.join(",")
}

pub(super) fn finish(d: &mut Drawn, mut ed: Ed, n_ops: usize) -> Result<()> {
    let (recounted, over) = count_h(&mut ed);
    let ix = Index::of(&ed.slots);
    let n = ed.slots.len();
    let (mut m, dropped) = rebuild(d, &ed, &ix);
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
    let mut wedge_gone = Vec::new();
    if !ed.wedges.is_empty() {
        (m, wedge_gone) = read_wedges(m, &ed, &ix, d)?;
    }
    let labelled = ed.slots.iter().filter(|s| s.label.is_some()).count();
    if labelled > 0
        && (m.atoms.len() > V2000_MAX || m.bonds.len() > V2000_MAX || !m.groups.is_empty())
    {
        d.warnings
            .push(note("label_not_in_molfile", labelled.to_string()));
    }
    d.labels = ed
        .slots
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.label.clone().map(|l| (i as u32, l)))
        .collect();
    d.mol = m;
    d.choices.push(note("edit", n_ops.to_string()));
    // the lists are output indices (the `i` of the scene atoms), ascending
    for (key, list) in [
        ("edit_added", out_list(&ix, n, &ed.added)),
        ("edit_placed", out_list(&ix, n, &placed)),
        ("edit_h_recounted", out_list(&ix, n, &recounted)),
        ("edit_stereo_dropped", dropped.join(",")),
        ("edit_wedge_dropped", wedge_gone.join(",")),
    ] {
        if !list.is_empty() {
            d.choices.push(note(key, list));
        }
    }
    let over = out_list(&ix, n, &over);
    if !over.is_empty() {
        d.warnings.push(note("edit_valence", over));
    }
    too_big(&d.mol)
}

/// The unsigned number in the columns `r` of a molfile line (0 when there is none).
fn field(l: &str, r: std::ops::Range<usize>) -> usize {
    l.get(r)
        .unwrap_or("")
        .bytes()
        .filter(u8::is_ascii_digit)
        .fold(0, |n, c| n * 10 + usize::from(c - b'0'))
}

/// The record with the wedges of the patch: written as a V2000 molfile with those marks, read back by the reader, which takes the stereo
/// from the marks and the coordinates. Atoms and their order stay, and so do the hydrogen counts of the record. The second list names the
/// bonds (`i-j`, output indices) whose own wedge mark went because a mark of the patch starts at the same atom.
fn read_wedges(m: Mol, ed: &Ed, ix: &Index, d: &Drawn) -> Result<(Mol, Vec<String>)> {
    let last = ed.wedges.last().map_or(0, |w| w.3);
    let bad = |reason, detail: &str| fail(last, "wedge", reason, detail);
    if m.atoms.len() > V2000_MAX || m.bonds.len() > V2000_MAX || !m.groups.is_empty() {
        return Err(bad(
            "wedge_size",
            "a wedge is set on a V2000 molfile: at most 999 atoms and bonds, no stereo groups",
        ));
    }
    let marks: Vec<(usize, usize, i8)> = ed
        .wedges
        .iter()
        .filter_map(|w| Some((ix.get(w.0)? as usize + 1, ix.get(w.1)? as usize + 1, w.2)))
        .collect();
    let text = m.to_molfile(&d.title);
    let lines: Vec<&str> = text.lines().collect();
    let n = lines.get(3).map_or(0, |l| field(l, 0..3));
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let mut gone = Vec::new();
    for l in out.iter_mut().skip(4 + n).take(m.bonds.len()) {
        let f = |r: std::ops::Range<usize>| field(l, r);
        let (a, b) = (f(0..3), f(3..6));
        let mine = marks
            .iter()
            .find(|w| (w.0 == a && w.1 == b) || (w.0 == b && w.1 == a));
        let (a, b, w) = match mine {
            Some(w) => (w.0, w.1, w.2),
            // another mark that starts at an atom the patch sets a wedge on goes: one wedge per set
            None if marks.iter().any(|w| w.0 == a) => {
                if f(9..12) != 0 {
                    gone.push(pair(a as u32 - 1, b as u32 - 1));
                }
                (a, b, 0)
            }
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
        Mol::from_molfile(&(out.join("\n") + "\n")).map_err(|e| bad("molfile", &e.to_string()))?;
    if back.atoms.len() != m.atoms.len()
        || back.atoms.iter().zip(&m.atoms).any(|(x, y)| x.el != y.el)
    {
        return Err(bad(
            "molfile",
            "the atoms changed when the wedges were read",
        ));
    }
    // a molfile has no hydrogen count: the counts of the record (the patch's own included) are kept
    for (x, y) in back.atoms.iter_mut().zip(&m.atoms) {
        x.h = y.h;
    }
    back.coords = m.coords;
    Ok((back, gone))
}
