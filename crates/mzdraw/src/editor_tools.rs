//! What the tools do with a press, a drag and a release, and the keyboard. Each completed gesture is one undo step.

use crate::build;
use crate::editor::{Editor, Hit, Press, Tool, CHANGED, CTRL, HANDLED, REPAINT, SHIFT, TOOL};
use crate::m;
use crate::ops::Op;
use std::f64::consts::PI;

/// A press becomes a drag after this many pixels.
const SLOP: f64 = 4.0;

impl Editor {
    pub fn pointer_down(&mut self, px: f64, py: f64, kind: u32, mods: u32) -> u32 {
        let hit = self.hit(px, py, kind);
        let shift = mods & SHIFT != 0;
        if self.tool == Tool::Select {
            self.select_press(hit, shift);
        }
        self.press = Some(Press {
            start: (px, py),
            now: (px, py),
            hit,
            moved: false,
            kind,
        });
        REPAINT
    }

    /// Selection on press: a thing not yet selected becomes the selection (added with Shift); an empty spot clears it.
    fn select_press(&mut self, hit: Hit, shift: bool) {
        let o = &mut self.over;
        let inside = match hit {
            Hit::Atom(a) => o.sel_atoms.contains(&a),
            Hit::Bond(b) => o.sel_bonds.contains(&b),
            Hit::None => false,
        };
        if !inside && !shift {
            o.sel_atoms.clear();
            o.sel_bonds.clear();
        }
        match hit {
            Hit::Atom(a) if !inside => o.sel_atoms.push(a),
            Hit::Bond(b) if !inside => {
                o.sel_bonds.push(b);
                if let Some(x) = self.session.doc.bond(b) {
                    for e in [x.a, x.b] {
                        if !o.sel_atoms.contains(&e) {
                            o.sel_atoms.push(e);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn pointer_move(&mut self, px: f64, py: f64, kind: u32) -> u32 {
        let Some(p) = self.press.as_mut() else {
            let hit = self.hit(px, py, kind);
            let (a, b) = match hit {
                Hit::Atom(a) => (Some(a), None),
                Hit::Bond(b) => (None, Some(b)),
                Hit::None => (None, None),
            };
            let same = self.over.hover_atom == a && self.over.hover_bond == b;
            (self.over.hover_atom, self.over.hover_bond) = (a, b);
            return if same { 0 } else { REPAINT };
        };
        p.now = (px, py);
        if m::hypot(px - p.start.0, py - p.start.1) > SLOP {
            p.moved = true;
        }
        if !p.moved {
            return 0;
        }
        let (start, hit, kind) = (p.start, p.hit, p.kind);
        match (&self.tool, hit) {
            (Tool::Select, Hit::None) => {
                self.over.band = Some((
                    start.0.min(px),
                    start.1.min(py),
                    (px - start.0).abs(),
                    (py - start.1).abs(),
                ));
            }
            (Tool::Select, _) => {
                self.over.drag = (
                    (px - start.0) / self.view.scale,
                    (start.1 - py) / self.view.scale,
                );
            }
            (Tool::Bond(_) | Tool::Atom(_), Hit::Atom(a)) => {
                let (to, _) = self.sprout_target(a, px, py, kind);
                if let Some(from) = self.session.doc.atom(a) {
                    self.over.preview = Some((from.x, from.y, to.0, to.1));
                }
            }
            _ => {}
        }
        REPAINT
    }

    /// Where a bond dragged from atom `a` ends: on the atom under the pointer, else one bond length away at a multiple of 30°.
    fn sprout_target(&self, a: u32, px: f64, py: f64, kind: u32) -> ((f64, f64), Option<u32>) {
        let d = &self.session.doc;
        if let Hit::Atom(b) = self.hit(px, py, kind)
            && b != a
            && let Some(t) = d.atom(b)
        {
            return ((t.x, t.y), Some(b));
        }
        let Some(from) = d.atom(a) else {
            return ((0.0, 0.0), None);
        };
        let (x, y) = self.view.to_doc(px, py);
        let step = PI / 6.0;
        let ang = (m::atan2(y - from.y, x - from.x) / step).round() * step;
        ((from.x + m::cos(ang), from.y + m::sin(ang)), None)
    }

    pub fn pointer_up(&mut self, px: f64, py: f64) -> u32 {
        let Some(p) = self.press.take() else {
            return 0;
        };
        let drag = self.over.drag;
        let band = self.over.band.take();
        self.over.drag = (0.0, 0.0);
        self.over.preview = None;
        let flags = match self.tool.clone() {
            Tool::Select => self.select_release(&p, drag, band),
            Tool::Atom(el) => self.atom_release(&p, &el, px, py),
            Tool::Bond(o) => self.bond_release(&p, o, px, py),
            Tool::Erase => self.erase(p.hit),
            Tool::Ring(n, arom) => {
                let (x, y) = self.view.to_doc(px, py);
                let (atom, bond) = match p.hit {
                    Hit::Atom(a) => (Some(a), None),
                    Hit::Bond(b) => (None, Some(b)),
                    Hit::None => (None, None),
                };
                self.run(
                    &[Op::Ring {
                        n,
                        arom,
                        atom,
                        bond,
                        x,
                        y,
                    }],
                    "draw2.say.ring",
                    &n.to_string(),
                )
            }
            Tool::Charge(c) => match p.hit {
                Hit::Atom(a) => {
                    let now = self.session.doc.atom(a).map_or(0, |x| x.chg);
                    let set = Op::AtomSet {
                        id: a,
                        el: None,
                        chg: Some(now + c),
                        iso: None,
                        rad: None,
                        h: None,
                    };
                    self.run(&[set], "draw2.say.charge", &(now + c).to_string())
                }
                _ => 0,
            },
        };
        flags | REPAINT
    }

    fn select_release(
        &mut self,
        p: &Press,
        drag: (f64, f64),
        band: Option<(f64, f64, f64, f64)>,
    ) -> u32 {
        if let Some((x, y, w, h)) = band {
            let d = &self.session.doc;
            let inside = |id: u32| {
                d.atom(id).is_some_and(|a| {
                    let (ax, ay) = self.view.to_px(a.x, a.y);
                    ax >= x && ax <= x + w && ay >= y && ay <= y + h
                })
            };
            let atoms: Vec<u32> = d
                .atoms
                .iter()
                .map(|a| a.id)
                .filter(|&i| inside(i))
                .collect();
            let bonds: Vec<u32> = d
                .bonds
                .iter()
                .filter(|b| inside(b.a) && inside(b.b))
                .map(|b| b.id)
                .collect();
            for a in atoms {
                if !self.over.sel_atoms.contains(&a) {
                    self.over.sel_atoms.push(a);
                }
            }
            for b in bonds {
                if !self.over.sel_bonds.contains(&b) {
                    self.over.sel_bonds.push(b);
                }
            }
            return REPAINT;
        }
        if p.moved && p.hit != Hit::None && drag != (0.0, 0.0) {
            let atoms = self.over.sel_atoms.clone();
            let n = atoms.len().to_string();
            return self.run(
                &[Op::Move {
                    atoms,
                    dx: drag.0,
                    dy: drag.1,
                }],
                "draw2.say.moved",
                &n,
            );
        }
        REPAINT
    }

    fn atom_release(&mut self, p: &Press, el: &str, px: f64, py: f64) -> u32 {
        match p.hit {
            Hit::Atom(a) if p.moved => self.sprout(a, el, 1, px, py, p.kind),
            Hit::Atom(a) => {
                let set = Op::AtomSet {
                    id: a,
                    el: Some(el.into()),
                    chg: None,
                    iso: None,
                    rad: None,
                    h: None,
                };
                self.run(&[set], "draw2.say.element", el)
            }
            Hit::Bond(_) => 0,
            Hit::None => {
                let (x, y) = self.view.to_doc(px, py);
                self.run(
                    &[Op::AtomAdd {
                        id: None,
                        el: el.into(),
                        x,
                        y,
                    }],
                    "draw2.say.atom",
                    el,
                )
            }
        }
    }

    /// A bond from atom `a` to the atom under the pointer or to a new atom `el`.
    fn sprout(&mut self, a: u32, el: &str, o: u8, px: f64, py: f64, kind: u32) -> u32 {
        let (to, onto) = self.sprout_target(a, px, py, kind);
        self.join(a, onto, to, el, o)
    }

    /// Bonds `a` to the existing atom `onto` (raising the order of a bond already there) or to a new atom at `to`.
    fn join(&mut self, a: u32, onto: Option<u32>, to: (f64, f64), el: &str, o: u8) -> u32 {
        let d = &self.session.doc;
        let ops = match onto {
            Some(b) => match d.bond_between(a, b) {
                Some(x) => vec![Op::BondSet { id: x.id, o }],
                None => vec![Op::BondAdd { id: None, a, b, o }],
            },
            None => {
                let id = d.next;
                vec![
                    Op::AtomAdd {
                        id: Some(id),
                        el: el.into(),
                        x: to.0,
                        y: to.1,
                    },
                    Op::BondAdd {
                        id: None,
                        a,
                        b: id,
                        o,
                    },
                ]
            }
        };
        self.run(&ops, "draw2.say.bond", &o.to_string())
    }

    fn bond_release(&mut self, p: &Press, o: u8, px: f64, py: f64) -> u32 {
        match p.hit {
            Hit::Atom(a) if p.moved => self.sprout(a, "C", o, px, py, p.kind),
            Hit::Atom(a) => {
                let ang = build::sprout_angle(&self.session.doc, a);
                let to = self
                    .session
                    .doc
                    .atom(a)
                    .map_or((0.0, 0.0), |f| (f.x + m::cos(ang), f.y + m::sin(ang)));
                self.join(a, None, to, "C", o)
            }
            Hit::Bond(b) => {
                // the single-bond tool on a bond walks 1 -> 2 -> 3 -> 1; the others set their order
                let now = self.session.doc.bond(b).map_or(1, |x| x.o);
                let next = if o == 1 { now % 3 + 1 } else { o };
                self.run(
                    &[Op::BondSet { id: b, o: next }],
                    "draw2.say.order",
                    &next.to_string(),
                )
            }
            Hit::None => {
                let (x, y) = self.view.to_doc(px, py);
                let id = self.session.doc.next;
                let (c, s) = (m::cos(PI / 6.0), m::sin(PI / 6.0));
                let ops = [
                    Op::AtomAdd {
                        id: Some(id),
                        el: "C".into(),
                        x,
                        y,
                    },
                    Op::AtomAdd {
                        id: Some(id + 1),
                        el: "C".into(),
                        x: x + c,
                        y: y + s,
                    },
                    Op::BondAdd {
                        id: None,
                        a: id,
                        b: id + 1,
                        o,
                    },
                ];
                self.run(&ops, "draw2.say.bond", &o.to_string())
            }
        }
    }

    /// Erases the thing hit, or the whole selection when the thing is part of it.
    fn erase(&mut self, hit: Hit) -> u32 {
        let o = &self.over;
        let op = match hit {
            Hit::Atom(a) if o.sel_atoms.contains(&a) => Op::Remove {
                atoms: o.sel_atoms.clone(),
                bonds: Vec::new(),
            },
            Hit::Atom(a) => Op::Remove {
                atoms: vec![a],
                bonds: Vec::new(),
            },
            Hit::Bond(b) => Op::Remove {
                atoms: Vec::new(),
                bonds: vec![b],
            },
            Hit::None => return 0,
        };
        self.run(&[op], "draw2.say.erased", "")
    }

    pub fn delete_selection(&mut self) -> u32 {
        let (atoms, bonds) = (self.over.sel_atoms.clone(), self.over.sel_bonds.clone());
        if atoms.is_empty() && bonds.is_empty() {
            return 0;
        }
        // a selected bond goes with its atoms only when they are selected too
        let bonds = bonds
            .into_iter()
            .filter(|&b| {
                self.session
                    .doc
                    .bond(b)
                    .is_some_and(|x| !atoms.contains(&x.a) && !atoms.contains(&x.b))
            })
            .collect();
        self.run(&[Op::Remove { atoms, bonds }], "draw2.say.erased", "")
    }

    pub fn select_all(&mut self) -> u32 {
        self.over.sel_atoms = self.session.doc.atoms.iter().map(|a| a.id).collect();
        self.over.sel_bonds = self.session.doc.bonds.iter().map(|b| b.id).collect();
        REPAINT
    }

    /// A key press (`key` as the browser names it). Returns `HANDLED` when the page must not let the browser act on it.
    pub fn key(&mut self, key: &str, mods: u32) -> u32 {
        let ctrl = mods & CTRL != 0;
        let shift = mods & SHIFT != 0;
        let lower = key.to_ascii_lowercase();
        let f = match (ctrl, lower.as_str()) {
            (true, "z") if shift => self.redo(),
            (true, "z") => self.undo(),
            (true, "y") => self.redo(),
            (true, "a") => self.select_all(),
            (true, _) => return 0,
            (false, "delete" | "backspace") => self.delete_selection(),
            (false, "escape") => {
                self.over.sel_atoms.clear();
                self.over.sel_bonds.clear();
                self.set_tool("select")
            }
            (false, "1" | "2" | "3") => self.set_tool(&format!("bond:{lower}")),
            (false, "+" | "-") => self.set_tool(&format!("charge:{lower}")),
            (false, "v") => self.set_tool("select"),
            (false, "e") => self.set_tool("erase"),
            (false, k) => return self.element_key(k, shift),
        };
        f | HANDLED
    }

    /// A letter sets the element of the atom under the pointer, or picks the atom tool.
    fn element_key(&mut self, k: &str, shift: bool) -> u32 {
        let el = match (k, shift) {
            ("c", true) => "Cl",
            ("b", true) => "Br",
            ("s", true) => "Si",
            ("c", _) => "C",
            ("n", _) => "N",
            ("o", _) => "O",
            ("s", _) => "S",
            ("p", _) => "P",
            ("f", _) => "F",
            ("h", _) => "H",
            ("b", _) => "B",
            ("i", _) => "I",
            _ => return 0,
        };
        let f = match self.over.hover_atom {
            Some(a) => {
                let set = Op::AtomSet {
                    id: a,
                    el: Some(el.into()),
                    chg: None,
                    iso: None,
                    rad: None,
                    h: None,
                };
                self.run(&[set], "draw2.say.element", el)
            }
            None => self.set_tool(&format!("atom:{el}")),
        };
        f | HANDLED | (f & (CHANGED | TOOL))
    }
}
