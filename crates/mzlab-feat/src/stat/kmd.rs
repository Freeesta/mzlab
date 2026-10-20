//! Kendrick mass defect of a list of ions, ported from `mzlab/web/kmd-calc.js` (parity: `tests/golden/kmd.json`, `tests/test_kmd.py`).
//! KM = m/z x (nominal mass of the repeating unit / its exact mass); KMD = round(KM) - KM. Ions of a homologous series share the KMD: the series
//! are CANDIDATES with their evidence (members, steps, retention trend); nothing here says what an ion is.

use crate::sortc;

/// A repeating unit: exact monoisotopic mass (C = 12 by definition) and nominal mass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit {
    pub id: &'static str,
    pub mass: f64,
    pub nominal: f64,
}

const H: f64 = 1.007_825_032_23;
const F: f64 = 18.998_403_162_73;
const O: f64 = 15.994_914_619_57;

fn unit(id: &'static str, mass: f64) -> Unit {
    Unit {
        id,
        mass,
        nominal: js_round(mass),
    }
}

/// The units of the panel, CF2 first (the default).
pub fn units() -> [Unit; 4] {
    [
        unit("CF2", 12.0 + 2.0 * F),
        unit("CF2O", 12.0 + 2.0 * F + O),
        unit("C2F4", 24.0 + 4.0 * F),
        unit("CH2", 12.0 + 2.0 * H),
    ]
}

/// The unit with this id; CF2 when it is unknown.
pub fn unit_by_id(id: &str) -> Unit {
    let all = units();
    all.iter().copied().find(|u| u.id == id).unwrap_or(all[0])
}

/// `Math.round` of JavaScript: halves go up.
fn js_round(x: f64) -> f64 {
    libm::floor(x + 0.5)
}

/// Kendrick mass, nominal Kendrick mass and KMD of one m/z.
pub fn kendrick(mz: f64, u: &Unit) -> (f64, f64, f64) {
    let km = mz * u.nominal / u.mass;
    let nkm = js_round(km);
    (km, nkm, nkm - km)
}

fn ranks(v: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..v.len()).collect();
    sortc::by(&mut idx, |&a, &b| v[a].total_cmp(&v[b]));
    let mut r = vec![0.0; v.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && v[idx[j + 1]] == v[idx[i]] {
            j += 1;
        }
        for &k in &idx[i..=j] {
            r[k] = (i + j) as f64 / 2.0 + 1.0;
        }
        i = j + 1;
    }
    r
}

/// Spearman rank correlation (average ranks for ties); `None` under 3 points or without variation.
pub fn spearman(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() < 3 {
        return None;
    }
    let (a, b) = (ranks(x), ranks(y));
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for i in 0..a.len() {
        sab += (a[i] - ma) * (b[i] - mb);
        saa += (a[i] - ma) * (a[i] - ma);
        sbb += (b[i] - mb) * (b[i] - mb);
    }
    (saa > 0.0 && sbb > 0.0).then(|| sab / libm::sqrt(saa * sbb))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ion {
    pub mz: f64,
    pub rt: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KmdRow {
    /// Index in the input list.
    pub i: usize,
    pub mz: f64,
    pub rt: Option<f64>,
    pub km: f64,
    pub nkm: f64,
    pub kmd: f64,
    /// 1-based number of the series, when the ion belongs to one.
    pub series: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub n: usize,
    /// Input indexes of the members, by nominal Kendrick mass.
    pub members: Vec<usize>,
    pub kmd: f64,
    pub span_mda: f64,
    pub steps: Vec<f64>,
    pub consecutive: f64,
    pub distinct: usize,
    pub rho: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KmdOut {
    pub unit: Unit,
    pub rows: Vec<KmdRow>,
    pub series: Vec<Series>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KmdOpts {
    pub unit: &'static str,
    /// Width of the KMD window of a series, in mDa.
    pub tol_mda: f64,
    /// Distinct nominal Kendrick masses a series needs.
    pub min_members: usize,
}

impl Default for KmdOpts {
    fn default() -> Self {
        KmdOpts {
            unit: "CF2",
            tol_mda: 3.0,
            min_members: 3,
        }
    }
}

/// The series of one window of KMD (`grp` sorted by KMD), or `None` when it has too few distinct nominal Kendrick masses.
fn series_of(grp: &[usize], rows: &[KmdRow], u: &Unit, n: usize, min: usize) -> Option<Series> {
    let mut nk: Vec<f64> = grp.iter().map(|&k| rows[k].nkm).collect();
    sortc::by(&mut nk, f64::total_cmp);
    nk.dedup();
    if nk.len() < min {
        return None;
    }
    let mut mem = grp.to_vec();
    sortc::by(&mut mem, |&a, &b| {
        rows[a]
            .nkm
            .total_cmp(&rows[b].nkm)
            .then(rows[a].kmd.total_cmp(&rows[b].kmd))
    });
    let steps: Vec<f64> = mem
        .windows(2)
        .filter(|w| rows[w[1]].nkm != rows[w[0]].nkm)
        .map(|w| js_round((rows[w[1]].nkm - rows[w[0]].nkm) / u.nominal))
        .collect();
    let with_rt: Vec<usize> = mem
        .iter()
        .copied()
        .filter(|&k| rows[k].rt.is_some())
        .collect();
    let rho = spearman(
        &with_rt.iter().map(|&k| rows[k].nkm).collect::<Vec<_>>(),
        &with_rt
            .iter()
            .map(|&k| rows[k].rt.unwrap_or(0.0))
            .collect::<Vec<_>>(),
    );
    let kmds: Vec<f64> = grp.iter().map(|&k| rows[k].kmd).collect();
    let (lo, hi) = (
        kmds.iter().copied().fold(f64::INFINITY, f64::min),
        kmds.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
    Some(Series {
        n,
        members: mem.iter().map(|&k| rows[k].i).collect(),
        kmd: kmds.iter().sum::<f64>() / kmds.len() as f64,
        span_mda: (hi - lo) * 1000.0,
        consecutive: if steps.is_empty() {
            0.0
        } else {
            steps.iter().filter(|&&s| s == 1.0).count() as f64 / steps.len() as f64
        },
        steps,
        distinct: nk.len(),
        rho,
    })
}

/// KMD windows over the ions sorted by KMD: a window starts at the lowest free KMD and takes every ion within 2 x tolerance of it.
pub fn analyse(ions: &[Ion], o: &KmdOpts) -> KmdOut {
    let u = unit_by_id(o.unit);
    let tol = o.tol_mda.max(0.0) / 1000.0;
    let mut rows: Vec<KmdRow> = ions
        .iter()
        .enumerate()
        .filter(|(_, g)| g.mz.is_finite() && g.mz > 0.0)
        .map(|(i, g)| {
            let (km, nkm, kmd) = kendrick(g.mz, &u);
            KmdRow {
                i,
                mz: g.mz,
                rt: g.rt,
                km,
                nkm,
                kmd,
                series: None,
            }
        })
        .collect();
    let mut order: Vec<usize> = (0..rows.len()).collect();
    sortc::by(&mut order, |&a, &b| rows[a].kmd.total_cmp(&rows[b].kmd));
    let mut series: Vec<Series> = Vec::new();
    let mut s = 0;
    while s < order.len() {
        let mut e = s;
        while e + 1 < order.len() && rows[order[e + 1]].kmd - rows[order[s]].kmd <= 2.0 * tol {
            e += 1;
        }
        match series_of(&order[s..=e], &rows, &u, series.len() + 1, o.min_members) {
            Some(ser) => {
                let by_i: Vec<usize> = (0..rows.len())
                    .filter(|&k| ser.members.contains(&rows[k].i))
                    .collect();
                by_i.into_iter().for_each(|k| rows[k].series = Some(ser.n));
                series.push(ser);
                s = e + 1;
            }
            None => s += 1,
        }
    }
    KmdOut {
        unit: u,
        rows,
        series,
    }
}
