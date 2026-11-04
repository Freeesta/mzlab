//! Isotope edges and ion identity edges over the correlated pairs, and the groups they make. An isotope edge is a step of 13C (or a +2 step of 13C2,
//! 34S, 37Cl, 81Br) over a charge 1..max with a height ratio natural abundance allows; an ion identity edge is two species of the library giving
//! one neutral mass. Each row keeps the species most of its partners agree on (ties: library order), as the IIN of mzmine resolves conflicts.

use super::ions::Ion;
use super::pairs::Corr;
use super::{Edge, EdgeKind, Params};
use crate::table::Aligned;

const C13: f64 = 1.003_354_8;
/// (step in Da, isotopes the step stands for, steps of 13C it counts as)
const ISO_STEPS: [(f64, &str, u8); 5] = [
    (C13, "13C", 1),
    (2.0 * C13, "13C2", 2),
    (1.995_796, "34S", 2),
    (1.997_050, "37Cl", 2),
    (1.997_953, "81Br", 2),
];

pub fn tol(p: &Params, mz: f64) -> f64 {
    (p.ppm * 1e-6 * mz).max(p.mz_abs)
}

/// Median of the height ratio b/a over the files that measure both (NaN when none).
fn ratio(t: &Aligned, a: usize, b: usize) -> f64 {
    let nf = t.n_files;
    let mut v: Vec<f64> = (0..nf)
        .filter(|&f| t.height[a * nf + f] > 0.0 && t.height[b * nf + f] > 0.0)
        .map(|f| f64::from(t.height[b * nf + f]) / f64::from(t.height[a * nf + f]))
        .collect();
    crate::sortc::select_median(&mut v).unwrap_or(f64::NAN)
}

/// The isotope step from light row `a` to heavy row `b`: (step index, charge, error in Da).
fn iso_step(t: &Aligned, p: &Params, a: usize, b: usize) -> Option<(usize, i8, f64)> {
    let d = t.mz[b] - t.mz[a];
    let mut best: Option<(usize, i8, f64)> = None;
    for z in 1..=p.max_charge {
        for (k, &(step, _, c13)) in ISO_STEPS.iter().enumerate() {
            let err = (d - step / f64::from(z)).abs();
            if err > tol(p, t.mz[b]) || best.is_some_and(|x| x.2 <= err) {
                continue;
            }
            // averagine: M+1/M is about 0.000475 per Da of neutral mass; allow 2.5 times that, +2 steps up to 1.2 (two bromines)
            let mass = t.mz[a] * f64::from(z);
            let max = if c13 == 1 {
                (2.5 * 0.000_475 * mass).max(0.1)
            } else {
                1.2
            };
            if ratio(t, a, b) <= max {
                best = Some((k, z, err));
            }
        }
    }
    best
}

/// Isotope edges: each heavy row gets the one light partner with the smallest error; `iso[b]` = 13C steps above the lightest member, `charge` the charge of the step.
pub fn isotopes(
    t: &Aligned,
    p: &Params,
    corr: &[Corr],
    iso: &mut [u8],
    charge: &mut [i8],
) -> Vec<Edge> {
    let mut light: Vec<Option<(usize, usize, i8, f64, f64)>> = vec![None; t.len()];
    for c in corr {
        let (a, b) = if t.mz[c.a] <= t.mz[c.b] {
            (c.a, c.b)
        } else {
            (c.b, c.a)
        };
        if let Some((k, z, err)) = iso_step(t, p, a, b) {
            if light[b].is_none_or(|x| err < x.3) {
                light[b] = Some((a, k, z, err, c.r));
            }
        }
    }
    // rows by m/z: a light partner has its own level before its heavy row
    let order = crate::sortc::order(t.len(), &mut |i, j| {
        crate::sortc::asc_nan_last(t.mz[i], t.mz[j]).then(i.cmp(&j))
    });
    let mut edges = Vec::new();
    for &b in &order {
        let b = b as usize;
        let Some((a, k, z, _, r)) = light[b] else {
            continue;
        };
        iso[b] = iso[a].saturating_add(ISO_STEPS[k].2);
        charge[b] = z;
        charge[a] = z;
        let note = format!("{} z={z} dm={:.4}", ISO_STEPS[k].1, t.mz[b] - t.mz[a]);
        edges.push(Edge {
            a,
            b,
            kind: EdgeKind::Isotope,
            score: r,
            note,
        });
    }
    edges
}

/// One candidate ion identity edge: the species of `a` and of `b` (indices into the list) that give one neutral mass.
struct Cand {
    a: usize,
    b: usize,
    ia: usize,
    ib: usize,
    r: f64,
}

fn fits(ion: &Ion, charge: i8) -> bool {
    charge <= 1 || ion.z.unsigned_abs() as i8 == charge
}

fn candidates(t: &Aligned, p: &Params, corr: &[Corr], iso: &[u8], charge: &[i8]) -> Vec<Cand> {
    let mut out = Vec::new();
    for c in corr.iter().filter(|c| iso[c.a] == 0 && iso[c.b] == 0) {
        for (ia, x) in p
            .ions
            .iter()
            .enumerate()
            .filter(|(_, x)| fits(x, charge[c.a]))
        {
            for (ib, y) in p
                .ions
                .iter()
                .enumerate()
                .filter(|(_, y)| fits(y, charge[c.b]))
            {
                let (ma, mb) = (x.neutral(t.mz[c.a]), y.neutral(t.mz[c.b]));
                if ia != ib && ma > 0.0 && (ma - mb).abs() <= tol(p, ma.max(mb)) {
                    out.push(Cand {
                        a: c.a,
                        b: c.b,
                        ia,
                        ib,
                        r: c.r,
                    });
                }
            }
        }
    }
    out
}

/// Ion identity edges: the species of each row is the one with most partners (ties: library order); an edge stays when both ends keep its species.
pub fn identities(
    t: &Aligned,
    p: &Params,
    corr: &[Corr],
    iso: &[u8],
    charge: &[i8],
    ion: &mut [Option<usize>],
    neutral: &mut [f64],
) -> Vec<Edge> {
    let cands = candidates(t, p, corr, iso, charge);
    let ni = p.ions.len();
    let mut support = vec![0u32; t.len() * ni];
    for c in &cands {
        support[c.a * ni + c.ia] += 1;
        support[c.b * ni + c.ib] += 1;
    }
    let pick: Vec<Option<usize>> = (0..t.len())
        .map(|r| {
            let s = &support[r * ni..(r + 1) * ni];
            (0..ni).fold(None, |acc: Option<usize>, k| match acc {
                Some(b) if s[b] >= s[k] => Some(b),
                _ if s[k] > 0 => Some(k),
                _ => acc,
            })
        })
        .collect();
    let mut edges = Vec::new();
    for c in cands
        .iter()
        .filter(|c| pick[c.a] == Some(c.ia) && pick[c.b] == Some(c.ib))
    {
        let (x, y) = (p.ions[c.ia], p.ions[c.ib]);
        ion[c.a] = Some(c.ia);
        ion[c.b] = Some(c.ib);
        neutral[c.a] = x.neutral(t.mz[c.a]);
        neutral[c.b] = y.neutral(t.mz[c.b]);
        let note = format!("{} {} dm={:.4}", x.name, y.name, t.mz[c.b] - t.mz[c.a]);
        edges.push(Edge {
            a: c.a,
            b: c.b,
            kind: EdgeKind::Ion,
            score: c.r,
            note,
        });
    }
    edges
}

/// Union-find roots; the group ids are numbered by the first row of each group (1-based).
pub fn components(n: usize, edges: impl Iterator<Item = (usize, usize)>) -> Vec<u32> {
    let mut up: Vec<usize> = (0..n).collect();
    fn root(up: &mut [usize], mut i: usize) -> usize {
        while up[i] != i {
            up[i] = up[up[i]];
            i = up[i];
        }
        i
    }
    for (a, b) in edges {
        let (ra, rb) = (root(&mut up, a), root(&mut up, b));
        if ra != rb {
            up[ra.max(rb)] = ra.min(rb);
        }
    }
    let mut id = vec![0u32; n];
    let mut next = 0u32;
    for i in 0..n {
        let r = root(&mut up, i);
        if r == i {
            next += 1;
            id[i] = next;
        } else {
            id[i] = id[r];
        }
    }
    id
}
