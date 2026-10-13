//! Cuts of a drawn structure (the page sends a V2000 molfile, see `Mol::from_molfile`): the two pieces of a bond with formula, neutral mass and m/z of [F+H]+.

use mzlab_core::elements::{self, Formula};
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

/// A delta of a partial structure, «+O -H2»: signed formulas (spaces optional, «+O-H2» too). Empty text = no delta. `error.grafo.delta` with the text otherwise.
pub fn parse_delta(s: &str) -> Result<Vec<(i64, Formula)>> {
    let bad = || Error::new("error.grafo.delta").with("delta", s.trim());
    let c: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out = vec![];
    let mut rest = c.as_str();
    while !rest.is_empty() {
        let sign = match rest.chars().next() {
            Some('+') => 1,
            Some('-') | Some('\u{2212}') => -1,
            _ => return Err(bad()),
        };
        rest = &rest[rest.chars().next().map_or(1, char::len_utf8)..];
        let end = rest.find(['+', '-', '\u{2212}']).unwrap_or(rest.len());
        let f = elements::parse_formula(&rest[..end]).map_err(|_| bad())?;
        out.push((sign, f));
        rest = &rest[end..];
    }
    Ok(out)
}

/// The delta written back in the same order, one signed Hill formula per term: «+O -H2».
pub fn fmt_delta(d: &[(i64, Formula)]) -> String {
    d.iter()
        .map(|(s, f)| format!("{}{}", if *s > 0 { "+" } else { "-" }, elements::fmt(f)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `f` plus the delta; an element that would go below zero gives `error.grafo.delta.neg`.
pub fn apply_delta(f: &Formula, d: &[(i64, Formula)]) -> Result<Formula> {
    let mut acc: Vec<(String, i64)> = f.iter().map(|(e, n)| (e.clone(), *n as i64)).collect();
    for (s, g) in d {
        for (e, n) in g {
            match acc.iter_mut().find(|(x, _)| x == e) {
                Some(x) => x.1 += s * *n as i64,
                None => acc.push((e.clone(), s * *n as i64)),
            }
        }
    }
    if let Some((e, _)) = acc.iter().find(|(_, n)| *n < 0) {
        return Err(Error::new("error.grafo.delta.neg").with("element", e));
    }
    Ok(acc
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .map(|(e, n)| (e, n as u64))
        .collect())
}

/// `parz/1`: a partial structure (a drawn piece whose exact position of a change is unknown) as `{"format","formula","mass","mz","delta","level":3}`:
/// the formula of the molfile plus the delta, its neutral mass and the m/z of [M+H]+. Level 3 of Schymanski: a candidate class, never one structure.
pub fn partial_json(molfile: &str, delta: &str) -> Result<String> {
    let m = Mol::from_molfile(molfile)?;
    let d = parse_delta(delta)?;
    let f = apply_delta(&m.formula(), &d)?;
    let mass = elements::mass(&f);
    let mz = elements::ion_mz(mass, "[M+H]+").unwrap_or(mass);
    let mut o = Out::default();
    o.raw("{\"format\":\"parz/1\",\"formula\":")
        .str(&elements::fmt(&f));
    o.raw(",\"mass\":").num(mzlab_core::json::round(mass, 4));
    o.raw(",\"mz\":").num(mzlab_core::json::round(mz, 4));
    o.raw(",\"delta\":")
        .str(&fmt_delta(&d))
        .raw(",\"level\":3}");
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

    #[test]
    fn delta_parses_and_applies() {
        let d = parse_delta(" +O -H2 ").unwrap();
        assert_eq!(fmt_delta(&d), "+O -H2");
        assert_eq!(fmt_delta(&parse_delta("+O-H2").unwrap()), "+O -H2");
        assert!(parse_delta("").unwrap().is_empty());
        assert_eq!(parse_delta("O").unwrap_err().key, "error.grafo.delta");
        assert_eq!(parse_delta("+Xx").unwrap_err().key, "error.grafo.delta");
        let j = partial_json(ETHANOL, "+O -H2").unwrap();
        assert!(j.contains("\"formula\":\"C2H4O2\""), "{j}");
        assert!(
            j.contains("\"level\":3") && j.contains("\"delta\":\"+O -H2\""),
            "{j}"
        );
        assert_eq!(
            partial_json(ETHANOL, "-N").unwrap_err().key,
            "error.grafo.delta.neg"
        );
    }
}
