//! Enhanced stereo groups in an edit patch: the `stereo.group` operation, the group label an atom carries while the patch runs (a literal
//! number, as `and2`), and the groups of the record rebuilt at the end on the centres still defined, numbered again per kind in order.
//! The numbering is the one of the CXSMILES (`&n`, `on`) and V3000 (`STERACn`, `STERELn`) writers: the n-th group of its kind in the list.

use mzlab_core::error::Result;
use mzlab_core::json::Out;
use mzlab_mol::{GroupKind, Mol, Status, Stereo, StereoGroup};

use super::draw::{Drawn, MAX_ATOMS};
use super::edit::Ed;
use super::edit_op::{clip, fail, Op};
use super::edit_set::sort64;

/// A group as its kind and its number among the groups of that kind (from 1; a patch's `abs` is 1).
pub(super) type Label = (GroupKind, u32);

fn kind_of(r: u64) -> GroupKind {
    match r {
        0 => GroupKind::Abs,
        1 => GroupKind::And,
        _ => GroupKind::Or,
    }
}

fn rank(k: GroupKind) -> u32 {
    match k {
        GroupKind::Abs => 0,
        GroupKind::And => 1,
        GroupKind::Or => 2,
    }
}

/// The label of each atom of the record, in the writers' numbering; the second list is the labels in the order of the groups.
pub(super) fn labels(m: &Mol) -> (Vec<Option<Label>>, Vec<Label>) {
    let (mut of, mut order) = (vec![None; m.atoms.len()], Vec::new());
    let mut n = [0u32; 3];
    for g in &m.groups {
        let k = rank(g.kind) as usize;
        n[k] += 1;
        order.push((g.kind, n[k]));
        for &a in &g.atoms {
            if let Some(x) = of.get_mut(a as usize) {
                *x = Some((g.kind, n[k]));
            }
        }
    }
    (of, order)
}

/// `abs`, `and<n>` or `or<n>`: the `stg` of a scene atom and the `g` of the operation.
pub(super) fn name(l: Label) -> String {
    match l.0 {
        GroupKind::Abs => "abs".to_string(),
        GroupKind::And => format!("and{}", l.1),
        GroupKind::Or => format!("or{}", l.1),
    }
}

/// The `label` (set by a patch) and the `stg` of atom `i` in the scene, when it has them; `stg` holds the labels of `labels`.
pub(super) fn put_atom(o: &mut Out, d: &Drawn, i: usize, stg: &[Option<Label>]) {
    if let Some(l) = d.label_at(i) {
        o.raw(",\"label\":").str(l);
    }
    if let Some(l) = stg.get(i).copied().flatten() {
        o.raw(",\"stg\":").str(&name(l));
    }
}

/// The `g` of the operation: `none` (no group) or a label, a number from 1 without leading zeros; `None` when it is neither.
fn parse(g: &str) -> Option<Option<Label>> {
    let (kind, n) = match g {
        "none" => return Some(None),
        "abs" => return Some(Some((GroupKind::Abs, 1))),
        _ => match (g.strip_prefix("and"), g.strip_prefix("or")) {
            (Some(n), _) => (GroupKind::And, n),
            (_, Some(n)) => (GroupKind::Or, n),
            _ => return None,
        },
    };
    let digits = !n.is_empty() && n.len() <= 4 && n.bytes().all(|c| c.is_ascii_digit());
    let n: u32 = n.parse().ok().filter(|_| digits && !n.starts_with('0'))?;
    (n as usize <= MAX_ATOMS).then_some(Some((kind, n)))
}

impl Ed {
    /// `stereo.group`: the atoms go into the group `g` or out of any (`none`); whether they are stereocentres is checked at the end.
    #[inline(never)]
    pub(super) fn stereo_group(&mut self, op: &Op) -> Result<()> {
        op.only(&["atoms", "g"])?;
        let atoms = op.ids("atoms")?;
        if atoms.is_empty() {
            return Err(op.err("missing_field", "atoms"));
        }
        let g = op.text("g")?.ok_or_else(|| op.err("missing_field", "g"))?;
        let l = parse(g).ok_or_else(|| op.err("bad_field", &format!("g {}", clip(g))))?;
        for &id in &atoms {
            self.slot(op, id)?;
        }
        for &id in &atoms {
            let s = self.slot(op, id)?;
            (s.stg, s.stg_op) = (l, Some(op.i));
        }
        Ok(())
    }
}

/// The groups of `m` from the labels of the slots (in output order), only on the centres `m` holds as defined: in the order of the groups
/// of the drawing `start`, new ones after them, numbered again per kind. A label a `stereo.group` gave an atom that is no defined centre
/// fails that operation (the first one when several do); an atom that kept its label from the drawing leaves its group. Returns the output
/// indices of the atoms that left their group and of those whose number changed.
pub(super) fn regroup(m: &mut Mol, ed: &Ed, start: &Mol) -> Result<(Vec<u32>, Vec<u32>)> {
    let defined = |i: u32| {
        m.stereo
            .iter()
            .any(|s| matches!(s, Stereo::Tetra { c, status: Status::Defined, .. } if *c == i))
    };
    let (mut members, mut left, mut bad) = (Vec::new(), Vec::new(), None::<(usize, u32)>);
    for (p, s) in ed.slots.iter().enumerate() {
        let (Some(l), p) = (s.stg, p as u32) else {
            continue;
        };
        match s.stg_op {
            _ if defined(p) => members.push((l, p)),
            Some(i) => bad = Some(bad.map_or((i, s.id), |b| b.min((i, s.id)))),
            None => left.push(p),
        }
    }
    if let Some((i, id)) = bad {
        return Err(fail(i, "stereo.group", "not_stereocentre", &id.to_string()));
    }
    // groups in the drawing's order, then the new ones by kind and number: within a kind the numbers stay ascending
    let mut order = labels(start).1;
    let mut keys: Vec<u64> = members
        .iter()
        .filter(|x| !order.contains(&x.0))
        .map(|x| u64::from(rank(x.0 .0)) << 32 | u64::from(x.0 .1))
        .collect();
    sort64(&mut keys);
    keys.dedup();
    order.extend(keys.iter().map(|&k| (kind_of(k >> 32), k as u32)));
    let (mut groups, mut moved, mut n) = (Vec::new(), vec![false; ed.slots.len()], [0u32; 3]);
    for l in order {
        let atoms: Vec<u32> = members.iter().filter(|x| x.0 == l).map(|x| x.1).collect();
        if atoms.is_empty() {
            continue;
        }
        let k = rank(l.0) as usize;
        n[k] += 1;
        // every abs group is written `abs`: its number is never seen, so it is never logged as changed
        if n[k] != l.1 && l.0 != GroupKind::Abs {
            atoms.iter().for_each(|&a| moved[a as usize] = true);
        }
        groups.push(StereoGroup { kind: l.0, atoms });
    }
    m.groups = groups;
    let moved = (0..moved.len() as u32)
        .filter(|&p| moved[p as usize])
        .collect();
    Ok((left, moved))
}

#[cfg(test)]
#[path = "edit_group_tests.rs"]
mod tests;
