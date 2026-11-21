//! A V3000 molfile as the V2000 text the molfile readers of the graph work on (`graph::Mol::from_molfile`, `cut`, `mech`, `selection/1`), so
//! both formats give the same graph for the same molecule. A small reader of its own (no `mzlab-mol` reader, no hash map: the `.wasm` `grafo`
//! stays small): atoms with `CHG`, `RAD`, `MASS`, `VAL`, bonds, the `MRV_IMPLICIT_H` data groups, the counts line checked against the blocks.
//! The graph takes at most `MAX_ATOMS` atoms, far under the 999 a V2000 file holds, so nothing is lost; a bigger file is refused for its
//! size, one that cannot be read is an error with its reason, never an empty molecule. Wedges are not carried (the graph holds no stereo).

use std::borrow::Cow;

use mzlab_core::elements;
use mzlab_core::error::{Error, Result};

use super::graph::MAX_ATOMS;
use super::smiles::normal_valence;

fn bad(message: &str) -> Error {
    Error::new("error.grafo.molfile").with("message", format!("molfile: {message}"))
}

/// Whitespace-separated tokens of a V3000 line, a `( ... )` list or a quoted text kept whole.
fn tokens(s: &str) -> Vec<&str> {
    let (mut out, mut from, mut depth, mut quoted) = (Vec::new(), None, 0i32, false);
    for (i, c) in s.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            _ => {}
        }
        if c.is_whitespace() && depth <= 0 && !quoted {
            out.extend(from.take().map(|f| &s[f..i]));
        } else {
            from.get_or_insert(i);
        }
    }
    out.extend(from.map(|f| &s[f..]));
    out
}

/// The `V30` lines with their `-` continuations joined, without the prefix.
fn logical_lines(record: &str) -> Vec<String> {
    let (mut out, mut open) = (Vec::new(), String::new());
    for l in record.lines().filter(|l| l.starts_with("M  V30")) {
        let body = l[6..].trim();
        match body.strip_suffix('-') {
            Some(head) => open = open + head + " ",
            None => out.push(std::mem::take(&mut open) + body),
        }
    }
    out
}

fn value<'a>(toks: &[&'a str], key: &str) -> Option<&'a str> {
    toks.iter()
        .find_map(|t| t.strip_prefix(key).and_then(|r| r.strip_prefix('=')))
}

fn num<T: std::str::FromStr>(t: &str) -> Result<T> {
    t.parse().map_err(|_| bad(&format!("{t} is not a number")))
}

type Props = Vec<(&'static str, usize, i64)>;

/// The id and the V2000 atom line of the `i`-th atom line `t`; its charge, isotope and radical go to `props` (key, atom, value).
fn atom_line(t: &[&str], l: &str, i: usize, props: &mut Props) -> Result<(usize, String)> {
    if t.len() < 5 {
        return Err(bad(&format!("the atom line {l:?} is too short")));
    }
    if i >= MAX_ATOMS {
        return Err(Error::new("error.grafo.tooBig").with("max", MAX_ATOMS));
    }
    let (el, mass) = match t[1] {
        "D" => ("H", 2),
        "T" => ("H", 3),
        e => (e, 0),
    };
    if elements::mass_of(el).is_none() || normal_valence(el).is_none() {
        return Err(Error::new("error.grafo.element").with("element", el));
    }
    let mass =
        value(&t[5..], "MASS").map_or(Ok(mass), |m| num::<f64>(m).map(|m| m.round() as i64))?;
    let valence = match value(&t[5..], "VAL").map(num::<i64>).transpose()? {
        Some(-1) => 15,
        Some(v @ 1..=14) => v,
        _ => 0,
    };
    for (key, v) in [
        ("M  CHG", value(&t[5..], "CHG")),
        ("M  RAD", value(&t[5..], "RAD")),
    ] {
        if let Some(v) = v.map(num::<i64>).transpose()?.filter(|v| *v != 0) {
            props.push((key, i, v));
        }
    }
    if mass != 0 {
        props.push(("M  ISO", i, mass));
    }
    let xyz = [num::<f64>(t[2])?, num::<f64>(t[3])?, num::<f64>(t[4])?];
    let line = format!(
        "{:>10.4}{:>10.4}{:>10.4} {el:<3} 0  0  0  0  0  0{valence:>3}  0  0  0  0\n",
        xyz[0], xyz[1], xyz[2]
    );
    Ok((num(t[0])?, line))
}

/// The V2000 bond line of the bond line `t`, its atoms looked up in `ids`.
fn bond_line(t: &[&str], l: &str, ids: &[usize]) -> Result<String> {
    if t.len() < 4 {
        return Err(bad(&format!("the bond line {l:?} is too short")));
    }
    let at = |s: &str| -> Result<usize> {
        let id: usize = num(s)?;
        ids.iter()
            .position(|x| *x == id)
            .ok_or_else(|| bad(&format!("a bond names the unknown atom {s}")))
    };
    // 1 to 4 as written, any other type (a query bond) is read as single, as `mzlab-mol` reads it
    let ty = num::<u32>(t[1])?;
    let ty = if (1..=4).contains(&ty) { ty } else { 8 };
    Ok(format!(
        "{:>3}{:>3}{ty:>3}  0  0  0  0\n",
        at(t[2])? + 1,
        at(t[3])? + 1
    ))
}

/// The `M  CHG`, `M  ISO` and `M  RAD` lines of `props`, eight atoms to a line.
fn prop_lines(props: &Props) -> String {
    let mut out = String::new();
    for key in ["M  CHG", "M  ISO", "M  RAD"] {
        let mine: Vec<_> = props.iter().filter(|p| p.0 == key).collect();
        for chunk in mine.chunks(8) {
            out += &format!("{key}{:>3}", chunk.len());
            for (_, i, v) in chunk {
                out += &format!(" {:>3} {v:>3}", i + 1);
            }
            out += "\n";
        }
    }
    out
}

/// The hydrogens of an `MRV_IMPLICIT_H` data group, written as the V2000 data group the graph reader takes (it checks the count).
fn data_groups(groups: &[Vec<&str>], ids: &[usize]) -> String {
    let (mut out, mut g) = (String::new(), 0);
    for t in groups {
        let data = value(t, "FIELDDATA").unwrap_or("").trim_matches('"');
        if value(t, "FIELDNAME") != Some("MRV_IMPLICIT_H") || !data.starts_with("IMPL_H") {
            continue;
        }
        let list = value(t, "ATOMS").unwrap_or("").trim_matches(['(', ')']);
        for id in list.split_whitespace().skip(1) {
            if let Some(a) = id
                .parse()
                .ok()
                .and_then(|id: usize| ids.iter().position(|x| *x == id))
            {
                g += 1;
                out += &format!(
                    "M  STY  1 {g:>3} DAT\nM  SAL {g:>3}  1 {:>3}\nM  SDT {g:>3} MRV_IMPLICIT_H\nM  SED {g:>3} {data}\n",
                    a + 1
                );
            }
        }
    }
    out
}

/// `mf` itself when it is not V3000, else its V2000 text.
pub(super) fn v2000(mf: &str) -> Result<Cow<'_, str>> {
    if !mf.lines().nth(3).is_some_and(|c| c.contains("V3000")) {
        return Ok(Cow::Borrowed(mf));
    }
    let record = mf.split("\n$$$$").next().unwrap_or("");
    let (mut counts, mut block) = (None, "");
    let (mut ids, mut atoms, mut bonds) = (
        Vec::<usize>::new(),
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    let mut groups: Vec<Vec<&str>> = Vec::new();
    let mut props = Props::new();
    let lines = logical_lines(record);
    for l in &lines {
        let t = tokens(l);
        match (t.first().copied(), t.get(1).copied()) {
            (Some("COUNTS"), _) => {
                let c = (
                    num::<usize>(t.get(1).unwrap_or(&""))?,
                    num::<usize>(t.get(2).unwrap_or(&""))?,
                );
                if c.0 > MAX_ATOMS {
                    return Err(Error::new("error.grafo.tooBig").with("max", MAX_ATOMS));
                }
                counts = Some(c);
            }
            (Some("BEGIN"), b) => block = b.unwrap_or(""),
            (Some("END"), _) => block = "",
            _ if block == "ATOM" => {
                let (id, line) = atom_line(&t, l, atoms.len(), &mut props)?;
                ids.push(id);
                atoms.push(line);
            }
            _ if block == "BOND" => bonds.push(bond_line(&t, l, &ids)?),
            _ if block == "SGROUP" => groups.push(t),
            _ => {}
        }
    }
    if atoms.is_empty() {
        return Err(bad("the V3000 file has no atoms"));
    }
    // a file cut short must not read as a smaller molecule
    if let Some(c) = counts.filter(|c| *c != (atoms.len(), bonds.len())) {
        return Err(bad(&format!(
            "the V3000 counts line says {} atoms and {} bonds, the blocks hold {} and {}",
            c.0,
            c.1,
            atoms.len(),
            bonds.len()
        )));
    }
    let mut out = format!(
        "{}\n  mzLab\n\n{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
        mf.lines().next().unwrap_or(""),
        atoms.len(),
        bonds.len()
    );
    out += &(atoms.concat() + &bonds.concat() + &prop_lines(&props) + &data_groups(&groups, &ids));
    Ok(Cow::Owned(out + "M  END\n"))
}
