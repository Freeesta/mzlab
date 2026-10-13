//! Isotopologues inside one file, before any alignment: 13C, 34S, 37Cl, 81Br, charge 1-3.
//!
//! A feature is the isotopologue of a lighter one when it lies one 13C (or one of the +2 steps: 13C2, 34S, 37Cl, 81Br) above it divided by
//! the charge, within a few ppm, with the same apex (RT within `rt_tol` minutes) and a ratio of `area_win` (raw trace around the apex, no floor) that natural abundance allows. The
//! allowed ratio is the one of an averagine of the same mass, computed with `isoscore::fine_pattern` (the same isotope table as the rest
//! of the program): M+1 over M between a quarter and two and a half times the averagine value; M+2 over M up to 1.2 (two bromines) and
//! not below a fifth of the averagine value. Chains (M+1 of an M+1) follow the same rule from the previous member. The group is
//! numbered after its lightest member (`iso_group` = position + 1, 0 = alone); isotopologues carry `FLAG_ISOTOPE`; members share the
//! charge. Everything is evidence: a feature that sits one mass unit above a stronger co-eluting ion with the right ratio is grouped,
//! never removed.

use mzlab_core::isoscore::fine_pattern;

use crate::axis::half;
use crate::table::{FileFeatures, FLAG_ISOTOPE};
use crate::trace::filled;
use mzlab_core::error::Result;

const ELECTRON_PROTON: f64 = 1.007_276;
const D_13C: f64 = 1.003_355;
/// +2 steps: 13C2, 34S, 37Cl, 81Br.
const D_PLUS2: [f64; 4] = [2.006_710, 1.995_796, 1.997_050, 1.997_954];
const AVERAGINE_DA: f64 = 111.1254;
const MASS_BIN: f64 = 25.0;

#[derive(Debug, Clone)]
pub struct IsoParams {
    pub max_charge: i32,
    pub rt_tol: f64,
    /// Tolerance of m/z, ppm or minus Da (`axis::half`); in Da (unit resolution) only the steps +1 and +2 are looked for.
    pub tol_ppm: f64,
}

impl Default for IsoParams {
    fn default() -> Self {
        IsoParams {
            max_charge: 3,
            rt_tol: 0.05,
            tol_ppm: 5.0,
        }
    }
}

/// Expected (M+1 / M, M+2 / M) of an averagine of the neutral mass, memoised per bin of 25 Da.
struct Expect {
    cache: Vec<Option<(f64, f64)>>,
}

impl Expect {
    fn new() -> Self {
        Expect { cache: Vec::new() }
    }

    fn ratios(&mut self, mass: f64) -> (f64, f64) {
        let bin = (mass / MASS_BIN).max(0.0) as usize;
        if self.cache.len() <= bin {
            self.cache.resize(bin + 1, None);
        }
        if let Some(r) = self.cache[bin] {
            return r;
        }
        let r = averagine_ratios((bin as f64 + 0.5) * MASS_BIN);
        self.cache[bin] = Some(r);
        r
    }
}

fn averagine_ratios(mass: f64) -> (f64, f64) {
    let u = mass / AVERAGINE_DA;
    let n = |x: f64| ((x * u).round() as i64).max(0);
    let counts = vec![
        ("C".to_string(), n(4.9384).max(1)),
        ("H".to_string(), n(7.7583)),
        ("N".to_string(), n(1.3577)),
        ("O".to_string(), n(1.4773)),
        ("S".to_string(), n(0.0417)),
    ];
    let Ok(pat) = fine_pattern(&counts, 1, 0.001) else {
        return (0.0107 * mass * 0.0444, 0.01);
    };
    let Some(first) = pat.first() else {
        return (0.0, 0.0);
    };
    let mut nominal = [0.0f64; 3];
    for p in &pat {
        let k = (p.mz - first.mz).round() as usize;
        if k < 3 {
            nominal[k] += p.rel;
        }
    }
    if nominal[0] <= 0.0 {
        return (0.0, 0.0);
    }
    (nominal[1] / nominal[0], nominal[2] / nominal[0])
}

/// Whether the area ratio `r` (isotopologue over its parent) is one natural abundance allows for a step of `k` mass units.
fn plausible(r: f64, k: u8, expect: (f64, f64)) -> bool {
    if k == 1 {
        r >= 0.25 * expect.0 - 0.005 && r <= 2.5 * expect.0 + 0.05
    } else {
        r >= 0.2 * expect.1 && r <= 1.2
    }
}

/// The steps `(k, delta)` looked for below a feature. At unit resolution a +1 step is only registered (it may be another ion) and the +2
/// steps are one (37Cl, 81Br: the ratio decides).
fn steps(unit: bool) -> Vec<(u8, f64)> {
    let mut v = vec![(1u8, D_13C)];
    if unit {
        v.push((2, D_PLUS2[2]));
        return v;
    }
    v.extend(D_PLUS2.iter().map(|&d| (2u8, d)));
    v
}

/// Best lighter parent of feature `j`: `(parent, charge)` with the smallest error in ppm (ties: lower charge, lower index).
fn parent_of(
    f: &FileFeatures,
    j: usize,
    charge: &[i8],
    ex: &mut Expect,
    p: &IsoParams,
    st: &[(u8, f64)],
) -> Option<(usize, i32)> {
    let mut best: Option<(f64, i32, usize)> = None;
    for z in 1..=p.max_charge {
        for &(k, d) in st {
            let want = f.mz[j] - d / z as f64;
            let tol = half(f.mz[j], p.tol_ppm) + 3e-4;
            let a = f.mz.partition_point(|&m| m < want - tol);
            let b = f.mz.partition_point(|&m| m <= want + tol);
            #[allow(clippy::needless_range_loop)]
            for i in a..b.min(j) {
                if (f.rt[i] - f.rt[j]).abs() as f64 > p.rt_tol
                    || (charge[i] != 0 && charge[i] as i32 != z)
                {
                    continue;
                }
                let mass = (f.mz[i] - ELECTRON_PROTON) * z as f64;
                let r = f.area_win[j] / f.area_win[i].max(1e-300);
                if !plausible(r, k, ex.ratios(mass)) {
                    continue;
                }
                let err = ((f.mz[i] + d / z as f64 - f.mz[j]) / f.mz[j]).abs() * 1e6;
                if best.is_none_or(|(e, bz, _)| err < e || (err == e && z < bz)) {
                    best = Some((err, z, i));
                }
            }
        }
    }
    best.map(|(_, z, i)| (i, z))
}

/// Group the isotopologues of the table: fills `iso_group`, `charge` and `FLAG_ISOTOPE` (features sorted by m/z).
pub fn group(f: &mut FileFeatures, p: &IsoParams) -> Result<()> {
    let n = f.len();
    let mut ex = Expect::new();
    let st = steps(p.tol_ppm < 0.0);
    let mut charge = filled(n, 0i8)?;
    let mut gid = filled(n, 0u32)?;
    for j in 0..n {
        let Some((i, z)) = parent_of(f, j, &charge, &mut ex, p, &st) else {
            continue;
        };
        let g = if gid[i] != 0 { gid[i] } else { i as u32 + 1 };
        gid[i] = g;
        gid[j] = g;
        charge[i] = z as i8;
        charge[j] = z as i8;
        f.flags[j] |= FLAG_ISOTOPE;
    }
    f.iso_group = gid;
    f.charge = charge;
    Ok(())
}
