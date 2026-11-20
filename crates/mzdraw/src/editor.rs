//! The interactive editor: a session, a view, a tool and the pointer state machine. The page forwards pointer, wheel and key events
//! as numbers and paints the display list; every decision (what is under the pointer, what a click does) is taken here.

use crate::doc::Document;
use crate::io;
use crate::m;
use crate::ops::{Op, Session};
use crate::scene::{self, Overlay, Scene, View};

/// Flags an event returns.
pub const REPAINT: u32 = 1;
pub const CHANGED: u32 = 2;
pub const TOOL: u32 = 4;
pub const HANDLED: u32 = 8;

/// Modifier bits of an event.
pub const SHIFT: u32 = 1;
pub const CTRL: u32 = 2;

/// Pointer kinds: mouse, pen, finger. A finger needs a wider reach.
pub const MOUSE: u32 = 0;
pub const PEN: u32 = 1;
pub const TOUCH: u32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Select,
    Atom(String),
    Bond(u8),
    Erase,
    Ring(u8, bool),
    Charge(i8),
}

impl Tool {
    /// `select`, `atom:N`, `bond:2`, `erase`, `ring:6`, `ring:6a` (alternating double bonds), `charge:+`, `charge:-`.
    pub fn parse(s: &str) -> Option<Tool> {
        let (name, arg) = s.split_once(':').unwrap_or((s, ""));
        Some(match name {
            "select" => Tool::Select,
            "erase" => Tool::Erase,
            "atom" if mzlab_mol::is_element(arg) => Tool::Atom(arg.into()),
            "bond" => Tool::Bond(arg.parse().ok().filter(|o| (1..=3).contains(o))?),
            "ring" => {
                let n = arg
                    .trim_end_matches('a')
                    .parse()
                    .ok()
                    .filter(|n| (3..=8).contains(n))?;
                Tool::Ring(n, arg.ends_with('a'))
            }
            "charge" => Tool::Charge(if arg == "-" { -1 } else { 1 }),
            _ => return None,
        })
    }

    pub fn name(&self) -> String {
        match self {
            Tool::Select => "select".into(),
            Tool::Erase => "erase".into(),
            Tool::Atom(e) => format!("atom:{e}"),
            Tool::Bond(o) => format!("bond:{o}"),
            Tool::Ring(n, a) => format!("ring:{n}{}", if *a { "a" } else { "" }),
            Tool::Charge(c) => format!("charge:{}", if *c < 0 { "-" } else { "+" }),
        }
    }
}

/// What is under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    None,
    Atom(u32),
    Bond(u32),
}

/// A press in progress: where it started (pixels), what it hit, whether it became a drag.
pub(crate) struct Press {
    pub start: (f64, f64),
    pub now: (f64, f64),
    pub hit: Hit,
    pub moved: bool,
    pub kind: u32,
}

pub struct Editor {
    pub session: Session,
    pub view: View,
    pub tool: Tool,
    pub over: Overlay,
    pub(crate) press: Option<Press>,
    /// What the last change did, as an i18n key with one parameter, for the screen reader line.
    pub said: (&'static str, String),
    /// The last refused operation, an i18n key.
    pub error: &'static str,
}

impl Default for Editor {
    fn default() -> Self {
        Editor {
            session: Session::default(),
            view: View {
                scale: 40.0,
                ox: 200.0,
                oy: 200.0,
            },
            tool: Tool::Bond(1),
            over: Overlay::default(),
            press: None,
            said: ("", String::new()),
            error: "",
        }
    }
}

impl Editor {
    pub fn doc(&self) -> &Document {
        &self.session.doc
    }

    /// Runs one undo step; a refusal is kept in `error` and changes nothing.
    pub(crate) fn run(&mut self, ops: &[Op], say: &'static str, what: &str) -> u32 {
        match self.session.run(ops) {
            Ok(()) => {
                self.said = (say, what.to_string());
                self.error = "";
                self.prune();
                REPAINT | CHANGED
            }
            Err(e) => {
                self.error = e.key;
                REPAINT
            }
        }
    }

    /// Drops selection, hover and flags that point at things that no longer exist.
    fn prune(&mut self) {
        let d = &self.session.doc;
        self.over.sel_atoms.retain(|&i| d.atom(i).is_some());
        self.over.sel_bonds.retain(|&i| d.bond(i).is_some());
        self.over.warn_atoms.retain(|&i| d.atom(i).is_some());
        self.over.hover_atom = self.over.hover_atom.filter(|&i| d.atom(i).is_some());
        self.over.hover_bond = self.over.hover_bond.filter(|&i| d.bond(i).is_some());
    }

    pub fn set_tool(&mut self, name: &str) -> u32 {
        match Tool::parse(name) {
            Some(t) => {
                self.tool = t;
                self.press = None;
                self.over.preview = None;
                self.over.band = None;
                REPAINT | TOOL
            }
            None => 0,
        }
    }

    pub fn undo(&mut self) -> u32 {
        if !self.session.undo() {
            return 0;
        }
        self.said = ("draw2.say.undo", String::new());
        self.prune();
        REPAINT | CHANGED
    }

    pub fn redo(&mut self) -> u32 {
        if !self.session.redo() {
            return 0;
        }
        self.said = ("draw2.say.redo", String::new());
        self.prune();
        REPAINT | CHANGED
    }

    /// Reach of the pointer in pixels.
    fn reach(kind: u32) -> f64 {
        match kind {
            TOUCH => 22.0,
            PEN => 16.0,
            _ => 10.0,
        }
    }

    /// The nearest atom within reach, else the nearest bond.
    pub fn hit(&self, px: f64, py: f64, kind: u32) -> Hit {
        let r = Self::reach(kind);
        let d = self.doc();
        let mut best = (r, Hit::None);
        for a in &d.atoms {
            let (x, y) = self.view.to_px(a.x, a.y);
            let dist = m::hypot(x - px, y - py);
            if dist < best.0 {
                best = (dist, Hit::Atom(a.id));
            }
        }
        if best.1 != Hit::None {
            return best.1;
        }
        best.0 = r * 0.7;
        for b in &d.bonds {
            let (Some(p), Some(q)) = (d.atom(b.a), d.atom(b.b)) else {
                continue;
            };
            let (s, e) = (self.view.to_px(p.x, p.y), self.view.to_px(q.x, q.y));
            let (dx, dy) = (e.0 - s.0, e.1 - s.1);
            let t = (((px - s.0) * dx + (py - s.1) * dy) / (dx * dx + dy * dy).max(1e-9))
                .clamp(0.0, 1.0);
            let dist = m::hypot(s.0 + t * dx - px, s.1 + t * dy - py);
            if dist < best.0 {
                best = (dist, Hit::Bond(b.id));
            }
        }
        best.1
    }

    /// Zooms by `factor` keeping the point under the pointer still (scale 8-240 px per bond).
    pub fn zoom(&mut self, px: f64, py: f64, factor: f64) -> u32 {
        let (x, y) = self.view.to_doc(px, py);
        self.view.scale = (self.view.scale * factor).clamp(8.0, 240.0);
        self.view.ox = px - x * self.view.scale;
        self.view.oy = py + y * self.view.scale;
        REPAINT
    }

    pub fn pan(&mut self, dx: f64, dy: f64) -> u32 {
        self.view.ox += dx;
        self.view.oy += dy;
        REPAINT
    }

    /// Centres the drawing in a `w` x `h` canvas, at most 40 px per bond.
    pub fn fit(&mut self, w: f64, h: f64) -> u32 {
        let d = self.doc();
        if d.atoms.is_empty() {
            self.view = View {
                scale: 40.0,
                ox: w / 2.0,
                oy: h / 2.0,
            };
            return REPAINT;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for a in &d.atoms {
            (x0, y0, x1, y1) = (x0.min(a.x), y0.min(a.y), x1.max(a.x), y1.max(a.y));
        }
        let scale = ((w - 80.0) / (x1 - x0).max(0.1))
            .min((h - 80.0) / (y1 - y0).max(0.1))
            .clamp(8.0, 40.0);
        self.view = View {
            scale,
            ox: w / 2.0 - (x0 + x1) / 2.0 * scale,
            oy: h / 2.0 + (y0 + y1) / 2.0 * scale,
        };
        REPAINT
    }

    /// Adds the structure of a text (SMILES, molfile, `mzdraw/1`) to the right of the drawing, as one undo step.
    pub fn import(&mut self, text: &str) -> Result<io::Imported, &'static str> {
        let (new, info) = io::import(text)?;
        let d = self.doc();
        let right = d.atoms.iter().map(|a| a.x).fold(f64::MIN, f64::max);
        let left = new.atoms.iter().map(|a| a.x).fold(f64::MAX, f64::min);
        let dx = if d.atoms.is_empty() || new.atoms.is_empty() {
            0.0
        } else {
            right + 2.0 - left
        };
        let base = d.next - 1;
        let mut ops = Vec::new();
        for a in &new.atoms {
            ops.push(Op::AtomAdd {
                id: Some(base + a.id),
                el: a.el.clone(),
                x: a.x + dx,
                y: a.y,
            });
            if a.chg != 0 || a.iso != 0 || a.rad != 0 || a.h.is_some() {
                let some = |v: bool| v.then_some(());
                ops.push(Op::AtomSet {
                    id: base + a.id,
                    el: None,
                    chg: some(a.chg != 0).map(|_| a.chg),
                    iso: some(a.iso != 0).map(|_| a.iso),
                    rad: some(a.rad != 0).map(|_| a.rad),
                    h: a.h,
                });
            }
        }
        for b in &new.bonds {
            ops.push(Op::BondAdd {
                id: Some(base + b.id),
                a: base + b.a,
                b: base + b.b,
                o: b.o,
            });
        }
        self.session.run(&ops).map_err(|e| e.key)?;
        self.said = ("draw2.say.imported", new.atoms.len().to_string());
        Ok(info)
    }

    /// Empties the canvas as one undo step.
    pub fn clear(&mut self) -> u32 {
        let atoms: Vec<u32> = self.doc().atoms.iter().map(|a| a.id).collect();
        if atoms.is_empty() {
            return 0;
        }
        self.run(
            &[Op::Remove {
                atoms,
                bonds: Vec::new(),
            }],
            "draw2.say.cleared",
            "",
        )
    }

    /// Marks the atoms a check flagged (ids), replacing the previous marks.
    pub fn set_warnings(&mut self, ids: Vec<u32>) {
        self.over.warn_atoms = ids;
    }

    pub fn scene(&self) -> Scene {
        scene::build(
            self.doc(),
            self.view,
            &self.over,
            (self.view.scale * 0.4).clamp(9.0, 40.0),
        )
    }
}
