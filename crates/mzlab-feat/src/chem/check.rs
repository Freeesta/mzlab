//! `check/1`: what is wrong in a drawing, never in data. The page sends the molfile of the canvas; the answer lists warnings (valence, charge,
//! isotope, wedges, aromatic rings that cannot alternate, query atoms, separate pieces), each with a code that names its `draw.check.*` texts
//! and the atoms (0-based, molfile order) to highlight. Valence rules come from `mzlab-mol` (the ones of its SMILES reader), isotopes from the
//! nuclide table of `mzlab-core`. Same bytes in the grafo `.wasm`, the app (`feat/grafo`, `kind: "check"`) and `mzlab-cli mol-check`.

use mzlab_core::elements;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out};
use mzlab_mol::{allowed_valences, has_valence_rules, Drawing, Mol, Site, Stereo};

/// Above this many atoms the check refuses (small molecules only; the stereo sites try every centre).
const MAX_ATOMS: usize = 400;

/// One warning: code, level (`warn` a mistake, `note` worth knowing), atoms and bonds to highlight, a few numbers for the texts.
struct Warning {
    code: &'static str,
    level: &'static str,
    atoms: Vec<u32>,
    bonds: Vec<(u32, u32)>,
    el: String,
    nums: Vec<(&'static str, i64)>,
    allowed: Vec<u32>,
}

impl Warning {
    fn new(code: &'static str, level: &'static str, atoms: Vec<u32>) -> Self {
        Warning {
            code,
            level,
            atoms,
            bonds: Vec::new(),
            el: String::new(),
            nums: Vec::new(),
            allowed: Vec::new(),
        }
    }
}

/// Sum of bond orders of every atom.
fn bond_sums(m: &Mol) -> Vec<u32> {
    let mut s = vec![0u32; m.atoms.len()];
    for b in &m.bonds {
        s[b.a as usize] += u32::from(b.o);
        s[b.b as usize] += u32::from(b.o);
    }
    s
}

/// Connected pieces as sorted atom lists, ordered by their first atom.
fn pieces(m: &Mol) -> Vec<Vec<u32>> {
    let n = m.atoms.len();
    let mut root: Vec<usize> = (0..n).collect();
    fn find(r: &mut [usize], mut x: usize) -> usize {
        while r[x] != x {
            r[x] = r[r[x]];
            x = r[x];
        }
        x
    }
    for b in &m.bonds {
        let (x, y) = (find(&mut root, b.a as usize), find(&mut root, b.b as usize));
        root[x.max(y)] = x.min(y);
    }
    let mut out: Vec<Vec<u32>> = Vec::new();
    let mut slot = vec![usize::MAX; n];
    for i in 0..n {
        let r = find(&mut root, i);
        if slot[r] == usize::MAX {
            slot[r] = out.len();
            out.push(Vec::new());
        }
        out[slot[r]].push(i as u32);
    }
    out
}

/// Valence and charge of each atom with rules: a total outside the allowed ones is `valence`, or `valenceCharge` when another charge
/// (0, +1 or -1) would make it right; a charge of 2 or more on such an atom, or one the element cannot take, is `charge`.
fn atoms(d: &Drawing, out: &mut Vec<Warning>) {
    let sums = bond_sums(&d.mol);
    for (i, a) in d.mol.atoms.iter().enumerate() {
        if d.query_atoms.contains(&(i as u32)) || !has_valence_rules(&a.el) {
            continue;
        }
        let chg = i32::from(a.chg);
        let total = sums[i] + u32::from(a.h) + u32::from(a.rad);
        let allowed = allowed_valences(&a.el, chg);
        if allowed.is_empty() || chg.abs() >= 2 {
            let mut w = Warning::new("charge", "warn", vec![i as u32]);
            w.el = a.el.clone();
            w.nums.push(("q", i64::from(chg)));
            out.push(w);
            continue;
        }
        if allowed.contains(&total) {
            continue;
        }
        let fix = [0, 1, -1]
            .into_iter()
            .filter(|&q| q != chg)
            .find(|&q| allowed_valences(&a.el, q).contains(&total));
        let mut w = Warning::new(
            if fix.is_some() {
                "valenceCharge"
            } else {
                "valence"
            },
            "warn",
            vec![i as u32],
        );
        w.el = a.el.clone();
        w.nums.push(("valence", i64::from(total)));
        w.nums.push(("q", i64::from(chg)));
        if let Some(q) = fix {
            w.nums.push(("fix", i64::from(q)));
        }
        w.allowed = allowed;
        out.push(w);
    }
}

/// A mass number that no known nuclide of the element has.
fn isotopes(d: &Drawing, out: &mut Vec<Warning>) {
    for (i, a) in d.mol.atoms.iter().enumerate() {
        if a.iso == 0 || d.query_atoms.contains(&(i as u32)) {
            continue;
        }
        if elements::nuclide(&a.el, Some(a.iso)).is_none() {
            let mut w = Warning::new("isotope", "warn", vec![i as u32]);
            w.el = a.el.clone();
            w.nums.push(("iso", i64::from(a.iso)));
            out.push(w);
        }
    }
}

/// Wedges: on a multiple bond (`wedgeMultiple`), from an atom that is no stereo centre (`wedgeNoCentre`), or at a centre whose wedges
/// leave its arrangement open (`wedgeUndecided`). Wavy marks say "unknown" on purpose and are only checked for the centre.
fn wedges(d: &Drawing, out: &mut Vec<Warning>) {
    let centre = |c: u32| d.sites.contains(&Site::Tetra(c));
    let defined = |c: u32| {
        d.mol
            .stereo
            .iter()
            .any(|s| matches!(s, Stereo::Tetra { c: x, .. } if *x == c))
    };
    let mut undecided: Vec<u32> = Vec::new();
    for w in &d.wedges {
        let bond = (w.from.min(w.to), w.from.max(w.to));
        let code = if w.multiple {
            "wedgeMultiple"
        } else if !centre(w.from) {
            "wedgeNoCentre"
        } else if !w.wavy && !defined(w.from) {
            if undecided.contains(&w.from) {
                continue;
            }
            undecided.push(w.from);
            "wedgeUndecided"
        } else {
            continue;
        };
        let mut x = Warning::new(code, "warn", vec![w.from]);
        x.bonds.push(bond);
        out.push(x);
    }
}

/// `check/1` of a request `{"molfile": "..."}`.
pub fn check_json(request: &str) -> Result<String> {
    let v = json::parse(request.as_bytes()).map_err(|_| Error::new("error.check.request"))?;
    let mf = v
        .get("molfile")
        .and_then(json::Value::str)
        .ok_or_else(|| Error::new("error.check.request"))?;
    let d = Mol::drawing_from_molfile(mf).map_err(|_| Error::new("error.check.molfile"))?;
    if d.mol.atoms.len() > MAX_ATOMS {
        return Err(Error::new("error.check.tooBig").with("max", MAX_ATOMS));
    }
    let mut out = Vec::new();
    if !d.query_atoms.is_empty() {
        out.push(Warning::new("query", "note", d.query_atoms.clone()));
    }
    if !d.query_bonds.is_empty() {
        let mut w = Warning::new("queryBond", "note", Vec::new());
        w.bonds = d.query_bonds.iter().map(|&(a, b, _)| (a, b)).collect();
        out.push(w);
    }
    if !d.kekule {
        out.push(Warning::new("kekule", "warn", d.arom_atoms.clone()));
    }
    atoms(&d, &mut out);
    isotopes(&d, &mut out);
    wedges(&d, &mut out);
    let parts = pieces(&d.mol);
    if parts.len() > 1 {
        let mut w = Warning::new("pieces", "note", Vec::new());
        w.nums.push(("n", parts.len() as i64));
        out.push(w);
    }
    Ok(write(&d, &parts, &out))
}

fn write(d: &Drawing, parts: &[Vec<u32>], ws: &[Warning]) -> String {
    let mut o = Out::default();
    o.raw("{\"format\":\"check/1\",\"atoms\":")
        .int(d.mol.atoms.len() as i64)
        .raw(",\"pieces\":")
        .int(parts.len() as i64)
        .raw(",\"warnings\":[");
    for (k, w) in ws.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" });
        o.raw("{\"code\":")
            .str(w.code)
            .raw(",\"level\":")
            .str(w.level)
            .raw(",\"atoms\":[");
        for (j, a) in w.atoms.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).int(i64::from(*a));
        }
        o.raw("],\"bonds\":[");
        for (j, (a, b)) in w.bonds.iter().enumerate() {
            o.raw(if j > 0 { ",[" } else { "[" })
                .int(i64::from(*a))
                .raw(",")
                .int(i64::from(*b))
                .raw("]");
        }
        o.raw("]");
        if !w.el.is_empty() {
            o.raw(",\"el\":").str(&w.el);
        }
        for (key, n) in &w.nums {
            o.raw(",").str(key).raw(":").int(*n);
        }
        if !w.allowed.is_empty() {
            o.raw(",\"allowed\":[");
            for (j, a) in w.allowed.iter().enumerate() {
                o.raw(if j > 0 { "," } else { "" }).int(i64::from(*a));
            }
            o.raw("]");
        }
        o.raw("}");
    }
    o.raw("]}");
    o.0
}
