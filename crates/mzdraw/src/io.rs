//! Bridges between the document and the `mol/1` record of `mzlab-mol`, whose readers and writers do all the format work: SMILES and
//! molfile in, molfile out. Nothing is parsed here.

use crate::doc::{round4, Atom, Bond, Document};
use crate::layout;
use crate::m;
use mzlab_mol::{Coords, Mol};

/// How an import went: the format read and whether the positions are placeholders (the source had no coordinates).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Imported {
    pub format: &'static str,
    pub placeholder: bool,
}

impl Document {
    /// The document as a `mol/1` record: atoms in pool order, hydrogens as the labels show them, the drawn coordinates as a layer.
    pub fn to_mol(&self) -> Mol {
        let atoms = self
            .atoms
            .iter()
            .map(|a| mzlab_mol::Atom {
                el: a.el.clone(),
                iso: a.iso,
                chg: a.chg,
                rad: a.rad,
                h: self.hydrogens(a) as u8,
            })
            .collect();
        let at = |id: u32| self.atom_index(id).unwrap_or(0) as u32;
        let bonds = self
            .bonds
            .iter()
            .map(|b| {
                let (i, j) = (at(b.a), at(b.b));
                mzlab_mol::Bond {
                    a: i.min(j),
                    b: i.max(j),
                    o: b.o,
                }
            })
            .collect();
        let mut m = Mol::new(atoms, bonds, Vec::new(), Vec::new());
        if !self.atoms.is_empty() {
            let xy = self.atoms.iter().map(|a| vec![a.x, a.y]).collect();
            m.coords = Some(Coords {
                dim: 2,
                xy,
                from: crate::NAME.into(),
            });
        }
        m
    }

    /// A document from a record: ids 1.. in record order; coordinates scaled so that the median bond is 1. Returns whether the
    /// positions are placeholders.
    pub fn from_mol(m: &Mol) -> (Document, bool) {
        let n = m.atoms.len() as u32;
        let mut d = Document {
            next: n + m.bonds.len() as u32 + 1,
            ..Document::default()
        };
        for (i, a) in m.atoms.iter().enumerate() {
            let p = m.coords.as_ref().and_then(|c| c.xy.get(i));
            let (x, y) = p.map_or((0.0, 0.0), |p| (p[0], p[1]));
            d.atoms.push(Atom {
                id: i as u32 + 1,
                el: a.el.clone(),
                x,
                y,
                chg: a.chg,
                iso: a.iso,
                rad: a.rad,
                h: None,
            });
        }
        for (i, b) in m.bonds.iter().enumerate() {
            d.bonds.push(Bond {
                id: n + i as u32 + 1,
                a: b.a + 1,
                b: b.b + 1,
                o: b.o,
            });
        }
        // keep a hydrogen count only where the valence rules would show another one
        for (i, a) in m.atoms.iter().enumerate() {
            if d.hydrogens(&d.atoms[i]) != a.h as u32 {
                d.atoms[i].h = Some(a.h);
            }
        }
        let placeholder = m.coords.is_none() && !m.atoms.is_empty();
        if placeholder {
            layout::placeholder(&mut d);
        } else {
            d.normalise();
        }
        (d, placeholder)
    }

    /// Scales the coordinates so that the median bond length is 1 (a drawing without bonds is left alone).
    fn normalise(&mut self) {
        let mut len: Vec<f64> = self
            .bonds
            .iter()
            .filter_map(|b| Some((self.atom(b.a)?, self.atom(b.b)?)))
            .map(|(p, q)| m::hypot(p.x - q.x, p.y - q.y))
            .collect();
        len.sort_by(f64::total_cmp);
        let k = len
            .get(len.len() / 2)
            .copied()
            .filter(|&m| m > 1e-6)
            .map_or(1.0, |m| 1.0 / m);
        for a in &mut self.atoms {
            a.x = round4(a.x * k);
            a.y = round4(a.y * k);
        }
    }

    /// The whole canvas as one molfile (V2000, or V3000 when it has to be), written by `mzlab-mol`.
    pub fn to_molfile(&self) -> String {
        self.to_mol().to_molfile(crate::NAME)
    }
}

/// Reads a structure text: an `mzdraw/1` document, a molfile (V2000/V3000) or a SMILES. The error is an i18n key.
pub fn import(text: &str) -> Result<(Document, Imported), &'static str> {
    let t = text.trim();
    if t.starts_with('{') {
        let d = Document::from_json(t)?;
        return Ok((
            d,
            Imported {
                format: "mzdraw",
                placeholder: false,
            },
        ));
    }
    if text.contains("M  END") {
        let m = Mol::from_molfile(text).map_err(|_| "error.draw2.molfile")?;
        let (d, placeholder) = Document::from_mol(&m);
        return Ok((
            d,
            Imported {
                format: "molfile",
                placeholder,
            },
        ));
    }
    if t.is_empty() || t.lines().count() > 1 {
        return Err("error.draw2.import");
    }
    let m = Mol::from_smiles(t).map_err(|_| "error.draw2.smiles")?;
    let (d, placeholder) = Document::from_mol(&m);
    Ok((
        d,
        Imported {
            format: "smiles",
            placeholder,
        },
    ))
}
