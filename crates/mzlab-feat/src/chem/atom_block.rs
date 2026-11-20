//! The atom block of the V2000 molfile of a drawing for `graph::Mol::from_molfile`: element, the charge of the legacy column and the
//! stated valence of every atom line, read as `mzlab-mol` reads them.

use mzlab_core::elements;
use mzlab_core::error::{Error, Result};

use super::smiles::{normal_valence, RawAtom};

/// The atoms of the atom block and the valence each line states (None: left to the rules). `column`: the charge of the atom line
/// counts, which it does only in a file with no `M  CHG` line (as in `mzlab-mol`).
pub(super) fn atom_block(lines: &[&str], column: bool) -> Result<(Vec<RawAtom>, Vec<Option<u8>>)> {
    let num = |s: &str, a: usize, b: usize| s.get(a..b).and_then(|t| t.trim().parse::<i64>().ok());
    let (mut raw, mut valence) = (Vec::new(), Vec::new());
    for l in lines {
        let el = l
            .get(31..34)
            .ok_or_else(|| Error::new("error.grafo.molfile"))?;
        let el = el.trim().to_string();
        if elements::mass_of(&el).is_none() || normal_valence(&el).is_none() {
            return Err(Error::new("error.grafo.element").with("element", &el));
        }
        let charge = match num(l, 36, 39) {
            Some(c @ (1..=3 | 5..=7)) if column => 4 - c as i32,
            _ => 0,
        };
        valence.push(match num(l, 48, 51) {
            Some(15) => Some(0),
            Some(v @ 1..=14) => Some(v as u8),
            _ => None,
        });
        raw.push(RawAtom {
            el,
            arom: false,
            h: None,
            charge,
        });
    }
    Ok((raw, valence))
}

/// Whether a line `prefix` comes before the `M  END` of `lines`.
pub(super) fn has(lines: &[&str], prefix: &str) -> bool {
    let mut lines = lines.iter().take_while(|l| !l.starts_with("M  END"));
    lines.any(|l| l.starts_with(prefix))
}
