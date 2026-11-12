//! `anal/1`: the numbers of one drawn structure beyond `etich/1` (whose formula, charge and masses it repeats): average molecular weight, mass % of
//! every element and the theoretical isotope pattern of the formula, grouped by nominal mass (centroided sticks M, M+1, ...). A drawn isotope (`[13C]`, D)
//! counts with its own mass and abundance 1; the other atoms with the natural abundances of `mzlab_core::isoscore` and the masses of `elements::nuclide`.

use mzlab_core::elements::{self, ELECTRON};
use mzlab_core::error::{Error, Result};
use mzlab_core::isoscore::isotopes_of;
use mzlab_core::json::{self, round, Out, Value};

use super::labels::{atom_in, shape, shape_json, Counts};

/// Sticks below this share of the tallest (%) are left out; probabilities below `PRUNE` are dropped while convolving.
const MIN_REL: f64 = 0.01;
const PRUNE: f64 = 1e-12;

/// Lightest mass number, then (probability, probability x mass) per mass number from there: dense vectors keep the .wasm small (no map).
type Dist = (u32, Vec<(f64, f64)>);
/// Per element: symbol, count and mass of its atoms.
type Per = Vec<(&'static str, u64, f64)>;

fn conv(a: &Dist, b: &Dist) -> Dist {
    let mut out = vec![(0.0, 0.0); a.1.len() + b.1.len() - 1];
    for (i, (pa, ma)) in a.1.iter().enumerate() {
        for (j, (pb, mb)) in b.1.iter().enumerate() {
            let p = pa * pb;
            if p >= PRUNE {
                out[i + j].0 += p;
                out[i + j].1 += ma * pb + mb * pa;
            }
        }
    }
    // the pruned tail goes, so the vector stays as long as the pattern (a pruned head stays: its sticks are below MIN_REL)
    out.truncate(out.iter().rposition(|x| x.0 > 0.0).map_or(1, |e| e + 1));
    (a.0 + b.0, out)
}

/// One key as a distribution and its average mass: a drawn isotope alone (abundance 1), an element with its natural abundances and the masses of
/// the nuclide table.
fn element(el: &str, a: u16) -> Result<(Dist, f64)> {
    if a > 0 {
        let m = elements::nuclide(el, Some(a)).map_or(a as f64, |x| x.1);
        return Ok(((a as u32, vec![(1.0, m)]), m));
    }
    let iso =
        isotopes_of(el).ok_or_else(|| Error::new("error.anal.abundance").with("label", el))?;
    let (total, lo) = iso.iter().fold((0.0, u32::MAX), |(t, l), i| {
        (t + i.1, l.min(i.0.round() as u32))
    });
    let (mut d, mut avg) = (Vec::new(), 0.0);
    for &(m, p) in iso {
        let k = (m.round() as u32 - lo) as usize;
        let (m, p) = (
            elements::nuclide(el, Some(k as u16 + lo as u16)).map_or(m, |x| x.1),
            p / total,
        );
        if d.len() <= k {
            d.resize(k + 1, (0.0, 0.0));
        }
        d[k] = (p, m * p);
        avg += m * p;
    }
    Ok(((lo, d), avg))
}

/// Average mass, then per element (isotopes and D/T under their element) count and mass in Hill order, then the pattern of the whole structure.
fn tally(n: &Counts) -> Result<(f64, Per, Dist)> {
    let rank = |e: &'static str| match e {
        "C" => (0, ""),
        "H" => (1, ""),
        _ => (2, e),
    };
    let (mut per, mut dist): (Per, Dist) = (Vec::new(), (0, vec![(1.0, 0.0)]));
    for ((e, a), c) in n {
        let (d, m) = element(e, *a)?;
        for _ in 0..*c {
            dist = conv(&dist, &d);
        }
        match per.iter_mut().find(|x| x.0 == *e) {
            Some(x) => {
                x.1 += c;
                x.2 += m * *c as f64;
            }
            // a handful of elements: an insertion in Hill order (a generic sort costs the .wasm more)
            None => {
                let i = per
                    .iter()
                    .position(|x| rank(x.0) > rank(e))
                    .unwrap_or(per.len());
                per.insert(i, (e, *c, m * *c as f64));
            }
        }
    }
    Ok((mzlab_core::pysum::pysum(per.iter().map(|x| x.2)), per, dist))
}

/// `anal/1` of a request `{"atoms": [{el, iso, h, q, r}, ...]}` (the atoms of one structure as `etich/1` reads them): the `etich/1` fields, then
/// `average`, `composition` `[{el, count, pct}]` (mass %), `ions` `[{ion, mz}]` (the drawn ion, or [M+H]+ and [M-H]- of a neutral: what the HR link
/// gives Composition) and `pattern` `[{off, mz, rel, abund}]` (offset from the lightest stick, m/z of the ion or
/// mass of a neutral, % of the tallest, % of the total). Theory for the drawn formula: nothing is compared with a spectrum.
pub fn analysis_json(request: &str) -> Result<String> {
    let v = json::parse(request.as_bytes()).map_err(|_| Error::new("error.anal.request"))?;
    let atoms: Vec<_> = v
        .get("atoms")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(atom_in)
        .collect();
    if atoms.is_empty() {
        return Err(Error::new("error.anal.empty"));
    }
    let s = shape(&atoms)?;
    let (avg, per, dist) = tally(&s.n)?;
    let mut o = Out::default();
    o.raw("{\"format\":\"anal/1\",\"structure\":");
    shape_json(&mut o, &s);
    o.raw(",\"average\":")
        .num(round(avg, 6))
        .raw(",\"composition\":[");
    for (i, (e, c, m)) in per.iter().enumerate() {
        o.raw(if i > 0 { ",{\"el\":" } else { "{\"el\":" }).str(e);
        o.raw(",\"count\":")
            .int(*c as i64)
            .raw(",\"pct\":")
            .num(round(100.0 * m / avg, 4))
            .raw("}");
    }
    // what Composition is asked for: the drawn ion itself, or [M+H]+ and [M-H]- of a neutral
    o.raw("],\"ions\":[");
    if s.q == 0 {
        let h = elements::nuclide("H", None).map_or(f64::NAN, |x| x.1) - ELECTRON;
        o.raw("{\"ion\":\"[M+H]+\",\"mz\":").num(s.mass + h);
        o.raw("},{\"ion\":\"[M-H]-\",\"mz\":")
            .num(s.mass - h)
            .raw("}");
    } else {
        let a = if s.q > 0 { "[M]+" } else { "[M]-" };
        let mz = (s.mass - s.q as f64 * ELECTRON) / s.q.abs() as f64;
        o.raw("{\"ion\":").str(a).raw(",\"mz\":").num(mz).raw("}");
    }
    o.raw("],\"pattern\":[");
    let total: f64 = dist.1.iter().map(|x| x.0).sum();
    let top = dist.1.iter().map(|x| x.0).fold(0.0, f64::max);
    let (z, mut k) = (s.q, 0);
    let first = dist
        .1
        .iter()
        .position(|x| 100.0 * x.0 / top >= MIN_REL)
        .unwrap_or(0);
    for (key, (p, pm)) in dist.1.iter().enumerate() {
        let rel = 100.0 * p / top;
        if rel < MIN_REL {
            continue;
        }
        let m = pm / p;
        let mz = if z == 0 {
            m
        } else {
            (m - z as f64 * ELECTRON) / z.abs() as f64
        };
        o.raw(if k > 0 { ",{\"off\":" } else { "{\"off\":" })
            .int(key as i64 - first as i64);
        o.raw(",\"mz\":")
            .num(round(mz, 6))
            .raw(",\"rel\":")
            .num(round(rel, 3))
            .raw(",\"abund\":")
            .num(round(100.0 * p / total, 4))
            .raw("}");
        k += 1;
    }
    o.raw("]}");
    Ok(o.0)
}
