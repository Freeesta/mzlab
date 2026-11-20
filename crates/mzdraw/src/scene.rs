//! The display list: the document, a view and the editor's transient state turned into flat primitives that any surface paints
//! without knowing chemistry. The page paints it on a canvas; `svg` writes the same list as a file.
//!
//! Layout of `data` (f32): `LINE x1 y1 x2 y2 width colour` · `TEXT x y size colour string anchor` (anchor 0 middle, 1 start) ·
//! `RECT x y w h colour` · `DISC x y r colour` · `RING x y r colour`. Coordinates are in output pixels, y down.

use crate::doc::{Atom, Document};
use crate::m;

pub const LINE: f32 = 1.0;
pub const TEXT: f32 = 2.0;
pub const RECT: f32 = 3.0;
pub const DISC: f32 = 4.0;
pub const RING: f32 = 5.0;

/// Colour indices; the surface owns the actual colours (CSS variables on the page, the style palette in a file).
pub mod colour {
    pub const INK: f32 = 0.0;
    pub const SELECT: f32 = 1.0;
    pub const HOVER: f32 = 2.0;
    pub const WARN: f32 = 3.0;
    pub const PREVIEW: f32 = 4.0;
    pub const N: f32 = 5.0;
    pub const O: f32 = 6.0;
    pub const S: f32 = 7.0;
    pub const HALOGEN: f32 = 8.0;
    pub const P: f32 = 9.0;
}

/// Document units to output pixels: `x' = ox + x * scale`, `y' = oy - y * scale`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub scale: f64,
    pub ox: f64,
    pub oy: f64,
}

impl View {
    pub fn to_px(&self, x: f64, y: f64) -> (f64, f64) {
        (self.ox + x * self.scale, self.oy - y * self.scale)
    }

    pub fn to_doc(&self, px: f64, py: f64) -> (f64, f64) {
        ((px - self.ox) / self.scale, (self.oy - py) / self.scale)
    }
}

/// What the editor adds on top of the document: selection, hover, flagged atoms, a moving selection, a preview bond, a rubber band.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    pub sel_atoms: Vec<u32>,
    pub sel_bonds: Vec<u32>,
    pub hover_atom: Option<u32>,
    pub hover_bond: Option<u32>,
    pub warn_atoms: Vec<u32>,
    /// Offset (document units) drawn on the selected atoms while they are dragged.
    pub drag: (f64, f64),
    /// A bond being drawn, in document units.
    pub preview: Option<(f64, f64, f64, f64)>,
    /// A rubber band, in pixels (x, y, w, h).
    pub band: Option<(f64, f64, f64, f64)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub data: Vec<f32>,
    pub strings: Vec<String>,
}

/// Approximate advance width of a label character, in units of the font size (Arial-like; the same table everywhere).
fn advance(c: char) -> f64 {
    match c {
        'I' | 'i' | 'l' => 0.28,
        '+' | '-' | '\u{2212}' => 0.58,
        '0'..='9' => 0.56,
        'a'..='z' => 0.5,
        'M' | 'W' => 0.9,
        _ => 0.7,
    }
}

fn width(s: &str, size: f64) -> f64 {
    s.chars().map(advance).sum::<f64>() * size
}

fn element_colour(el: &str) -> f32 {
    match el {
        "N" => colour::N,
        "O" => colour::O,
        "S" => colour::S,
        "P" => colour::P,
        "F" | "Cl" | "Br" | "I" => colour::HALOGEN,
        _ => colour::INK,
    }
}

/// Whether the atom is drawn as text (a carbon inside a chain is a bare vertex).
pub fn has_label(doc: &Document, a: &Atom) -> bool {
    a.el != "C"
        || a.chg != 0
        || a.iso != 0
        || a.rad != 0
        || a.h.is_some()
        || doc.order_sum(a.id) == 0
}

struct Painter<'a> {
    doc: &'a Document,
    view: View,
    over: &'a Overlay,
    font: f64,
    out: Scene,
}

impl Painter<'_> {
    fn pos(&self, a: &Atom) -> (f64, f64) {
        let moving = self.over.drag != (0.0, 0.0) && self.over.sel_atoms.contains(&a.id);
        let (dx, dy) = if moving { self.over.drag } else { (0.0, 0.0) };
        self.view.to_px(a.x + dx, a.y + dy)
    }

    fn push(&mut self, v: &[f64]) {
        self.out.data.extend(v.iter().map(|&x| x as f32));
    }

    fn line(&mut self, p: (f64, f64), q: (f64, f64), w: f64, c: f32) {
        self.push(&[LINE as f64, p.0, p.1, q.0, q.1, w, c as f64]);
    }

    fn text(&mut self, x: f64, y: f64, size: f64, c: f32, s: &str, anchor: f64) {
        let i = self.out.strings.len();
        self.out.strings.push(s.to_string());
        self.push(&[TEXT as f64, x, y, size, c as f64, i as f64, anchor]);
    }

    fn bond(&mut self, b: &crate::doc::Bond) {
        let (Some(p), Some(q)) = (self.doc.atom(b.a), self.doc.atom(b.b)) else {
            return;
        };
        let (mut s, mut e) = (self.pos(p), self.pos(q));
        let (dx, dy) = (e.0 - s.0, e.1 - s.1);
        let len = m::hypot(dx, dy);
        if len < 1e-6 {
            return;
        }
        let (ux, uy) = (dx / len, dy / len);
        // a bond stops short of a label
        let gap = (self.font * 0.62).min(len * 0.4);
        let (lp, lq) = (has_label(self.doc, p), has_label(self.doc, q));
        if lp {
            s = (s.0 + ux * gap, s.1 + uy * gap);
        }
        if lq {
            e = (e.0 - ux * gap, e.1 - uy * gap);
        }
        let w = (self.view.scale * 0.045).max(1.0);
        if self.over.sel_bonds.contains(&b.id) || self.over.hover_bond == Some(b.id) {
            let c = if self.over.sel_bonds.contains(&b.id) {
                colour::SELECT
            } else {
                colour::HOVER
            };
            self.line(s, e, w * 5.0, c);
        }
        let off = self.view.scale * 0.13;
        let (nx, ny) = (-uy * off, ux * off);
        let shift = |p: (f64, f64), k: f64| (p.0 + nx * k, p.1 + ny * k);
        match b.o {
            2 => {
                let side = self.double_side(b, (ux, uy));
                if side == 0.0 {
                    self.line(shift(s, 0.5), shift(e, 0.5), w, colour::INK);
                    self.line(shift(s, -0.5), shift(e, -0.5), w, colour::INK);
                } else {
                    self.line(s, e, w, colour::INK);
                    // the second line sits inside, a little shorter unless it ends at a label
                    let trim = len * 0.12;
                    let s2 = if lp {
                        s
                    } else {
                        (s.0 + ux * trim, s.1 + uy * trim)
                    };
                    let e2 = if lq {
                        e
                    } else {
                        (e.0 - ux * trim, e.1 - uy * trim)
                    };
                    self.line(shift(s2, side), shift(e2, side), w, colour::INK);
                }
            }
            3 => {
                for k in [-1.0, 0.0, 1.0] {
                    self.line(shift(s, k), shift(e, k), w, colour::INK);
                }
            }
            _ => self.line(s, e, w, colour::INK),
        }
    }

    /// Which side the second line of a double bond takes: the side with more substituents (the inside of a ring), 0 = centred
    /// (an end without other bonds, as in C=O).
    fn double_side(&self, b: &crate::doc::Bond, u: (f64, f64)) -> f64 {
        let (Some(p), Some(q)) = (self.doc.atom(b.a), self.doc.atom(b.b)) else {
            return 0.0;
        };
        let mut sum = 0.0;
        for (end, other) in [(p, q), (q, p)] {
            let subs: Vec<u32> = self
                .doc
                .neighbours(end.id)
                .into_iter()
                .filter(|&n| n != other.id)
                .collect();
            if subs.is_empty() {
                return 0.0;
            }
            let o = self.pos(end);
            for n in subs.iter().filter_map(|&n| self.doc.atom(n)) {
                let m = self.pos(n);
                sum += (u.0 * (m.1 - o.1) - u.1 * (m.0 - o.0)).signum();
            }
        }
        if sum < 0.0 {
            -1.0
        } else {
            1.0
        }
    }

    /// The label of an atom: symbol centred on the atom, hydrogens on the free side, isotope and charge raised.
    fn label(&mut self, a: &Atom) {
        let (x, y) = self.pos(a);
        let f = self.font;
        let c = element_colour(&a.el);
        let small = f * 0.7;
        self.text(x, y, f, c, &a.el, 0.0);
        let half = width(&a.el, f) / 2.0;
        if a.iso != 0 {
            let s = a.iso.to_string();
            self.text(x - half - width(&s, small), y - f * 0.35, small, c, &s, 1.0);
        }
        let h = self.doc.hydrogens(a);
        // hydrogens go left when the bonds leave to the right
        let lean: f64 = self
            .doc
            .neighbours(a.id)
            .iter()
            .filter_map(|&n| self.doc.atom(n))
            .map(|n| n.x - a.x)
            .sum();
        let left = lean > 0.01;
        let count = if h > 1 { h.to_string() } else { String::new() };
        let hw = if h > 0 {
            width("H", f) + width(&count, small)
        } else {
            0.0
        };
        if h > 0 {
            let hx = if left { x - half - hw } else { x + half };
            self.text(hx, y, f, c, "H", 1.0);
            if h > 1 {
                self.text(hx + width("H", f), y + f * 0.3, small, c, &count, 1.0);
            }
        }
        let mut mark = String::new();
        if a.chg != 0 {
            if a.chg.abs() > 1 {
                mark.push_str(&a.chg.abs().to_string());
            }
            mark.push(if a.chg > 0 { '+' } else { '\u{2212}' });
        }
        mark.push_str(&"\u{2022}".repeat(a.rad as usize));
        if !mark.is_empty() {
            let right = x + half + if left { 0.0 } else { hw };
            self.text(right, y - f * 0.35, small, c, &mark, 1.0);
        }
    }

    fn atoms(&mut self) {
        let r = self.font * 0.7;
        for a in &self.doc.atoms {
            let (x, y) = self.pos(a);
            if self.over.sel_atoms.contains(&a.id) {
                self.push(&[DISC as f64, x, y, r, colour::SELECT as f64]);
            } else if self.over.hover_atom == Some(a.id) {
                self.push(&[DISC as f64, x, y, r, colour::HOVER as f64]);
            }
            if self.over.warn_atoms.contains(&a.id) {
                self.push(&[RING as f64, x, y, r * 1.15, colour::WARN as f64]);
            }
        }
        for a in &self.doc.atoms {
            if has_label(self.doc, a) {
                self.label(a);
            }
        }
    }
}

/// Builds the display list of a document under a view. `font` is the label size in output pixels.
pub fn build(doc: &Document, view: View, over: &Overlay, font: f64) -> Scene {
    let mut p = Painter {
        doc,
        view,
        over,
        font,
        out: Scene::default(),
    };
    for b in &doc.bonds {
        p.bond(b);
    }
    p.atoms();
    if let Some((x1, y1, x2, y2)) = over.preview {
        let w = (view.scale * 0.045).max(1.0);
        p.line(view.to_px(x1, y1), view.to_px(x2, y2), w, colour::PREVIEW);
    }
    if let Some((x, y, w, h)) = over.band {
        p.push(&[RECT as f64, x, y, w, h, colour::SELECT as f64]);
    }
    p.out
}
