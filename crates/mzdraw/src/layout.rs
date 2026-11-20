//! The single door to 2D layout. The real layout is `Mol::layout_2d` of `mzlab-mol` (agent drawing); until it exists `clean_up` leaves
//! the drawing as it is and says so, and structures that arrive without coordinates get placeholder positions (each piece on a regular
//! polygon), marked as such.

use crate::doc::{round4, Document};
use crate::m;
use std::f64::consts::{PI, TAU};

/// What `clean_up` did: `unavailable` until the layout engine is linked.
pub const CLEAN_UP_STATUS: &str = "unavailable";

/// New coordinates for the atoms (id, x, y); empty when nothing moves.
pub fn clean_up(_doc: &Document) -> Vec<(u32, f64, f64)> {
    Vec::new()
}

/// The connected pieces of a document, as lists of atom ids in pool order.
pub fn pieces(doc: &Document) -> Vec<Vec<u32>> {
    let mut seen: Vec<u32> = Vec::new();
    let mut out = Vec::new();
    for a in &doc.atoms {
        if seen.contains(&a.id) {
            continue;
        }
        let mut piece = vec![a.id];
        let mut k = 0;
        while k < piece.len() {
            for n in doc.neighbours(piece[k]) {
                if !piece.contains(&n) {
                    piece.push(n);
                }
            }
            k += 1;
        }
        seen.extend(&piece);
        out.push(piece);
    }
    out
}

/// Placeholder positions for a document without coordinates: every piece on a regular polygon of side 1, pieces left to right.
pub fn placeholder(doc: &mut Document) {
    let mut left = 0.0;
    for piece in pieces(doc) {
        let n = piece.len();
        let r = if n > 1 {
            1.0 / (2.0 * m::sin(PI / n as f64))
        } else {
            0.0
        };
        for (i, id) in piece.iter().enumerate() {
            let t = PI / 2.0 - TAU * i as f64 / n as f64;
            if let Some(k) = doc.atom_index(*id) {
                doc.atoms[k].x = round4(left + r + r * m::cos(t));
                doc.atoms[k].y = round4(r * m::sin(t));
            }
        }
        left += 2.0 * r + 1.5;
    }
}
