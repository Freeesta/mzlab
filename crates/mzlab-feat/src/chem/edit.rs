//! The edit patch of `mol draw --set edit=`: a JSON list of operations applied to a drawn structure, all or none. Atoms keep their id (the
//! index in the drawing the patch starts from; a new atom gets the next free id) while the patch runs, bonds are named by their two atoms.
//! Coordinates the patch does not touch stay as they are; only atoms added without a position are placed, around the fixed ones. Stereo
//! elements of atoms whose bonds changed are dropped and logged; a wedge set by the patch is read back by the molfile reader's own rules.
//! Enhanced stereo groups follow the atoms (`stereo.group` sets them, `edit_group.rs`).
//! Units: a bare `x`, `y`, `dx`, `dy` is in bond lengths (1.0 = 1.5 angstrom), the same names ending in `_a` are angstrom; never mixed.
//! This file holds the operations; `edit_op.rs` reads their fields, `edit_end.rs` builds the record.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;
use mzlab_mol::{is_element, Atom};

use super::draw::{Drawn, MAX_ATOMS};
use super::edit_end::finish;
use super::edit_group::labels;
use super::edit_op::{clip, fail, pair, r4, Op, MAX_ID};
use super::edit_set::{weight, Index, Sets, Slot, MAX_BONDS, MAX_OPS, MAX_PATCH};

impl Drawn {
    /// The molfile of the drawing: the record's, with an alias line for each atom label (V2000 only).
    pub fn molfile(&self) -> String {
        let mut t = self.mol.to_molfile(&self.title);
        if self.labels.is_empty() || t.lines().nth(3).is_some_and(|l| l.contains("V3000")) {
            return t;
        }
        t.truncate(t.len() - "M  END\n".len());
        for (i, l) in &self.labels {
            t += &format!("A  {:>3}\n{l}\n", i + 1);
        }
        t + "M  END\n"
    }
}

impl Drawn {
    /// The text a patch gave atom `i`, drawn instead of its symbol (and its hydrogens) in the pictures, KET and CDXML.
    pub(super) fn label_at(&self, i: usize) -> Option<&str> {
        self.labels
            .iter()
            .find(|l| l.0 == i as u32)
            .map(|l| l.1.as_str())
    }
}

pub(super) struct Ed {
    pub(super) slots: Vec<Slot>,
    pub(super) bonds: Vec<(u32, u32, u8)>,
    pub(super) next: u32,
    pub(super) sets: Sets,
    pub(super) added: Vec<u32>,
    /// Wedges the patch sets: begin atom, end atom, mark, index of the operation.
    pub(super) wedges: Vec<(u32, u32, i8, usize)>,
}

fn same(b: &(u32, u32, u8), x: u32, y: u32) -> bool {
    (b.0 == x && b.1 == y) || (b.0 == y && b.1 == x)
}

impl Ed {
    pub(super) fn slot(&mut self, op: &Op, id: u32) -> Result<&mut Slot> {
        self.slots
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| op.err_id("unknown_atom", id))
    }

    fn bond_at(&self, op: &Op, a: u32, b: u32) -> Result<usize> {
        self.bonds
            .iter()
            .position(|x| same(x, a, b))
            .ok_or_else(|| op.err("unknown_bond", &pair(a, b)))
    }

    fn element(op: &Op, el: &str) -> Result<()> {
        if is_element(el) {
            Ok(())
        } else {
            Err(op.err("bad_field", &format!("el {}", clip(el))))
        }
    }

    /// The fields of an atom shared by `atom.add` and `atom.set`: charge, isotope, radical, hydrogens, label (an empty one clears it).
    #[inline(never)]
    fn atom_fields(&mut self, op: &Op, id: u32) -> Result<()> {
        let el = op.text("el")?;
        if let Some(el) = el {
            Self::element(op, el)?;
        }
        let (chg, iso) = (op.int("chg", -9, 9)?, op.int("iso", 0, 999)?);
        let (rad, h) = (op.int("rad", 0, 3)?, op.int("h", 0, 8)?);
        let label = op.text("label")?.map(str::to_string);
        if label
            .as_ref()
            .is_some_and(|l| l.len() > 70 || l.contains('\n'))
        {
            return Err(op.err("bad_field", "label (one line, at most 70 bytes)"));
        }
        let moved = el.is_some() || chg.is_some() || rad.is_some();
        if h.is_some() {
            self.sets.h_given.add(id);
        } else if moved {
            self.sets.h_given.0.retain(|&x| x != id);
        }
        // an atom with hydrogens given is still checked against its valence
        if moved || h.is_some() {
            self.sets.atom.add(id);
        }
        let s = self.slot(op, id)?;
        if let Some(el) = el {
            s.atom.el = el.to_string();
        }
        s.atom.chg = chg.map_or(s.atom.chg, |v| v as i8);
        s.atom.iso = iso.map_or(s.atom.iso, |v| v as u16);
        s.atom.rad = rad.map_or(s.atom.rad, |v| v as u8);
        if let Some(h) = h {
            s.atom.h = h as u8;
        }
        if let Some(l) = label {
            s.label = Some(l).filter(|l| !l.is_empty());
        }
        Ok(())
    }

    #[inline(never)]
    fn atom_add(&mut self, op: &Op) -> Result<()> {
        op.only(&[
            "id", "el", "chg", "iso", "rad", "h", "label", "x", "y", "x_a", "y_a",
        ])?;
        let id = match op.int("id", 0, i64::from(MAX_ID))? {
            Some(id) => id as u32,
            None => self.next,
        };
        if self.slots.iter().any(|s| s.id == id) {
            return Err(op.err_id("atom_exists", id));
        }
        let el = op
            .text("el")?
            .ok_or_else(|| op.err("missing_field", "el"))?;
        Self::element(op, el)?;
        if self.slots.len() >= MAX_ATOMS {
            return Err(Error::new("err.mol.draw.tooBig").with("max", MAX_ATOMS));
        }
        let xy = op.point(["x", "y", "x_a", "y_a"])?;
        self.slots.push(Slot {
            id,
            atom: Atom {
                el: el.to_string(),
                iso: 0,
                chg: 0,
                rad: 0,
                h: 0,
            },
            xy,
            label: None,
            stg: None,
            stg_op: None,
        });
        self.next = self.next.max(id + 1);
        self.added.push(id);
        self.atom_fields(op, id)
    }

    #[inline(never)]
    fn atom_set(&mut self, op: &Op) -> Result<()> {
        op.only(&["id", "el", "chg", "iso", "rad", "h", "label"])?;
        let id = op.id("id")?;
        self.slot(op, id)?;
        if ["el", "chg", "iso", "rad", "h", "label"]
            .iter()
            .all(|k| op.v.get(k).is_none())
        {
            return Err(op.err("missing_field", "el, chg, iso, rad, h or label"));
        }
        self.atom_fields(op, id)
    }

    fn mark(&mut self, op: &Op, a: u32, b: u32, w: Option<i8>) {
        if let Some(w) = w {
            self.wedges.retain(|x| !(same(&(x.0, x.1, 0), a, b)));
            self.wedges.push((a, b, w, op.i));
        }
    }

    #[inline(never)]
    fn bond_add(&mut self, op: &Op) -> Result<()> {
        op.only(&["a", "b", "o", "wedge"])?;
        let (a, b) = (op.id("a")?, op.id("b")?);
        if a == b {
            return Err(op.err_id("self_bond", a));
        }
        self.slot(op, a)?;
        self.slot(op, b)?;
        if self.bonds.iter().any(|x| same(x, a, b)) {
            return Err(op.err("bond_exists", &pair(a, b)));
        }
        if self.bonds.len() >= MAX_BONDS {
            return Err(op.err("too_many_bonds", "at most 2500 bonds"));
        }
        let o = op.int("o", 1, 3)?.unwrap_or(1) as u8;
        self.bonds.push((a, b, o));
        self.sets.shape.add(a);
        self.sets.shape.add(b);
        let w = op.wedge()?;
        self.mark(op, a, b, w);
        Ok(())
    }

    #[inline(never)]
    fn bond_set(&mut self, op: &Op) -> Result<()> {
        op.only(&["a", "b", "o", "wedge"])?;
        let (a, b) = (op.id("a")?, op.id("b")?);
        let k = self.bond_at(op, a, b)?;
        let (o, w) = (op.int("o", 1, 3)?, op.wedge()?);
        if o.is_none() && w.is_none() {
            return Err(op.err("missing_field", "o or wedge"));
        }
        if let Some(o) = o.filter(|&o| self.bonds[k].2 != o as u8) {
            self.bonds[k].2 = o as u8;
            self.sets.shape.add(a);
            self.sets.shape.add(b);
        }
        self.mark(op, a, b, w);
        Ok(())
    }

    #[inline(never)]
    fn remove(&mut self, op: &Op) -> Result<()> {
        op.only(&["atoms", "bonds"])?;
        let atoms = op.ids("atoms")?;
        let pairs = op
            .list("bonds", MAX_BONDS)?
            .iter()
            .map(|p| match (p.at(0), p.arr().len()) {
                (Some(a), 2) => Ok((op.id_in(a, "bonds")?, op.id_in(&p.arr()[1], "bonds")?)),
                _ => Err(op.err("bad_field", "bonds")),
            })
            .collect::<Result<Vec<_>>>()?;
        if atoms.is_empty() && pairs.is_empty() {
            return Err(op.err("missing_field", "atoms or bonds"));
        }
        for &(a, b) in &pairs {
            let k = self.bond_at(op, a, b)?;
            self.bonds.remove(k);
            self.wedges.retain(|w| !same(&(w.0, w.1, 0), a, b));
            self.sets.shape.add(a);
            self.sets.shape.add(b);
        }
        for &id in &atoms {
            self.slot(op, id)?;
        }
        for &id in &atoms {
            let at = self.slots.iter().position(|s| s.id == id);
            if let Some(at) = at {
                self.slots.remove(at);
                for b in self.bonds.iter().filter(|b| b.0 == id || b.1 == id) {
                    self.sets.shape.add(if b.0 == id { b.1 } else { b.0 });
                }
                self.bonds.retain(|b| b.0 != id && b.1 != id);
                self.wedges.retain(|w| w.0 != id && w.1 != id);
                self.sets.shape.add(id);
            }
        }
        Ok(())
    }

    /// The slots (as a mask over their places) of the piece, the connected part, that holds the atom at place `start`.
    #[inline(never)]
    fn piece(&self, ix: &Index, start: u32) -> Vec<bool> {
        let mut adj = vec![Vec::new(); self.slots.len()];
        for b in &self.bonds {
            if let (Some(x), Some(y)) = (ix.get(b.0), ix.get(b.1)) {
                adj[x as usize].push(y as usize);
                adj[y as usize].push(x as usize);
            }
        }
        let mut seen = vec![false; self.slots.len()];
        seen[start as usize] = true;
        let mut todo = vec![start as usize];
        while let Some(x) = todo.pop() {
            for &y in &adj[x] {
                if !seen[y] {
                    seen[y] = true;
                    todo.push(y);
                }
            }
        }
        seen
    }

    /// Moves the slots in `mask` by `d` angstrom. An atom without a position is an error when the move names it (`strict`), else it is left for
    /// the layout at the end.
    fn shift(&mut self, op: &Op, mask: &[bool], d: [f64; 2], strict: bool) -> Result<()> {
        for (s, _) in self.slots.iter_mut().zip(mask).filter(|(_, m)| **m) {
            match s.xy {
                Some(p) => s.xy = Some([r4(p[0] + d[0]), r4(p[1] + d[1])]),
                None if strict => return Err(op.err_id("unplaced", s.id)),
                None => {}
            }
        }
        Ok(())
    }

    #[inline(never)]
    fn move_atoms(&mut self, op: &Op) -> Result<()> {
        op.only(&["atoms", "dx", "dy", "dx_a", "dy_a"])?;
        let atoms = op.ids("atoms")?;
        if atoms.is_empty() {
            return Err(op.err("missing_field", "atoms"));
        }
        let d = op
            .point(["dx", "dy", "dx_a", "dy_a"])?
            .ok_or_else(|| op.err("missing_field", "dx, dy"))?;
        for &id in &atoms {
            self.slot(op, id)?;
        }
        let ix = Index::of(&self.slots);
        let mut mask = vec![false; self.slots.len()];
        atoms.iter().for_each(|&id| {
            if let Some(p) = ix.get(id) {
                mask[p as usize] = true;
            }
        });
        self.shift(op, &mask, d, true)
    }

    /// Atom `from` is fused into atom `into`: its bonds go to `into`, which keeps its own element and charge. When the two sit in different
    /// pieces, the piece of `from` is first moved so that the two atoms coincide (its atoms without a position stay for the layout).
    #[inline(never)]
    fn merge(&mut self, op: &Op) -> Result<()> {
        op.only(&["from", "into"])?;
        let (from, into) = (op.id("from")?, op.id("into")?);
        if from == into {
            return Err(op.err_id("self_bond", from));
        }
        let (pf, pi) = (self.slot(op, from)?.xy, self.slot(op, into)?.xy);
        let mut bonds = Vec::new();
        for b in self.bonds.iter().filter(|b| b.0 == from || b.1 == from) {
            let other = if b.0 == from { b.1 } else { b.0 };
            if other != into {
                bonds.push((other, b.2));
            }
        }
        for &(other, _) in &bonds {
            if self.bonds.iter().any(|x| same(x, into, other)) {
                return Err(op.err("bond_exists", &pair(into, other)));
            }
        }
        let ix = Index::of(&self.slots);
        let piece = self.piece(&ix, ix.get(from).unwrap_or(0));
        let together = ix.get(into).is_some_and(|p| piece[p as usize]);
        if let (false, Some(f), Some(i)) = (together, pf, pi) {
            self.shift(op, &piece, [i[0] - f[0], i[1] - f[1]], false)?;
        }
        self.bonds.retain(|b| b.0 != from && b.1 != from);
        for (other, o) in bonds {
            self.bonds.push((into, other, o));
            self.sets.shape.add(other);
        }
        let at = self.slots.iter().position(|s| s.id == from).unwrap_or(0);
        let gone = self.slots.remove(at);
        let keep = self.slot(op, into)?;
        keep.label = keep.label.take().or(gone.label);
        self.wedges.retain(|w| w.0 != from && w.1 != from);
        self.sets.shape.add(from);
        self.sets.shape.add(into);
        Ok(())
    }
}

impl Ed {
    /// The slots, bonds and labels of the drawing the patch starts from.
    #[inline(never)]
    fn start(d: &Drawn) -> Ed {
        let m = &d.mol;
        let stg = labels(m).0;
        let xy = |i: usize| {
            m.coords
                .as_ref()
                .and_then(|c| c.xy.get(i))
                .filter(|r| r.len() >= 2)
                .map(|r| [r[0], r[1]])
        };
        Ed {
            slots: m
                .atoms
                .iter()
                .enumerate()
                .map(|(i, a)| Slot {
                    id: i as u32,
                    atom: a.clone(),
                    xy: xy(i),
                    label: d
                        .labels
                        .iter()
                        .find(|l| l.0 == i as u32)
                        .map(|l| l.1.clone()),
                    stg: stg[i],
                    stg_op: None,
                })
                .collect(),
            bonds: m.bonds.iter().map(|b| (b.a, b.b, b.o)).collect(),
            next: m.atoms.len() as u32,
            sets: Sets::default(),
            added: Vec::new(),
            wedges: Vec::new(),
        }
    }
}

/// The patch as its list of operations: an array, or an object with `ops`.
fn ops_of(v: &Value) -> Result<&[Value]> {
    let bad = |m: &str| Error::new("err.mol.draw.editJson").with("message", m);
    match v {
        Value::Arr(a) => Ok(a),
        Value::Obj(e) => {
            // `schema` may open the object, as in every document of mzLab
            let known = e.iter().all(|(k, x)| match k.as_str() {
                "schema" => x.str() == Some("draw_edit/1"),
                "ops" => true,
                _ => false,
            });
            match v.get("ops") {
                Some(Value::Arr(a)) if known => Ok(a),
                _ => Err(bad(
                    "an object patch has ops, a list (and optionally schema draw_edit/1)",
                )),
            }
        }
        _ => Err(bad("a patch is a list of operations")),
    }
}

/// A patch over `MAX_PATCH` bytes (as text, or as the text of a list) is refused.
fn too_long(n: usize) -> Result<()> {
    if n > MAX_PATCH {
        return Err(
            Error::new("err.mol.draw.editJson").with("message", "the patch is over 65536 bytes")
        );
    }
    Ok(())
}

/// Applies the patch `edit` (a list of operations, or JSON text of one) to the drawing: all operations apply or none does.
pub(super) fn apply(d: &mut Drawn, edit: &Value) -> Result<()> {
    let parsed;
    let edit = match edit {
        Value::Str(s) => {
            too_long(s.len())?;
            parsed = mzlab_core::json::parse(s.as_bytes()).map_err(|_| {
                Error::new("err.mol.draw.editJson").with("message", "the patch is not valid JSON")
            })?;
            &parsed
        }
        // a patch given as a list is held to the same size (counted as compact text), so every way to give it has one limit
        v => {
            too_long(weight(v))?;
            v
        }
    };
    let list = ops_of(edit)?;
    if list.len() > MAX_OPS {
        return Err(fail(
            MAX_OPS,
            "?",
            "too_many_ops",
            "at most 1000 operations",
        ));
    }
    let mut ed = Ed::start(d);
    for (i, v) in list.iter().enumerate() {
        let name = v.get("op").and_then(Value::str).ok_or_else(|| {
            fail(
                i,
                "?",
                "bad_op",
                "an operation is an object with an op name",
            )
        })?;
        let op = Op { i, name, v };
        match name {
            "atom.add" => ed.atom_add(&op),
            "atom.set" => ed.atom_set(&op),
            "bond.add" => ed.bond_add(&op),
            "bond.set" => ed.bond_set(&op),
            "remove" => ed.remove(&op),
            "move" => ed.move_atoms(&op),
            "merge" => ed.merge(&op),
            "stereo.group" => ed.stereo_group(&op),
            _ => Err(op.err("unknown_op", name)),
        }?;
    }
    finish(d, ed, list.len())
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;
