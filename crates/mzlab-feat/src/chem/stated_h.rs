//! The hydrogen that a molfile states for an atom of a ring written aromatic (`MRV_IMPLICIT_H` data group, how Ketcher says which N of
//! imidazole carries the H), read for `graph::Mol::from_molfile` with the meaning `mzlab-mol` gives it.

use std::collections::BTreeMap;

use mzlab_core::error::{Error, Result};

use super::smiles::{self, RawAtom, RawBond};

/// More hydrogens than any atom carries: a larger count in an `MRV_IMPLICIT_H` group is a damaged file (same bound as `mzlab-mol`).
const MAX_STATED_H: u32 = 4;

/// The (atom index, hydrogens) that the `MRV_IMPLICIT_H` data groups of a molfile state (`M  SDT g MRV_IMPLICIT_H`, `M  SED g IMPL_Hn`,
/// `M  SAL g n atoms`): how Ketcher says which atom of an aromatic ring carries the hydrogen. One pass, up to `M  END`.
fn read(lines: &[&str], na: usize) -> Result<Vec<(usize, u32)>> {
    // data groups by number: (atoms, field name, data)
    let mut data: BTreeMap<&str, (Vec<usize>, &str, &str)> = BTreeMap::new();
    for line in lines.iter().take_while(|l| !l.starts_with("M  END")) {
        let kind = line.starts_with("M  ").then(|| line.get(3..6)).flatten();
        let Some(kind @ ("SAL" | "SDT" | "SED")) = kind else {
            continue;
        };
        let mut t = line[6..].split_whitespace();
        let Some(group) = t.next() else { continue };
        let g = data.entry(group).or_default();
        match kind {
            "SAL" => {
                let atoms = t.skip(1).filter_map(|a| a.parse::<usize>().ok());
                g.0.extend(atoms.filter(|a| (1..=na).contains(a)).map(|a| a - 1));
            }
            "SDT" => g.1 = t.next().unwrap_or(""),
            _ => g.2 = t.next().unwrap_or(""),
        }
    }
    let mut out = vec![];
    for (list, field, value) in data.values() {
        let count =
            (*field == "MRV_IMPLICIT_H").then(|| value.trim_matches('"').strip_prefix("IMPL_H"));
        let Some(count) = count.flatten() else {
            continue;
        };
        match count.parse::<u32>() {
            Ok(h) if h <= MAX_STATED_H => out.extend(list.iter().map(|&a| (a, h))),
            _ => return Err(Error::new("error.grafo.molfile")),
        }
    }
    Ok(out)
}

/// The stated hydrogens of the atoms of a ring written aromatic, with the meaning `mzlab-mol` gives them: an atom whose bonds and stated
/// hydrogens fill its valence gives no electron to the ring (pyrrole-type N) and its hydrogens are those the valence leaves; any other
/// statement changes nothing (an aromatic CH already has its H), and so does one on an atom outside an aromatic ring.
pub(super) fn apply(raw: &mut [RawAtom], written: &[RawBond], lines: &[&str]) -> Result<()> {
    let stated = read(lines, raw.len())?;
    let mut used = vec![0f32; raw.len()];
    for &(a, b, order, _) in written {
        let v = if order == 1.5 { 1.0 } else { order };
        used[a] += v;
        used[b] += v;
    }
    for (i, h) in stated {
        let r = &mut raw[i];
        let valence = smiles::valence_for(&r.el, r.charge, true, used[i]);
        if let Some(v) = valence.filter(|v| r.arom && h > 0 && used[i] + h as f32 >= *v) {
            r.h = Some((v - used[i]).round().max(0.0) as u32);
        }
    }
    Ok(())
}
