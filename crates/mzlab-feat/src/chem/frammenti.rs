//! Cuts of a drawn structure (the page sends a V2000 molfile, see `Mol::from_molfile`): the two pieces of a bond with formula, neutral mass and m/z of [F+H]+.

use mzlab_core::elements;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Out;

use super::grafo::{add_h, hydrolysable, Mol};

/// Atoms reached from `start` without crossing bond `skip`.
fn reach(m: &Mol, start: usize, skip: usize) -> Vec<bool> {
    let mut seen = vec![false; m.atoms.len()];
    seen[start] = true;
    let mut st = vec![start];
    while let Some(u) = st.pop() {
        for (v, kk) in &m.adj[u] {
            if *kk != skip && !seen[*v] {
                seen[*v] = true;
                st.push(*v);
            }
        }
    }
    seen
}

/// `{"atoms":[..],"formula","mass","mz"}` of one side of a cut: the side closes with `h` hydrogens (the bond order, as in `cleavages`), `mz` is [F+H]+.
fn side_json(o: &mut Out, m: &Mol, keep: &[bool], h: u64) {
    let f = add_h(m.formula_of(keep), h, 0);
    let mass = elements::mass(&f);
    o.raw("{\"atoms\":[");
    for (n, i) in (0..keep.len()).filter(|i| keep[*i]).enumerate() {
        o.raw(if n > 0 { "," } else { "" }).int(i as i64);
    }
    o.raw("],\"formula\":").str(&elements::fmt(&f));
    o.raw(",\"mass\":").num(mzlab_core::json::round(mass, 4));
    let mz = elements::ion_mz(mass, "[M+H]+").unwrap_or(mass);
    o.raw(",\"mz\":")
        .num(mzlab_core::json::round(mz, 4))
        .raw("}");
}

/// `frag/1`: the cuts of a molfile as `{"cuts":[{"a","b","order","kind","sides":[{atoms,formula,mass,mz},..]}]}`. `a`, `b` are atom positions of the
/// file (0-based); with both >= 0 only the bond between them, otherwise every bond outside the rings. A ring bond gives `error.grafo.ring`.
pub fn fragments_json(molfile: &str, a: i32, b: i32) -> Result<String> {
    let m = Mol::from_molfile(molfile)?;
    let on_ring = m.ring_bonds();
    let one = a >= 0 && b >= 0;
    let ks: Vec<usize> = (0..m.bonds.len())
        .filter(|k| {
            let bd = &m.bonds[*k];
            !one || (bd.a as i32, bd.b as i32) == (a, b) || (bd.a as i32, bd.b as i32) == (b, a)
        })
        .collect();
    if one && ks.is_empty() {
        return Err(Error::new("error.grafo.molfile"));
    }
    if one && on_ring[ks[0]] {
        return Err(Error::new("error.grafo.ring"));
    }
    let mut o = Out::default();
    o.raw("{\"format\":\"frag/1\",\"cuts\":[");
    let mut first = true;
    for k in ks.into_iter().filter(|k| !on_ring[*k]) {
        let bd = m.bonds[k];
        let h = if bd.order == 1.5 { 1 } else { bd.order as u64 };
        o.raw(if first { "" } else { "," });
        first = false;
        o.raw("{\"a\":")
            .int(bd.a as i64)
            .raw(",\"b\":")
            .int(bd.b as i64);
        o.raw(",\"order\":")
            .num(bd.order as f64)
            .raw(",\"kind\":")
            .str(hydrolysable(&m, k));
        o.raw(",\"sides\":[");
        side_json(&mut o, &m, &reach(&m, bd.a, k), h);
        o.raw(",");
        side_json(&mut o, &m, &reach(&m, bd.b, k), h);
        o.raw("]}");
    }
    o.raw("]}");
    Ok(o.0)
}

#[cfg(test)]
mod molfile_tests {
    use super::*;
    use crate::chem::grafo::Mol;

    const ETHANOL: &str = "\n  qqq\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    2.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\n  2  3  1  0  0  0  0\nM  END";

    #[test]
    fn ethanol_molfile_matches_smiles() {
        let m = Mol::from_molfile(ETHANOL).unwrap();
        assert_eq!(
            elements::fmt(&m.formula()),
            elements::fmt(&Mol::parse("CCO").unwrap().formula())
        );
    }

    #[test]
    fn cut_gives_two_sides() {
        let j = fragments_json(ETHANOL, 1, 2).unwrap();
        assert!(j.contains("\"formula\":\"C2H6\""), "{j}");
        assert!(j.contains("\"formula\":\"H2O\""), "{j}");
        assert!(j.contains("\"mz\":19.0178"), "{j}");
    }
}
