//! The document `mzdraw/1`: atoms and bonds in flat pools with stable ids and 2D coordinates in bond lengths (y up, as in a molfile).
//! A structure is a connected piece; nothing chemical is derived here except the implicit hydrogens a label shows.

use serde::{Deserialize, Serialize};

/// The value of the `format` field.
pub const FORMAT: &str = "mzdraw/1";

fn is_zero<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// One atom. `h` is set only when the hydrogens were given explicitly (a file said so); otherwise they follow the valence rules.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Atom {
    pub id: u32,
    pub el: String,
    pub x: f64,
    pub y: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub chg: i8,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub iso: u16,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rad: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<u8>,
}

/// One bond between the atoms with ids `a` and `b`, Kekulé order 1-3.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bond {
    pub id: u32,
    pub a: u32,
    pub b: u32,
    pub o: u8,
}

/// The document. `next` is the next free id (atoms and bonds share one counter), so replaying a log gives the same ids.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub format: String,
    pub next: u32,
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
}

impl Default for Document {
    fn default() -> Self {
        Document {
            format: FORMAT.into(),
            next: 1,
            atoms: Vec::new(),
            bonds: Vec::new(),
        }
    }
}

/// Coordinates are kept to 4 decimals so that the same edits give the same bytes everywhere.
pub fn round4(v: f64) -> f64 {
    let r = (v * 1e4).round() / 1e4;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

impl Document {
    /// Compact JSON with a fixed field order: same document, same bytes.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Reads and checks a document; the error is an i18n key.
    pub fn from_json(text: &str) -> Result<Document, &'static str> {
        let d: Document = serde_json::from_str(text).map_err(|_| "error.draw2.document")?;
        if d.format != FORMAT || !d.consistent() {
            return Err("error.draw2.document");
        }
        Ok(d)
    }

    /// Ids unique and below `next`, bonds between two different existing atoms, orders 1-3, one bond per pair.
    fn consistent(&self) -> bool {
        let mut ids: Vec<u32> = self.atoms.iter().map(|a| a.id).collect();
        ids.extend(self.bonds.iter().map(|b| b.id));
        ids.sort_unstable();
        let unique = ids.windows(2).all(|w| w[0] != w[1]);
        let mut pairs: Vec<(u32, u32)> = self
            .bonds
            .iter()
            .map(|b| (b.a.min(b.b), b.a.max(b.b)))
            .collect();
        pairs.sort_unstable();
        unique
            && ids.iter().all(|&i| i < self.next)
            && pairs.windows(2).all(|w| w[0] != w[1])
            && self.atoms.iter().all(|a| mzlab_mol::is_element(&a.el))
            && self.bonds.iter().all(|b| {
                b.a != b.b
                    && (1..=3).contains(&b.o)
                    && self.atom(b.a).is_some()
                    && self.atom(b.b).is_some()
            })
    }

    pub fn atom_index(&self, id: u32) -> Option<usize> {
        self.atoms.iter().position(|a| a.id == id)
    }

    pub fn atom(&self, id: u32) -> Option<&Atom> {
        self.atoms.iter().find(|a| a.id == id)
    }

    pub fn bond_index(&self, id: u32) -> Option<usize> {
        self.bonds.iter().position(|b| b.id == id)
    }

    pub fn bond(&self, id: u32) -> Option<&Bond> {
        self.bonds.iter().find(|b| b.id == id)
    }

    /// The bond between two atoms, in either direction.
    pub fn bond_between(&self, a: u32, b: u32) -> Option<&Bond> {
        self.bonds
            .iter()
            .find(|x| (x.a == a && x.b == b) || (x.a == b && x.b == a))
    }

    /// Ids of the atoms bonded to `id`, in bond order.
    pub fn neighbours(&self, id: u32) -> Vec<u32> {
        self.bonds
            .iter()
            .filter_map(|b| match (b.a == id, b.b == id) {
                (true, _) => Some(b.b),
                (_, true) => Some(b.a),
                _ => None,
            })
            .collect()
    }

    /// Sum of the bond orders on an atom.
    pub fn order_sum(&self, id: u32) -> u32 {
        self.bonds
            .iter()
            .filter(|b| b.a == id || b.b == id)
            .map(|b| b.o as u32)
            .sum()
    }

    /// Hydrogens the atom carries: the explicit count, or what the lowest allowed valence leaves free.
    pub fn hydrogens(&self, a: &Atom) -> u32 {
        if let Some(h) = a.h {
            return h as u32;
        }
        let used = self.order_sum(a.id) + a.rad as u32;
        mzlab_mol::allowed_valences(&a.el, a.chg as i32)
            .into_iter()
            .find(|&v| v >= used)
            .map_or(0, |v| v - used)
    }
}
