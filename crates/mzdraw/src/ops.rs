//! Edit operations: the command log language (the draft of the `mol draw --set edit=` patches). Every operation applied to a document
//! records an exact inverse, so undo restores the document byte for byte; a `Session` is a document with its undo and redo stacks.

use crate::build;
use crate::doc::{round4, Atom, Bond, Document};
use serde::{Deserialize, Serialize};

fn one() -> u8 {
    1
}

/// One operation. Ids left out are taken from the document counter; `ring` is shorthand that expands to atoms and bonds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum Op {
    #[serde(rename = "atom.add")]
    AtomAdd {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<u32>,
        el: String,
        x: f64,
        y: f64,
    },
    #[serde(rename = "bond.add")]
    BondAdd {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<u32>,
        a: u32,
        b: u32,
        #[serde(default = "one")]
        o: u8,
    },
    #[serde(rename = "atom.set")]
    AtomSet {
        id: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        el: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        chg: Option<i8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        iso: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rad: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        h: Option<u8>,
    },
    #[serde(rename = "bond.set")]
    BondSet { id: u32, o: u8 },
    #[serde(rename = "move")]
    Move { atoms: Vec<u32>, dx: f64, dy: f64 },
    #[serde(rename = "remove")]
    Remove {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        atoms: Vec<u32>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        bonds: Vec<u32>,
    },
    /// A ring of `n` atoms (3-8), `arom` = alternating double bonds: fused on the bond `bond`, or with the atom `atom` as a corner, or
    /// centred on (`x`, `y`).
    #[serde(rename = "ring")]
    Ring {
        n: u8,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        arom: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        atom: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bond: Option<u32>,
        #[serde(default)]
        x: f64,
        #[serde(default)]
        y: f64,
    },
}

/// Why an operation was refused: an i18n key and the id it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpError {
    pub key: &'static str,
    pub id: u32,
}

fn err<T>(key: &'static str, id: u32) -> Result<T, OpError> {
    Err(OpError { key, id })
}

/// What undoes one change; applied in reverse order of recording.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Inv {
    DropAtom(u32),
    DropBond(u32),
    PutAtom(usize, Atom),
    PutBond(usize, Bond),
    SetAtom(Atom),
    SetBond(Bond),
    Next(u32),
}

impl Document {
    fn take_id(&mut self, id: Option<u32>, inv: &mut Vec<Inv>) -> Result<u32, OpError> {
        let id = id.unwrap_or(self.next);
        if id == 0 || self.atom(id).is_some() || self.bond(id).is_some() {
            return err("error.draw2.idTaken", id);
        }
        inv.push(Inv::Next(self.next));
        self.next = self.next.max(id + 1);
        Ok(id)
    }

    fn undo_one(&mut self, inv: Inv) {
        match inv {
            Inv::DropAtom(id) => self.atoms.retain(|a| a.id != id),
            Inv::DropBond(id) => self.bonds.retain(|b| b.id != id),
            Inv::PutAtom(i, a) => self.atoms.insert(i, a),
            Inv::PutBond(i, b) => self.bonds.insert(i, b),
            Inv::SetAtom(a) => {
                if let Some(i) = self.atom_index(a.id) {
                    self.atoms[i] = a;
                }
            }
            Inv::SetBond(b) => {
                if let Some(i) = self.bond_index(b.id) {
                    self.bonds[i] = b;
                }
            }
            Inv::Next(n) => self.next = n,
        }
    }

    pub(crate) fn revert(&mut self, inv: Vec<Inv>) {
        for i in inv.into_iter().rev() {
            self.undo_one(i);
        }
    }

    /// Applies one primitive operation, pushing its inverse and its resolved form (ids filled in, numbers rounded).
    fn apply_one(
        &mut self,
        op: &Op,
        done: &mut Vec<Op>,
        inv: &mut Vec<Inv>,
    ) -> Result<(), OpError> {
        match op {
            Op::AtomAdd { id, el, x, y } => {
                if !mzlab_mol::is_element(el) {
                    return err("error.draw2.element", id.unwrap_or(0));
                }
                let id = self.take_id(*id, inv)?;
                let (x, y) = (round4(*x), round4(*y));
                self.atoms.push(Atom {
                    id,
                    el: el.clone(),
                    x,
                    y,
                    chg: 0,
                    iso: 0,
                    rad: 0,
                    h: None,
                });
                inv.push(Inv::DropAtom(id));
                done.push(Op::AtomAdd {
                    id: Some(id),
                    el: el.clone(),
                    x,
                    y,
                });
            }
            Op::BondAdd { id, a, b, o } => {
                if self.atom(*a).is_none() || a == b {
                    return err("error.draw2.noAtom", *a);
                }
                if self.atom(*b).is_none() {
                    return err("error.draw2.noAtom", *b);
                }
                if !(1..=3).contains(o) || self.bond_between(*a, *b).is_some() {
                    return err("error.draw2.bond", *a);
                }
                let id = self.take_id(*id, inv)?;
                self.bonds.push(Bond {
                    id,
                    a: *a,
                    b: *b,
                    o: *o,
                });
                inv.push(Inv::DropBond(id));
                done.push(Op::BondAdd {
                    id: Some(id),
                    a: *a,
                    b: *b,
                    o: *o,
                });
            }
            Op::AtomSet {
                id,
                el,
                chg,
                iso,
                rad,
                h,
            } => {
                let Some(i) = self.atom_index(*id) else {
                    return err("error.draw2.noAtom", *id);
                };
                if el.as_ref().is_some_and(|e| !mzlab_mol::is_element(e)) {
                    return err("error.draw2.element", *id);
                }
                inv.push(Inv::SetAtom(self.atoms[i].clone()));
                let a = &mut self.atoms[i];
                if let Some(e) = el {
                    a.el = e.clone();
                    a.h = None;
                }
                a.chg = chg.unwrap_or(a.chg).clamp(-4, 4);
                a.iso = iso.unwrap_or(a.iso);
                a.rad = rad.unwrap_or(a.rad).min(3);
                a.h = h.or(a.h);
                done.push(op.clone());
            }
            Op::BondSet { id, o } => {
                let Some(i) = self.bond_index(*id) else {
                    return err("error.draw2.noBond", *id);
                };
                if !(1..=3).contains(o) {
                    return err("error.draw2.bond", *id);
                }
                inv.push(Inv::SetBond(self.bonds[i].clone()));
                self.bonds[i].o = *o;
                done.push(op.clone());
            }
            Op::Move { atoms, dx, dy } => {
                let (dx, dy) = (round4(*dx), round4(*dy));
                for id in atoms {
                    let Some(i) = self.atom_index(*id) else {
                        return err("error.draw2.noAtom", *id);
                    };
                    inv.push(Inv::SetAtom(self.atoms[i].clone()));
                    self.atoms[i].x = round4(self.atoms[i].x + dx);
                    self.atoms[i].y = round4(self.atoms[i].y + dy);
                }
                done.push(Op::Move {
                    atoms: atoms.clone(),
                    dx,
                    dy,
                });
            }
            Op::Remove { atoms, bonds } => self.remove(atoms, bonds, done, inv)?,
            Op::Ring { .. } => {
                for p in build::ring(self, op)? {
                    self.apply_one(&p, done, inv)?;
                }
            }
        }
        Ok(())
    }

    /// Removes atoms (with every bond on them) and bonds; positions in the pools are recorded so that undo puts each one back in place.
    fn remove(
        &mut self,
        atoms: &[u32],
        bonds: &[u32],
        done: &mut Vec<Op>,
        inv: &mut Vec<Inv>,
    ) -> Result<(), OpError> {
        if let Some(&id) = atoms.iter().find(|&&id| self.atom(id).is_none()) {
            return err("error.draw2.noAtom", id);
        }
        if let Some(&id) = bonds.iter().find(|&&id| self.bond(id).is_none()) {
            return err("error.draw2.noBond", id);
        }
        for i in (0..self.bonds.len()).rev() {
            let b = &self.bonds[i];
            if bonds.contains(&b.id) || atoms.contains(&b.a) || atoms.contains(&b.b) {
                inv.push(Inv::PutBond(i, self.bonds.remove(i)));
            }
        }
        for i in (0..self.atoms.len()).rev() {
            if atoms.contains(&self.atoms[i].id) {
                inv.push(Inv::PutAtom(i, self.atoms.remove(i)));
            }
        }
        done.push(Op::Remove {
            atoms: atoms.to_vec(),
            bonds: bonds.to_vec(),
        });
        Ok(())
    }

    /// Applies a group of operations as one step: all or nothing. Returns the resolved primitive operations and the inverse.
    pub(crate) fn apply(&mut self, ops: &[Op]) -> Result<(Vec<Op>, Vec<Inv>), OpError> {
        let (mut done, mut inv) = (Vec::new(), Vec::new());
        for op in ops {
            if let Err(e) = self.apply_one(op, &mut done, &mut inv) {
                self.revert(inv);
                return Err(e);
            }
        }
        Ok((done, inv))
    }
}

struct Step {
    ops: Vec<Op>,
    inv: Vec<Inv>,
}

/// A document with its history. `run` is one undo step.
#[derive(Default)]
pub struct Session {
    pub doc: Document,
    undo: Vec<Step>,
    redo: Vec<Vec<Op>>,
}

impl Session {
    pub fn new(doc: Document) -> Self {
        Session {
            doc,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Applies the operations as one step and forgets the redo stack. An empty group is not a step.
    pub fn run(&mut self, ops: &[Op]) -> Result<(), OpError> {
        let (done, inv) = self.doc.apply(ops)?;
        if !done.is_empty() {
            self.undo.push(Step { ops: done, inv });
            self.redo.clear();
        }
        Ok(())
    }

    /// Number of steps that can be undone.
    pub fn steps(&self) -> usize {
        self.undo.len()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let Some(s) = self.undo.pop() else {
            return false;
        };
        self.doc.revert(s.inv);
        self.redo.push(s.ops);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(ops) = self.redo.pop() else {
            return false;
        };
        // resolved operations of a step that applied before on this same document: they apply again
        if let Ok((done, inv)) = self.doc.apply(&ops) {
            self.undo.push(Step { ops: done, inv });
        }
        true
    }

    /// The command log: every operation still applied, in order, as primitive operations with their ids.
    pub fn log(&self) -> Vec<Op> {
        self.undo
            .iter()
            .flat_map(|s| s.ops.iter().cloned())
            .collect()
    }

    pub fn log_json(&self) -> String {
        serde_json::to_string(&self.log()).unwrap_or_default()
    }

    /// Replays a JSON command log, one step per operation; stops at the first refused operation.
    pub fn replay(&mut self, log_json: &str) -> Result<(), OpError> {
        let ops: Vec<Op> = serde_json::from_str(log_json).map_err(|_| OpError {
            key: "error.draw2.log",
            id: 0,
        })?;
        for op in &ops {
            self.run(std::slice::from_ref(op))?;
        }
        Ok(())
    }
}
