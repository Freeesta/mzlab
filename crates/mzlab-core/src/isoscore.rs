//! Isotope-pattern score and fragment constraint of a candidate formula (`mzlab/chem/isoscore.py`): evidence with its parts, never a
//! verdict. Same fusion at the resolving power, same matching, same weights; the scores are the ones Python computes.

use crate::composition::Counts;
use crate::elements::{mass_of, parse_formula, ELECTRON};
use crate::error::{Error, Result};
use crate::pysum::pysum;
use crate::subformulas::submasses;

const ISO: [(&str, &[(f64, f64)]); 13] = [
    ("H", &[(1.00782503223, 0.999885), (2.01410177812, 0.000115)]),
    ("C", &[(12.0, 0.9893), (13.00335483507, 0.0107)]),
    ("N", &[(14.00307400443, 0.99636), (15.00010889888, 0.00364)]),
    (
        "O",
        &[
            (15.99491461957, 0.99757),
            (16.99913175650, 0.00038),
            (17.99915961286, 0.00205),
        ],
    ),
    ("F", &[(18.99840316273, 1.0)]),
    ("Na", &[(22.989769282, 1.0)]),
    (
        "Si",
        &[
            (27.9769265350, 0.92223),
            (28.9764946653, 0.04685),
            (29.973770136, 0.03092),
        ],
    ),
    ("P", &[(30.97376199842, 1.0)]),
    (
        "S",
        &[
            (31.9720711744, 0.9499),
            (32.9714589098, 0.0075),
            (33.967867004, 0.0425),
            (35.96708071, 0.0001),
        ],
    ),
    ("Cl", &[(34.968852682, 0.7576), (36.965902602, 0.2424)]),
    ("K", &[(38.9637064864, 0.932581), (40.9618252579, 0.067302)]),
    ("Br", &[(78.9183376, 0.5069), (80.9162897, 0.4931)]),
    ("I", &[(126.9044719, 1.0)]),
];
const MERGE_TOL: f64 = 0.0005;
const MAX_SHIFT: f64 = 6.0;

#[derive(Debug, Clone)]
pub struct Peak {
    pub mz: f64,
    pub rel: f64,
}

#[derive(Debug, Clone)]
pub struct PeakScore {
    pub mz: f64,
    pub rel: f64,
    pub obs_mz: Option<f64>,
    pub obs_rel: Option<f64>,
    pub d_ppm: Option<f64>,
    pub t_mass: f64,
    pub t_int: f64,
    pub t: f64,
    pub w: f64,
}

#[derive(Debug, Clone)]
pub struct Extra {
    pub mz: f64,
    pub rel: f64,
    pub w: f64,
}

#[derive(Debug, Clone)]
pub struct IsoScore {
    pub score: f64,
    pub peaks: Vec<PeakScore>,
    pub extra: Vec<Extra>,
    pub n_missing: usize,
}

fn sort_pairs(v: &mut [(f64, f64)]) {
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
}

fn join(mut items: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    sort_pairs(&mut items);
    let mut out: Vec<[f64; 3]> = vec![];
    for (m, p) in items {
        match out.last_mut() {
            Some(l) if m - l[2] <= MERGE_TOL => {
                l[0] += m * p;
                l[1] += p;
                l[2] = m;
            }
            _ => out.push([m * p, p, m]),
        }
    }
    out.into_iter().map(|[s, p, _]| (s / p, p)).collect()
}

fn conv(a: &[(f64, f64)], b: &[(f64, f64)], floor: f64) -> Vec<(f64, f64)> {
    let mut r = vec![];
    for &(ma, pa) in a {
        for &(mb, pb) in b {
            if pa * pb > floor {
                r.push((ma + mb, pa * pb));
            }
        }
    }
    join(r)
        .into_iter()
        .filter(|(m, _)| *m <= MAX_SHIFT + 0.5)
        .collect()
}

/// Fine isotope structure of an ion formula: (m/z, % of the most intense isotopologue), sorted by m/z.
pub fn fine_pattern(counts: &Counts, z: i32, min_rel: f64) -> Result<Vec<Peak>> {
    let mut dist = vec![(0.0, 1.0)];
    let floor = 1e-12;
    for (el, k) in counts {
        if *k <= 0 {
            continue;
        }
        let Some(iso) = ISO.iter().find(|(e, _)| e == el).map(|(_, p)| *p) else {
            return Err(Error::new("err.iso.element").with("symbol", el));
        };
        let mut base: Vec<(f64, f64)> = iso.iter().map(|(m, p)| (m - iso[0].0, *p)).collect();
        let (mut res, mut c) = (vec![(0.0, 1.0)], *k);
        while c > 0 {
            if c & 1 == 1 {
                res = conv(&res, &base, floor);
            }
            c >>= 1;
            if c > 0 {
                base = conv(&base, &base, floor);
            }
        }
        dist = conv(&dist, &res, floor);
    }
    let mono = pysum(
        counts
            .iter()
            .filter(|(_, k)| *k > 0)
            .map(|(el, k)| ISO.iter().find(|(e, _)| e == el).unwrap().1[0].0 * *k as f64),
    );
    let top = dist.iter().map(|d| d.1).fold(f64::MIN, f64::max);
    Ok(dist
        .iter()
        .filter(|(_, p)| 100.0 * p / top >= min_rel)
        .map(|(m, p)| Peak {
            mz: (mono + m - z as f64 * ELECTRON) / (z.abs() as f64),
            rel: 100.0 * p / top,
        })
        .collect())
}

/// Fuse the peaks that the instrument does not resolve; `r200 <= 0` keeps the fine structure.
pub fn merge_res(pat: Vec<Peak>, r200: f64) -> Vec<Peak> {
    if !(r200 > 0.0) || pat.is_empty() {
        return pat;
    }
    let mut out: Vec<[f64; 3]> = vec![];
    for r in &pat {
        let fw = r.mz / (r200 * (200.0 / r.mz).sqrt());
        match out.last_mut() {
            Some(l) if r.mz - l[2] <= fw => {
                l[0] += r.mz * r.rel;
                l[1] += r.rel;
                l[2] = r.mz;
            }
            _ => out.push([r.mz * r.rel, r.rel, r.mz]),
        }
    }
    let top = out.iter().map(|c| c[1]).fold(f64::MIN, f64::max);
    out.iter()
        .map(|c| Peak {
            mz: c[0] / c[1],
            rel: 100.0 * c[1] / top,
        })
        .collect()
}

pub struct IsoOpts<'a> {
    pub r200: f64,
    pub sigma_ppm: f64,
    pub sigma_int: f64,
    pub min_rel: f64,
    pub noise_rel: f64,
    pub norm: &'a str,
}

/// Score of the isotope pattern of `counts` against the observed peaks (m/z, intensity).
pub fn isotope_score(counts: &Counts, z: i32, obs: &[(f64, f64)], o: &IsoOpts) -> Result<IsoScore> {
    let th: Vec<Peak> = merge_res(fine_pattern(counts, z, 0.001)?, o.r200)
        .into_iter()
        .filter(|p| p.rel >= o.min_rel)
        .collect();
    let blank = |p: &Peak| PeakScore {
        mz: p.mz,
        rel: p.rel,
        obs_mz: None,
        obs_rel: None,
        d_ppm: None,
        t_mass: 0.0,
        t_int: 0.0,
        t: 0.0,
        w: 0.0,
    };
    if th.is_empty() || obs.is_empty() {
        return Ok(IsoScore {
            score: 0.0,
            peaks: th.iter().map(blank).collect(),
            extra: vec![],
            n_missing: th.len(),
        });
    }
    let mut ob = obs.to_vec();
    sort_pairs(&mut ob);
    let omax = ob.iter().map(|x| x.1).fold(f64::MIN, f64::max);
    let ob: Vec<(f64, f64)> = ob
        .into_iter()
        .filter(|x| x.1 >= omax * o.noise_rel / 100.0)
        .collect();
    let mut used = vec![false; ob.len()];
    let mut matched: Vec<Option<usize>> = vec![];
    for p in &th {
        let win = p.mz * (3.0 * o.sigma_ppm).max(1e-9) * 1e-6 + 0.0005;
        let mut best: Option<usize> = None;
        for (j, (m, i)) in ob.iter().enumerate() {
            if !used[j] && (m - p.mz).abs() <= win && best.is_none_or(|b| *i > ob[b].1) {
                best = Some(j);
            }
        }
        if let Some(b) = best {
            used[b] = true;
        }
        matched.push(best);
    }
    let num = pysum(
        th.iter()
            .zip(&matched)
            .filter_map(|(p, m)| m.map(|j| p.rel * ob[j].1)),
    );
    let den = pysum(
        th.iter()
            .zip(&matched)
            .filter_map(|(p, m)| m.map(|_| p.rel * p.rel)),
    );
    let mut scale = if den > 0.0 { Some(num / den) } else { None };
    if matches!(o.norm, "mono" | "max" | "sum") {
        let mt: Vec<(&Peak, usize)> = th
            .iter()
            .zip(&matched)
            .filter_map(|(p, m)| m.map(|j| (p, j)))
            .collect();
        if o.norm == "sum" && !mt.is_empty() {
            scale = Some(
                pysum(mt.iter().map(|(_, j)| ob[*j].1)) / pysum(mt.iter().map(|(p, _)| p.rel)),
            );
        } else {
            let ai = if o.norm == "mono" {
                0
            } else {
                let mut b = 0;
                for (i, p) in th.iter().enumerate() {
                    if p.rel > th[b].rel {
                        b = i;
                    }
                }
                b
            };
            if let Some(j) = matched[ai] {
                scale = Some(ob[j].1 / th[ai].rel);
            }
        }
    }
    let zf = (z.abs()) as f64;
    let mut extra: Vec<Extra> = vec![];
    if let Some(sc) = scale.filter(|s| *s != 0.0) {
        for (j, (m, i)) in ob.iter().enumerate() {
            let r = i / sc;
            if !used[j]
                && r >= o.min_rel
                && (0.0..=MAX_SHIFT + 0.5).contains(&((m - th[0].mz) * zf))
                && *m >= th[0].mz - 0.3 / zf
            {
                extra.push(Extra {
                    mz: *m,
                    rel: r,
                    w: 0.0,
                });
            }
        }
    }
    let tot_w = th.len() as f64 + extra.len() as f64;
    let (mut peaks, mut s) = (vec![], 0.0);
    for (p, m) in th.iter().zip(&matched) {
        let w = if tot_w != 0.0 { 1.0 / tot_w } else { 0.0 };
        let mut row = PeakScore { w, ..blank(p) };
        if let (Some(j), Some(sc)) = (m, scale.filter(|s| *s != 0.0)) {
            let (mm, i) = ob[*j];
            let d = (mm - p.mz) / p.mz * 1e6;
            let r = i / sc;
            let tm = (-0.5 * (d / o.sigma_ppm).powi(2)).exp();
            let ti = if r > 0.0 {
                (-0.5 * ((r / p.rel).ln() / o.sigma_int).powi(2)).exp()
            } else {
                0.0
            };
            row.obs_mz = Some(mm);
            row.obs_rel = Some(r);
            row.d_ppm = Some(d);
            row.t_mass = tm;
            row.t_int = ti;
            row.t = tm * ti;
        }
        s += w * row.t;
        peaks.push(row);
    }
    for e in extra.iter_mut() {
        e.w = 1.0 / tot_w;
    }
    let n_missing = peaks.iter().filter(|r| r.obs_mz.is_none()).count();
    Ok(IsoScore {
        score: s,
        peaks,
        extra,
        n_missing,
    })
}

pub struct Frag {
    pub explained: Option<f64>,
    pub n_explained: Option<usize>,
    pub n_peaks: Option<usize>,
}

/// Share of the MS2 intensity explained by sub-formulas of the ion `formula` (charge `z`).
pub fn fragment_constraint(
    formula: &str,
    z: i32,
    frags: &[(f64, f64)],
    ppm: f64,
    min_mz: f64,
) -> Result<Frag> {
    let f = parse_formula(formula)?;
    let Ok(sm) = submasses(formula, z, min_mz) else {
        return Ok(Frag {
            explained: None,
            n_explained: None,
            n_peaks: None,
        });
    };
    let s = pysum(f.iter().map(|(e, k)| mass_of(e).unwrap() * *k as f64));
    let mono = (s - z as f64 * ELECTRON) / (z.abs() as f64);
    let peaks: Vec<(f64, f64)> = frags
        .iter()
        .copied()
        .filter(|(m, i)| min_mz <= *m && *m <= mono + 0.5 && *i > 0.0)
        .collect();
    if peaks.is_empty() {
        return Ok(Frag {
            explained: None,
            n_explained: Some(0),
            n_peaks: Some(0),
        });
    }
    let (mut tot, mut hit, mut n) = (0.0, 0.0, 0);
    for (m, i) in &peaks {
        tot += i;
        if !sm.is_empty() {
            let k = sm.partition_point(|v| v < m);
            let near = &sm[k.saturating_sub(1)..(k + 1).min(sm.len())];
            if near
                .iter()
                .map(|v| (v - m).abs())
                .fold(f64::INFINITY, f64::min)
                <= m * ppm * 1e-6
            {
                hit += i;
                n += 1;
            }
        }
    }
    Ok(Frag {
        explained: if tot != 0.0 { Some(hit / tot) } else { None },
        n_explained: Some(n),
        n_peaks: Some(peaks.len()),
    })
}

/// Weighted mean of the available terms (a missing term does not count).
pub fn combined(m_term: f64, iso: Option<f64>, frag: Option<f64>, w: (f64, f64, f64)) -> f64 {
    let mut parts = vec![(w.0, m_term)];
    if let Some(i) = iso {
        parts.push((w.1, i));
    }
    if let Some(f) = frag {
        parts.push((w.2, f));
    }
    let ws = pysum(parts.iter().map(|p| p.0));
    if ws > 0.0 {
        pysum(parts.iter().map(|(a, b)| a * b)) / ws
    } else {
        0.0
    }
}
