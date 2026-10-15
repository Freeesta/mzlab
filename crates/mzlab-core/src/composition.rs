//! Elemental composition of an ion from its exact m/z (`mzlab/chem/composition.py`): the formulas that fit, with the limits the student
//! chose. Candidates, never a verdict. This port answers the requests WITHOUT observed isotope peaks (`obs`) or MS2 peaks (`frags`):
//! those need `isoscore` and the caller leaves them to Python (`Opts::scored`).
//!
//! The candidates are the ones numpy builds: for every combination of the elements other than C and H and every number of carbons, the
//! hydrogens that close the mass (rounded value -1, 0, +1), then the same filters in the same order; ties are broken as `np.lexsort` does
//! on the rows sorted as `np.unique(axis=0)` sorts them.

use crate::elements::{mass_of, parse_formula, Formula, ELECTRON};
use crate::error::{Error, Result};
use crate::isoscore::{combined, fragment_constraint, isotope_score, Frag, IsoOpts, IsoScore};
use crate::json::{round, Out};

pub type Counts = Vec<(String, i64)>;

const ISOTOPES: [(&str, f64, &str); 7] = [
    ("2H", 2.01410177812, "H"),
    ("13C", 13.00335483507, "C"),
    ("15N", 15.00010889888, "N"),
    ("18O", 17.99915961286, "O"),
    ("34S", 33.96786701, "S"),
    ("37Cl", 36.96590260, "Cl"),
    ("81Br", 80.9162897, "Br"),
];
const DBEQ: [(&str, f64); 13] = [
    ("C", 1.0),
    ("H", -0.5),
    ("N", 0.5),
    ("O", 0.0),
    ("S", 0.0),
    ("P", 0.5),
    ("F", -0.5),
    ("Cl", -0.5),
    ("Br", -0.5),
    ("I", -0.5),
    ("Si", 1.0),
    ("Na", -0.5),
    ("K", -0.5),
];
const VALENCE: [(&str, i64); 13] = [
    ("C", 4),
    ("H", 1),
    ("N", 3),
    ("O", 2),
    ("S", 2),
    ("P", 3),
    ("F", 1),
    ("Cl", 1),
    ("Br", 1),
    ("I", 1),
    ("Si", 4),
    ("Na", 1),
    ("K", 1),
];
const DEFAULT_ELEMENTS: [(&str, i64, i64); 9] = [
    ("C", 0, 100),
    ("H", 0, 200),
    ("N", 0, 10),
    ("O", 0, 15),
    ("S", 0, 3),
    ("P", 0, 3),
    ("F", 0, 6),
    ("Cl", 0, 3),
    ("Br", 0, 2),
];
const ATOMIC_NUMBER: [(&str, i64); 14] = [
    ("H", 1),
    ("C", 6),
    ("N", 7),
    ("O", 8),
    ("F", 9),
    ("Na", 11),
    ("Si", 14),
    ("P", 15),
    ("S", 16),
    ("Cl", 17),
    ("K", 19),
    ("Br", 35),
    ("I", 53),
    ("Se", 34),
];
type Adduct = (&'static str, &'static [(&'static str, i64)], i32);
const ADDUCT_ATOMS: [Adduct; 11] = [
    ("[M+H]+", &[("H", 1)], 1),
    ("[M+NH4]+", &[("N", 1), ("H", 4)], 1),
    ("[M+Na]+", &[("Na", 1)], 1),
    ("[M+K]+", &[("K", 1)], 1),
    ("[M-H]-", &[("H", -1)], -1),
    ("[M+Cl]-", &[("Cl", 1)], -1),
    ("[M+HCOO]-", &[("C", 1), ("H", 1), ("O", 2)], -1),
    ("M+.", &[], 1),
    ("M-.", &[], -1),
    ("[M]+", &[], 1),
    ("[M]-", &[], -1),
];
const RATIO_RULES: [(&str, &str, f64, f64); 8] = [
    ("HC", "H", 0.2, 3.1),
    ("NC", "N", 0.0, 1.3),
    ("OC", "O", 0.0, 1.2),
    ("PC", "P", 0.0, 0.3),
    ("SC", "S", 0.0, 0.8),
    ("FC", "F", 0.0, 1.5),
    ("ClC", "Cl", 0.0, 0.8),
    ("BrC", "Br", 0.0, 0.8),
];
const RULES: [&str; 10] = [
    "HC", "NC", "OC", "PC", "SC", "FC", "ClC", "BrC", "LEWIS", "SENIOR",
];
const ISO_PATTERN: [(&str, &[(usize, f64)]); 13] = [
    ("C", &[(0, 0.9893), (1, 0.0107)]),
    ("H", &[(0, 0.999885), (1, 0.000115)]),
    ("N", &[(0, 0.99636), (1, 0.00364)]),
    ("O", &[(0, 0.99757), (1, 0.00038), (2, 0.00205)]),
    ("S", &[(0, 0.9499), (1, 0.0075), (2, 0.0425), (4, 0.0001)]),
    ("Cl", &[(0, 0.7576), (2, 0.2424)]),
    ("Br", &[(0, 0.5069), (2, 0.4931)]),
    ("F", &[(0, 1.0)]),
    ("P", &[(0, 1.0)]),
    ("I", &[(0, 1.0)]),
    ("Na", &[(0, 1.0)]),
    ("K", &[(0, 0.932581), (2, 0.067302)]),
    ("Si", &[(0, 0.9223), (1, 0.0468), (2, 0.0309)]),
];
const MAX_GRID: u64 = 3_000_000;

fn lookup<T: Copy>(t: &[(&str, T)], k: &str) -> Option<T> {
    t.iter().find(|(e, _)| *e == k).map(|(_, v)| *v)
}
fn iso_info(sym: &str) -> Option<(f64, &'static str)> {
    ISOTOPES
        .iter()
        .find(|(s, _, _)| *s == sym)
        .map(|(_, m, b)| (*m, *b))
}
fn sym_mass(sym: &str) -> f64 {
    iso_info(sym)
        .map(|(m, _)| m)
        .or_else(|| mass_of(sym))
        .unwrap_or(0.0)
}
fn base_of(sym: &str) -> &str {
    iso_info(sym).map_or(sym, |(_, b)| b)
}

/// Element ranges in the order they were written: 'C:0-13,H:0-25,N:0-4,13C:0-2'. A single number means that exact count.
pub type Ranges = Vec<(String, (i64, i64))>;

pub fn parse_elements(text: &str) -> Result<Ranges> {
    let mut out: Ranges = vec![];
    for part in text.split([',', ';']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let bad = || Error::new("err.comp.element").with("part", part);
        let c: Vec<char> = part.chars().collect();
        let mut i = 0;
        while i < c.len() && c[i].is_ascii_digit() {
            i += 1;
        }
        if !(i < c.len() && c[i].is_ascii_uppercase()) {
            return Err(bad());
        }
        i += 1;
        if i < c.len() && c[i].is_ascii_lowercase() {
            i += 1;
        }
        let sym: String = c[..i].iter().collect();
        let skip = |i: &mut usize| {
            while *i < c.len() && c[*i].is_whitespace() {
                *i += 1;
            }
        };
        skip(&mut i);
        if !(i < c.len() && c[i] == ':') {
            return Err(bad());
        }
        i += 1;
        skip(&mut i);
        let num = |i: &mut usize| -> Option<i64> {
            let a = *i;
            while *i < c.len() && c[*i].is_ascii_digit() {
                *i += 1;
            }
            (*i > a).then(|| {
                c[a..*i]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .unwrap_or(i64::MAX / 4)
            })
        };
        let lo = num(&mut i).ok_or_else(bad)?;
        let mut hi = lo;
        let save = i;
        skip(&mut i);
        if i < c.len() && c[i] == '-' {
            i += 1;
            skip(&mut i);
            hi = num(&mut i).ok_or_else(bad)?;
        } else {
            i = save;
        }
        if i != c.len() {
            return Err(bad());
        }
        if mass_of(&sym).is_none() && iso_info(&sym).is_none() {
            return Err(Error::new("err.comp.unknown").with("symbol", sym));
        }
        if hi < lo {
            return Err(Error::new("err.comp.range").with("symbol", sym));
        }
        match out.iter_mut().find(|(s, _)| *s == sym) {
            Some(x) => x.1 = (lo, hi),
            None => out.push((sym, (lo, hi))),
        }
    }
    Ok(out)
}

/// Hill order (C, H, then alphabetical); isotopes written as 13C2 after the element they replace.
pub fn format_formula(counts: &[(String, i64)]) -> String {
    let items: Vec<&(String, i64)> = counts.iter().filter(|(_, v)| *v != 0).collect();
    let has = |k: &str| items.iter().any(|(s, _)| s == k);
    let mut order: Vec<&str> = vec![];
    if has("C") {
        order.push("C");
    }
    if has("H") {
        order.push("H");
    }
    let mut plain: Vec<&str> = items
        .iter()
        .map(|(s, _)| s.as_str())
        .filter(|s| *s != "C" && *s != "H" && iso_info(s).is_none())
        .collect();
    crate::sortc::ord(&mut plain);
    let mut iso: Vec<&str> = items
        .iter()
        .map(|(s, _)| s.as_str())
        .filter(|s| iso_info(s).is_some())
        .collect();
    crate::sortc::ord(&mut iso);
    order.extend(plain);
    order.extend(iso);
    order
        .iter()
        .map(|k| {
            let v = items.iter().find(|(s, _)| s == k).unwrap().1;
            if v != 1 {
                format!("{k}{v}")
            } else {
                k.to_string()
            }
        })
        .collect()
}

pub fn ion_mass(counts: &[(String, i64)], z: i32) -> f64 {
    let s = crate::pysum::pysum(counts.iter().map(|(k, v)| sym_mass(k) * *v as f64));
    (s - z as f64 * ELECTRON) / (z.abs() as f64)
}

/// (M+1 / M, M+2 / M) of an ion formula at the natural abundances.
pub fn isotope_ratios(counts: &[(String, i64)]) -> (f64, f64) {
    fn conv(a: &[f64], b: &[f64]) -> Vec<f64> {
        let n = (a.len() + b.len() - 1).min(5);
        (0..n)
            .map(|k| {
                (0..a.len())
                    .filter(|&i| k >= i && k - i < b.len())
                    .map(|i| a[i] * b[k - i])
                    .fold(0.0, |s, v| s + v)
            })
            .collect()
    }
    let mut pat = vec![1.0];
    for (el, k) in counts {
        if *k <= 0 || iso_info(el).is_some() {
            continue;
        }
        let Some(iso) = ISO_PATTERN.iter().find(|(e, _)| e == el).map(|(_, p)| *p) else {
            continue;
        };
        let mut single = vec![0.0; iso.iter().map(|(s, _)| *s).max().unwrap() + 1];
        for (s, a) in iso {
            single[*s] += a;
        }
        let tot: f64 = single.iter().sum();
        single.iter_mut().for_each(|v| *v /= tot);
        let (mut acc, mut base, mut e) = (vec![1.0], single, *k);
        while e > 0 {
            if e & 1 == 1 {
                acc = conv(&acc, &base);
            }
            base = conv(&base, &base);
            e >>= 1;
        }
        pat = conv(&pat, &acc);
    }
    pat.resize(pat.len().max(5) + 5, 0.0);
    (pat[1] / pat[0], pat[2] / pat[0])
}

pub struct Opts<'a> {
    pub elements: Option<Ranges>,
    pub ion: &'a str,
    pub z: Option<i32>,
    pub tol: f64,
    pub unit: &'a str,
    pub rdb: (f64, f64),
    pub nitrogen: &'a str,
    pub rules: Vec<String>,
    pub max_results: usize,
    pub parent: Option<&'a str>,
    pub m1: Option<f64>,
    pub m2: Option<f64>,
    pub iso_tol: f64,
    /// Observed isotope peaks (m/z, intensity) and MS2 peaks of the precursor: they switch the combined score on.
    pub obs: Vec<(f64, f64)>,
    pub frags: Vec<(f64, f64)>,
    pub r200: f64,
    pub sigma_ppm: f64,
    pub sigma_int: f64,
    pub weights: (f64, f64, f64),
    pub pool: usize,
    pub iso_min_rel: f64,
    pub iso_noise_rel: f64,
    pub frag_ppm: Option<f64>,
    pub norm: &'a str,
}

impl Default for Opts<'_> {
    fn default() -> Self {
        Opts {
            elements: None,
            ion: "[M+H]+",
            z: None,
            tol: 5.0,
            unit: "ppm",
            rdb: (-1.0, 100.0),
            nitrogen: "none",
            rules: vec![],
            max_results: 10,
            parent: None,
            m1: None,
            m2: None,
            iso_tol: 0.20,
            obs: vec![],
            frags: vec![],
            r200: 70000.0,
            sigma_ppm: 3.0,
            sigma_int: 0.10,
            weights: (1.0, 2.0, 1.0),
            pool: 100,
            iso_min_rel: 1.0,
            iso_noise_rel: 1.0,
            frag_ppm: None,
            norm: "fit",
        }
    }
}

/// Counts by base element (isotopes folded in), in the order of `bases`.
fn folded(row: &[i64], base: &[&str], bases: &[&str]) -> Vec<i64> {
    bases
        .iter()
        .map(|b| {
            row.iter()
                .zip(base)
                .filter(|(_, x)| *x == b)
                .map(|(v, _)| *v)
                .sum()
        })
        .collect()
}

/// Which rules a row BREAKS, in the order of `RULES`.
fn broken(form: &[i64], bases: &[&str], r_db: f64, half_ok: bool) -> [bool; 10] {
    let at = |s: &str| bases.iter().position(|b| *b == s).map_or(0, |i| form[i]);
    let c = at("C");
    let mut out = [false; 10];
    for (i, (_, sym, lo, hi)) in RATIO_RULES.iter().enumerate() {
        let ratio = if c > 0 {
            at(sym) as f64 / c as f64
        } else {
            0.0
        };
        out[i] = c > 0 && (ratio < lo - 1e-12 || ratio > hi + 1e-12);
    }
    let whole = (r_db - r_db.round_ties_even()).abs() < 1e-6
        || (half_ok && (r_db - r_db.floor() - 0.5).abs() < 1e-6);
    out[8] = r_db < 0.0 || !whole;
    let val = |b: &str| lookup(&VALENCE, b).unwrap_or(0);
    let tot: i64 = bases.iter().zip(form).map(|(b, n)| val(b) * n).sum();
    let n_atoms: i64 = form.iter().sum();
    let vmax = bases
        .iter()
        .zip(form)
        .map(|(b, n)| if *n > 0 { val(b) } else { 0 })
        .max()
        .unwrap_or(0);
    out[9] = n_atoms > 0 && !(tot % 2 == 0 && tot >= 2 * vmax && tot >= 2 * (n_atoms - 1));
    out
}

/// Formulas of the ion at `mz`, as the JSON of `/api/composition` (without the isotope-peak and fragment scores).
pub fn compose(mz: f64, o: &Opts) -> Result<String> {
    if mz.is_nan() || mz <= 0.0 {
        return Err(Error::new("err.comp.mz"));
    }
    let Some(&(_, add, ch)) = ADDUCT_ATOMS.iter().find(|(n, _, _)| *n == o.ion) else {
        return Err(Error::new("err.comp.ion").with("ion", o.ion));
    };
    let z = match o.z {
        Some(z) if z != 0 => z.abs() * if ch > 0 { 1 } else { -1 },
        _ => ch,
    };
    if (z > 0) != (ch > 0) {
        return Err(Error::new("err.comp.charge"));
    }
    if o.unit != "ppm" && o.unit != "mda" {
        return Err(Error::new("err.comp.unit"));
    }
    let mut els: Ranges = match &o.elements {
        Some(e) if !e.is_empty() => e.clone(),
        _ => DEFAULT_ELEMENTS
            .iter()
            .map(|(s, a, b)| (s.to_string(), (*a, *b)))
            .collect(),
    };
    for (sym, n) in add.iter().filter(|(_, n)| *n > 0) {
        match els.iter_mut().find(|(s, _)| s == sym) {
            Some(x) => x.1 = (x.1 .0.max(*n), x.1 .1.max(*n)),
            None => els.push((sym.to_string(), (*n, *n))),
        }
    }
    let par: Option<Formula> = match o.parent {
        Some(p) if !p.is_empty() => Some(parse_formula(p)?),
        _ => None,
    };
    let zf = z as f64;
    let t = mz * zf.abs() + zf * ELECTRON;
    let tol_da = (if o.unit == "ppm" {
        mz * o.tol * 1e-6
    } else {
        o.tol / 1000.0
    }) * zf.abs();
    if let Some(p) = &par {
        for (sym, r) in els.iter_mut() {
            let hi = p
                .iter()
                .find(|(e, _)| e == sym)
                .map_or(0, |(_, n)| *n as i64);
            *r = (r.0.min(hi), r.1.min(hi));
        }
        els.retain(|(_, r)| r.1 >= r.0);
    }
    let get = |s: &str| els.iter().find(|(e, _)| e == s).map(|(_, r)| *r);
    let mh = mass_of("H").unwrap();
    let (cmin, cmax) = get("C").unwrap_or((0, (t / 12.0).floor() as i64));
    let (hmin, hmax) = get("H").unwrap_or((0, (t / mh).floor() as i64 + 1));
    let others: Vec<&(String, (i64, i64))> =
        els.iter().filter(|(s, _)| s != "C" && s != "H").collect();
    let mut size: u64 = 1;
    for (_, r) in &others {
        size = size.saturating_mul((r.1 - r.0 + 1) as u64);
    }
    if size > MAX_GRID {
        return Err(Error::new("err.comp.many"));
    }
    let mo: Vec<f64> = others.iter().map(|(s, _)| sym_mass(s)).collect();
    let empty = |ion: &str| {
        let mut e = Out::default();
        e.raw("{\"mz\":")
            .num(mz)
            .raw(",\"charge\":")
            .int(z as i64)
            .raw(",\"tol\":")
            .num(o.tol)
            .raw(",\"unit\":")
            .str(o.unit)
            .raw(",\"ion\":")
            .str(ion);
        e.raw(",\"n_candidates\":0,\"results\":[]}");
        e.0
    };
    // hits: (carbons, hydrogens, grid row, mass error)
    let mut hits: Vec<(i64, i64, Vec<i64>, f64)> = vec![];
    let mut cur: Vec<i64> = others.iter().map(|(_, r)| r.0).collect();
    for _ in 0..size {
        let mut dot = 0.0;
        for (i, v) in cur.iter().enumerate() {
            dot += *v as f64 * mo[i];
        }
        let r = if others.is_empty() { t } else { t - dot };
        if r >= -tol_da && r <= mh * hmax as f64 + 12.0 * cmax as f64 + tol_da {
            let clo = (cmin as f64).max(((r - mh * hmax as f64 - tol_da) / 12.0).ceil()) as i64;
            let chi = (cmax as f64).min(((r + tol_da - mh * hmin as f64) / 12.0).floor()) as i64;
            for cm in clo..=chi {
                let h0 = (r - 12.0 * cm as f64) / mh;
                for d in [-1i64, 0, 1] {
                    let h = h0.round_ties_even() as i64 + d;
                    let err = r - 12.0 * cm as f64 - h as f64 * mh;
                    if err.abs() <= tol_da && h >= hmin && h <= hmax {
                        hits.push((cm, h, cur.clone(), err));
                    }
                }
            }
        }
        for d in (0..cur.len()).rev() {
            if cur[d] < others[d].1 .1 {
                cur[d] += 1;
                break;
            }
            cur[d] = others[d].1 .0;
        }
    }
    if hits.is_empty() {
        return Ok(empty(o.ion));
    }
    let mut syms: Vec<String> = vec!["C".into(), "H".into()];
    syms.extend(others.iter().map(|(s, _)| s.clone()));
    let base: Vec<&str> = syms.iter().map(|s| base_of(s)).collect();
    let mut bases: Vec<&str> = base.clone();
    crate::sortc::ord(&mut bases);
    bases.dedup();
    let a: Vec<i64> = syms
        .iter()
        .map(|k| add.iter().find(|(e, _)| e == k).map_or(0, |(_, n)| *n))
        .collect();
    let db = |b: &str| lookup(&DBEQ, b).unwrap_or(0.0);
    struct Cand {
        x: Vec<i64>,
        nn: Vec<i64>,
        e: f64,
        r_db: f64,
        failed: [bool; 10],
    }
    let mut cands: Vec<Cand> = vec![];
    for (cm, h, g, e) in hits {
        let mut x = vec![cm, h];
        x.extend(g);
        if x.iter().sum::<i64>() <= 0 {
            continue;
        }
        let b = folded(&x, &base, &bases);
        if let Some(p) = &par {
            if bases.iter().zip(&b).any(|(bb, n)| {
                *n > p
                    .iter()
                    .find(|(e, _)| e == bb)
                    .map_or(0, |(_, c)| *c as i64)
            }) {
                continue;
            }
        }
        let nn: Vec<i64> = x.iter().zip(&a).map(|(v, d)| v - d).collect();
        if nn.iter().any(|v| *v < 0) {
            continue;
        }
        let bn = folded(&nn, &base, &bases);
        let form = if add.is_empty() { &b } else { &bn };
        let mut r_db = 0.0;
        for (bb, n) in bases.iter().zip(form) {
            r_db += *n as f64 * db(bb);
        }
        let r_db = 1.0 + r_db;
        if !(r_db >= o.rdb.0 && r_db <= o.rdb.1) {
            continue;
        }
        if o.nitrogen != "none" {
            let n_e: i64 = bases
                .iter()
                .zip(&b)
                .map(|(bb, n)| lookup(&ATOMIC_NUMBER, bb).unwrap_or(0) * n)
                .sum::<i64>()
                - z as i64;
            let odd = n_e.rem_euclid(2) == 1;
            if (o.nitrogen == "even") == odd {
                continue;
            }
        }
        let failed = broken(form, &bases, r_db, o.ion == "[M]+" || o.ion == "[M]-");
        if o.rules.iter().any(|rn| {
            RULES
                .iter()
                .position(|r| r == rn)
                .is_some_and(|i| failed[i])
        }) {
            continue;
        }
        cands.push(Cand {
            x,
            nn,
            e,
            r_db,
            failed,
        });
    }
    // np.unique(axis=0): rows in lexicographic order, the first of equal rows kept
    crate::sortc::slice_by(&mut cands, |p, q| p.x.cmp(&q.x));
    cands.dedup_by(|b, a| a.x == b.x);
    if cands.is_empty() {
        return Ok(empty(o.ion));
    }
    let n_candidates = cands.len();
    // np.lexsort((nfail, |E|)): by |E|, then by the rules broken; stable
    crate::sortc::slice_by(&mut cands, |p, q| {
        let nf = |c: &Cand| c.failed.iter().filter(|b| **b).count();
        p.e.abs().total_cmp(&q.e.abs()).then(nf(p).cmp(&nf(q)))
    });
    struct Res {
        formula: String,
        neutral: Option<String>,
        mz_calc: f64,
        delta_mda: f64,
        delta_ppm: f64,
        rdb: f64,
        bad: Vec<&'static str>,
        t1: f64,
        t2: f64,
        errs: Option<(Option<f64>, Option<f64>, bool)>,
        iso: Option<IsoScore>,
        mass_t: f64,
        iso_t: Option<f64>,
        frag_t: Option<f64>,
        total: f64,
        frag: Option<Frag>,
    }
    let scored = !o.obs.is_empty() || !o.frags.is_empty();
    let take = if scored {
        o.max_results.max(o.pool)
    } else {
        o.max_results
    };
    let mut res: Vec<Res> = vec![];
    for c in cands.iter().take(take) {
        let counts: Counts = syms
            .iter()
            .cloned()
            .zip(c.x.iter().copied())
            .filter(|(_, v)| *v != 0)
            .collect();
        let neutral: Counts = syms
            .iter()
            .cloned()
            .zip(c.nn.iter().copied())
            .filter(|(_, v)| *v != 0)
            .collect();
        let calc = ion_mass(&counts, z);
        let d_da = calc - mz;
        let bad: Vec<&'static str> = RULES
            .iter()
            .zip(&c.failed)
            .filter(|(_, f)| **f)
            .map(|(r, _)| *r)
            .collect();
        let (t1, t2) = isotope_ratios(&counts);
        let errs = if o.m1.is_some() || o.m2.is_some() {
            let e1 = o.m1.map(|m| (m - t1).abs() / t1.max(1e-9));
            let e2 = o.m2.map(|m| (m - t2).abs() / t2.max(1e-9));
            Some((
                e1.map(|e| round(e, 3)),
                e2.map(|e| round(e, 3)),
                [e1, e2].iter().flatten().all(|e| *e <= o.iso_tol),
            ))
        } else {
            None
        };
        let iso = if o.obs.is_empty() {
            None
        } else {
            let io = IsoOpts {
                r200: o.r200,
                sigma_ppm: o.sigma_ppm,
                sigma_int: o.sigma_int,
                min_rel: o.iso_min_rel,
                noise_rel: o.iso_noise_rel,
                norm: o.norm,
            };
            Some(isotope_score(&counts, z, &o.obs, &io)?)
        };
        let delta_ppm = round(d_da / mz * 1e6, 3);
        let iso_t = iso.as_ref().map(|s| round(s.score, 4));
        let total = if scored {
            round(
                combined(mass_t_raw(delta_ppm, o.sigma_ppm), iso_t, None, o.weights),
                4,
            )
        } else {
            0.0
        };
        res.push(Res {
            formula: format_formula(&counts),
            neutral: if add.is_empty() {
                None
            } else {
                Some(format_formula(&neutral))
            },
            mz_calc: round(calc, 5),
            delta_mda: round(d_da * 1000.0, 3),
            delta_ppm,
            rdb: c.r_db,
            bad,
            t1,
            t2,
            errs,
            iso,
            mass_t: round(mass_t_raw(delta_ppm, o.sigma_ppm), 4),
            iso_t,
            frag_t: None,
            total,
            frag: None,
        });
    }
    if scored {
        crate::sortc::slice_by(&mut res, |a, b| b.total.total_cmp(&a.total));
        if !o.frags.is_empty() {
            res.truncate(o.max_results.max(40));
            let ppm = o
                .frag_ppm
                .filter(|p| *p != 0.0)
                .unwrap_or(if o.unit == "ppm" {
                    o.tol.max(5.0)
                } else {
                    10.0
                });
            for r in res.iter_mut() {
                let f = fragment_constraint(&r.formula, z, &o.frags, ppm, 50.0)?;
                r.frag_t = f.explained.map(|v| round(v, 4));
                r.total = round(combined(r.mass_t, r.iso_t, f.explained, o.weights), 4);
                r.frag = Some(f);
            }
            crate::sortc::slice_by(&mut res, |a, b| b.total.total_cmp(&a.total));
        }
        res.truncate(o.max_results);
    }
    let mut out = Out::default();
    out.raw("{\"mz\":")
        .num(mz)
        .raw(",\"charge\":")
        .int(z as i64)
        .raw(",\"tol\":")
        .num(o.tol)
        .raw(",\"unit\":")
        .str(o.unit)
        .raw(",\"ion\":")
        .str(o.ion);
    out.raw(",\"n_candidates\":")
        .int(n_candidates as i64)
        .raw(",\"results\":[");
    for (i, r) in res.iter().enumerate() {
        out.raw(if i > 0 { "," } else { "" })
            .raw("{\"formula\":")
            .str(&r.formula)
            .raw(",\"neutral\":");
        match &r.neutral {
            Some(n) => out.str(n),
            None => out.raw("null"),
        };
        out.raw(",\"mz_calc\":")
            .num(r.mz_calc)
            .raw(",\"delta_mda\":")
            .num(r.delta_mda)
            .raw(",\"delta_ppm\":")
            .num(r.delta_ppm);
        out.raw(",\"rdb\":").num(r.rdb).raw(",\"rules_failed\":[");
        for (j, b) in r.bad.iter().enumerate() {
            out.raw(if j > 0 { "," } else { "" }).str(b);
        }
        out.raw("],\"rules_ok\":")
            .int((RULES.len() - r.bad.len()) as i64);
        out.raw(",\"iso\":{\"m1\":")
            .num(round(r.t1, 4))
            .raw(",\"m2\":")
            .num(round(r.t2, 4));
        if let Some((e1, e2, ok)) = r.errs {
            out.raw(",\"err1\":")
                .opt(e1)
                .raw(",\"err2\":")
                .opt(e2)
                .raw(",\"ok\":")
                .raw(if ok { "true" } else { "false" });
        }
        if let Some(s) = &r.iso {
            out.raw(",\"score\":")
                .num(round(s.score, 4))
                .raw(",\"peaks\":[");
            for (j, p) in s.peaks.iter().enumerate() {
                out.raw(if j > 0 { "," } else { "" })
                    .raw("{\"mz\":")
                    .num(p.mz)
                    .raw(",\"rel\":")
                    .num(p.rel)
                    .raw(",\"obs_mz\":")
                    .opt(p.obs_mz)
                    .raw(",\"obs_rel\":")
                    .opt(p.obs_rel);
                out.raw(",\"d_ppm\":")
                    .opt(p.d_ppm)
                    .raw(",\"t_mass\":")
                    .num(p.t_mass)
                    .raw(",\"t_int\":")
                    .num(p.t_int)
                    .raw(",\"t\":")
                    .num(p.t)
                    .raw(",\"w\":")
                    .num(p.w)
                    .raw("}");
            }
            out.raw("],\"extra\":[");
            for (j, e) in s.extra.iter().enumerate() {
                out.raw(if j > 0 { "," } else { "" })
                    .raw("{\"mz\":")
                    .num(e.mz)
                    .raw(",\"rel\":")
                    .num(e.rel)
                    .raw(",\"w\":")
                    .num(e.w)
                    .raw("}");
            }
            out.raw("],\"n_missing\":")
                .int(s.n_missing as i64)
                .raw(",\"n_extra\":")
                .int(s.extra.len() as i64);
        }
        out.raw("}");
        if scored {
            out.raw(",\"score\":{\"mass\":")
                .num(r.mass_t)
                .raw(",\"iso\":")
                .opt(r.iso_t)
                .raw(",\"frag\":")
                .opt(r.frag_t)
                .raw(",\"total\":")
                .num(r.total)
                .raw("}");
        }
        if let Some(f) = &r.frag {
            out.raw(",\"frag\":{\"explained\":")
                .opt(f.explained)
                .raw(",\"n_explained\":");
            match f.n_explained {
                Some(n) => out.int(n as i64),
                None => out.raw("null"),
            };
            out.raw(",\"n_peaks\":");
            match f.n_peaks {
                Some(n) => out.int(n as i64),
                None => out.raw("null"),
            };
            out.raw("}");
        }
        out.raw("}");
    }
    out.raw("]}");
    Ok(out.0)
}

/// The Gaussian mass term of the combined score, from the rounded error in ppm.
fn mass_t_raw(delta_ppm: f64, sigma_ppm: f64) -> f64 {
    (-0.5 * (delta_ppm / sigma_ppm).powi(2)).exp()
}
