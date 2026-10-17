// Ported from Attributio src/attributio/review/explain.py.
//! Does the candidate structure explain the feature's MS/MS spectrum? The ion is the adduct of the formula with the smallest error among the common adducts
//! of the polarity; every cleaned peak is assigned the closest sub-formula of the ion within `ppm_frag` (RDBE ≥ -0.5; a Na/K adduct keeps its cation in every
//! fragment); the same peaks shifted by a few hundredths of a Da are assigned again (the decoy: the share of intensity explained by chance).
//! Errors are (observed - theoretical) / theoretical × 10⁶. A sub-formula supports the FORMULA, not the structure: isomers share it.

use super::adduct::{mass_of, ELECTRON};
use super::settings::ReviewSettings;
use crate::sortc::SortC;
use std::collections::BTreeMap;

pub const C13: f64 = 1.003355;

/// Elements of the review (`ATOM` of the reference tool), in the order they are listed there.
const ATOMS: [&str; 16] = [
    "C", "H", "N", "O", "P", "S", "F", "Cl", "Br", "I", "Na", "K", "Si", "B", "As", "Se",
];

/// Element → atoms. Sorted by symbol, like `sorted(ion)` of the reference tool.
pub type Formula = BTreeMap<String, i64>;

/// The mass of an element of the review, `None` outside the 16 elements.
pub fn atom_mass(el: &str) -> Option<f64> {
    ATOMS.contains(&el).then(|| mass_of(el)).flatten()
}

fn rdbe_weight(el: &str) -> f64 {
    match el {
        "C" | "Si" => 1.0,
        "N" | "P" | "As" | "B" => 0.5,
        "H" | "F" | "Cl" | "Br" | "I" | "Na" | "K" => -0.5,
        _ => 0.0,
    }
}

/// The common adducts of a polarity: name and the elements it adds (a negative count removes).
fn adducts(pol: &str) -> &'static [(&'static str, &'static [(&'static str, i64)])] {
    const POS: &[(&str, &[(&str, i64)])] = &[
        ("[M+H]+", &[("H", 1)]),
        ("[M+Na]+", &[("Na", 1)]),
        ("[M+NH4]+", &[("N", 1), ("H", 4)]),
        ("[M+K]+", &[("K", 1)]),
        ("[M-H2O+H]+", &[("H", -1), ("O", -1)]),
        ("[M]+", &[]),
    ];
    const NEG: &[(&str, &[(&str, i64)])] = &[
        ("[M-H]-", &[("H", -1)]),
        ("[M+Cl]-", &[("Cl", 1)]),
        ("[M+FA-H]-", &[("C", 1), ("H", 1), ("O", 2)]),
        ("[M-H2O-H]-", &[("H", -3), ("O", -1)]),
        ("[M]-", &[]),
    ];
    if pol == "POS" {
        POS
    } else {
        NEG
    }
}

// ------------------------------------------------------------------------------ formulas

/// `C10H22O6` → `{C: 10, H: 22, O: 6}` (charge signs and brackets ignored; text that is not an element symbol is skipped).
pub fn parse_formula(s: &str) -> Formula {
    let t = s.trim().trim_start_matches('[');
    let t = t.trim_end_matches(['+', '-']).trim_end_matches(']');
    let b = t.as_bytes();
    let mut d = Formula::new();
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_uppercase() {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        if j < b.len() && b[j].is_ascii_lowercase() {
            j += 1;
        }
        let k = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        let n = if j > k {
            t[k..j].parse().unwrap_or(1)
        } else {
            1
        };
        *d.entry(t[i..k].to_string()).or_insert(0) += n;
        i = j;
    }
    d
}

/// Hill order (C, H, then alphabetical), plain ASCII: `{C: 10, H: 22, O: 6}` → `C10H22O6`. Elements with a count of 0 or less are left out.
pub fn fmt_formula(d: &Formula) -> String {
    let mut order: Vec<&str> = ["C", "H"]
        .into_iter()
        .filter(|e| d.get(*e).is_some_and(|n| *n > 0))
        .collect();
    order.extend(
        d.iter()
            .filter(|(e, n)| **n > 0 && *e != "C" && *e != "H")
            .map(|(e, _)| e.as_str()),
    );
    let mut s = String::new();
    for e in order {
        s.push_str(e);
        if d[e] > 1 {
            s.push_str(&d[e].to_string());
        }
    }
    s
}

/// Signed difference: `{C: 1, H: 2}` → `+CH2`; `{H: -2, O: 1}` → `+O-H2`; empty → `0`.
pub fn fmt_delta(d: &Formula) -> String {
    let pos: Formula = d
        .iter()
        .filter(|(_, n)| **n > 0)
        .map(|(e, n)| (e.clone(), *n))
        .collect();
    let neg: Formula = d
        .iter()
        .filter(|(_, n)| **n < 0)
        .map(|(e, n)| (e.clone(), -*n))
        .collect();
    let (p, n) = (fmt_formula(&pos), fmt_formula(&neg));
    let s = format!(
        "{}{}",
        if p.is_empty() {
            String::new()
        } else {
            format!("+{p}")
        },
        if n.is_empty() {
            String::new()
        } else {
            format!("-{n}")
        }
    );
    if s.is_empty() {
        "0".into()
    } else {
        s
    }
}

/// m/z of the singly charged ion of a formula (electron mass taken off in positive mode and added in negative mode).
pub fn ion_mass(fd: &Formula, pol: &str) -> f64 {
    let t: f64 = fd
        .iter()
        .map(|(e, n)| atom_mass(e).unwrap_or(0.0) * *n as f64)
        .sum();
    if pol == "POS" {
        t - ELECTRON
    } else {
        t + ELECTRON
    }
}

/// The adduct of `fd` that fits `mz` best.
#[derive(Debug, Clone, PartialEq)]
pub struct AdductMatch {
    pub adduct: String,
    pub ion: Formula,
    pub ppm: f64,
}

/// The adduct with the smallest `|ppm|` (the first on a tie), or `None` for an empty formula or an element outside the 16.
pub fn match_adduct(fd: &Formula, mz: f64, pol: &str) -> Option<AdductMatch> {
    if fd.is_empty() || fd.keys().any(|e| atom_mass(e).is_none()) {
        return None;
    }
    let mut best: Option<AdductMatch> = None;
    for (name, add) in adducts(pol) {
        let mut ion = fd.clone();
        for (e, n) in *add {
            *ion.entry((*e).to_string()).or_insert(0) += n;
        }
        if ion.values().any(|n| *n < 0) {
            continue;
        }
        ion.retain(|_, n| *n > 0);
        let th = ion_mass(&ion, pol);
        let ppm = (mz - th) / th * 1e6;
        if best.as_ref().is_none_or(|b| ppm.abs() < b.ppm.abs()) {
            best = Some(AdductMatch {
                adduct: (*name).to_string(),
                ion,
                ppm,
            });
        }
    }
    best
}

// ------------------------------------------------------------------------------ sub-formulas

/// Every sub-formula of an ion with RDBE ≥ -0.5, sorted by mass. The counts are not stored: a row is its position in the grid of the elements (last element
/// fastest), decoded on demand.
#[derive(Debug, Clone)]
pub struct SubTable {
    pub elements: Vec<String>,
    radix: Vec<i64>,
    masses: Vec<f64>,
    cells: Vec<u32>,
}

impl SubTable {
    pub fn len(&self) -> usize {
        self.masses.len()
    }
    pub fn is_empty(&self) -> bool {
        self.masses.is_empty()
    }
    pub fn mass(&self, row: usize) -> f64 {
        self.masses[row]
    }
    /// The atoms of a row, in the order of `elements`.
    pub fn counts(&self, row: usize) -> Vec<i64> {
        let mut c = self.cells[row] as i64;
        let mut out = vec![0; self.radix.len()];
        for k in (0..self.radix.len()).rev() {
            out[k] = c % self.radix[k];
            c /= self.radix[k];
        }
        out
    }
    pub fn formula(&self, row: usize) -> Formula {
        self.elements
            .iter()
            .cloned()
            .zip(self.counts(row))
            .collect()
    }
}

/// The sub-formulas of `ion`, or `None` when the grid would hold more than `max_n` cells (or an element is outside the 16). A Na/K adduct keeps its cation.
pub fn subformula_table(ion: &Formula, pol: &str, adduct: &str, max_n: i64) -> Option<SubTable> {
    let els: Vec<String> = ion.keys().cloned().collect();
    let radix: Vec<i64> = els.iter().map(|e| ion[e] + 1).collect();
    let total = radix.iter().fold(1f64, |a, r| a * *r as f64);
    if total > max_n as f64 || total >= u32::MAX as f64 {
        return None;
    }
    let mass: Vec<f64> = els.iter().map(|e| atom_mass(e)).collect::<Option<_>>()?;
    let w: Vec<f64> = els.iter().map(|e| rdbe_weight(e)).collect();
    let cation = ["Na", "K"]
        .into_iter()
        .find(|c| adduct.contains(c) && (adduct == "[M+Na]+" || adduct == "[M+K]+"))
        .and_then(|c| els.iter().position(|e| e == c).map(|k| (k, ion[c])));
    let n = total as usize;
    let mut rows: Vec<(f64, u32)> = Vec::new();
    let mut counts = vec![0i64; els.len()];
    for cell in 0..n {
        let (mut tot, mut rdbe, mut atoms) = (0.0, 1.0, 0);
        for k in 0..els.len() {
            tot += counts[k] as f64 * mass[k];
            rdbe += counts[k] as f64 * w[k];
            atoms += counts[k];
        }
        let ok = rdbe >= -0.5 && atoms > 0 && cation.is_none_or(|(k, c)| counts[k] == c);
        if ok {
            let m = if pol == "POS" {
                tot - ELECTRON
            } else {
                tot + ELECTRON
            };
            rows.push((m, cell as u32));
        }
        for k in (0..els.len()).rev() {
            counts[k] += 1;
            if counts[k] < radix[k] {
                break;
            }
            counts[k] = 0;
        }
    }
    rows.sortc_by(|a, b| a.0.total_cmp(&b.0));
    Some(SubTable {
        elements: els,
        radix,
        masses: rows.iter().map(|r| r.0).collect(),
        cells: rows.iter().map(|r| r.1).collect(),
    })
}

/// For each m/z: the row of the table and the ppm error of the closest sub-formula within `ppm`, or `None`.
pub fn assign(mzs: &[f64], table: &SubTable, ppm: f64) -> Vec<Option<(usize, f64)>> {
    let masses = &table.masses;
    mzs.iter()
        .map(|&m| {
            let tol = m * ppm * 1e-6;
            let lo = masses.partition_point(|x| *x < m - tol);
            let hi = masses.partition_point(|x| *x < m + tol);
            if hi <= lo {
                return None;
            }
            let mut j = lo;
            for k in lo + 1..hi {
                if (masses[k] - m).abs() < (masses[j] - m).abs() {
                    j = k;
                }
            }
            Some((j, (m - masses[j]) / masses[j] * 1e6))
        })
        .collect()
}

/// (M+1/M, M+2/M) of an ion formula (nominal-mass isotopologue sums of the elements the reference tool lists; the others count as monoisotopic).
pub fn iso_pattern(ion: &Formula) -> (f64, f64) {
    fn table(e: &str) -> [f64; 3] {
        match e {
            "C" => [0.9893, 0.0107, 0.0],
            "H" => [0.999885, 0.000115, 0.0],
            "N" => [0.99636, 0.00364, 0.0],
            "O" => [0.99757, 0.00038, 0.00205],
            "S" => [0.9493, 0.0076, 0.0429],
            "Cl" => [0.7576, 0.0, 0.2424],
            "Br" => [0.5069, 0.0, 0.4931],
            "K" => [0.932581, 0.0, 0.067302],
            "Si" => [0.9223, 0.0468, 0.0309],
            _ => [1.0, 0.0, 0.0],
        }
    }
    let mut dist = [1.0, 0.0, 0.0];
    for (e, n) in ion {
        let a = table(e);
        for _ in 0..*n {
            let mut next = [0.0; 3];
            for (i, d) in dist.iter().enumerate() {
                for (j, x) in a.iter().enumerate() {
                    if i + j < 3 {
                        next[i + j] += d * x;
                    }
                }
            }
            dist = next;
        }
    }
    (dist[1] / dist[0], dist[2] / dist[0])
}

// ------------------------------------------------------------------------------ spectra

/// Why `prep_ms2` drops a peak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removed {
    Precursor,
    Below1Percent,
    MassDefect,
    Isotope13C,
    MergeSatellite,
}

impl Removed {
    pub fn label(self) -> &'static str {
        match self {
            Removed::Precursor => "precursor",
            Removed::Below1Percent => "below 1%",
            Removed::MassDefect => "mass defect",
            Removed::Isotope13C => "13C isotope",
            Removed::MergeSatellite => "merge satellite",
        }
    }
}

fn frac(m: f64) -> f64 {
    m - m.floor()
}

fn max_of(v: impl Iterator<Item = f64>) -> Option<f64> {
    v.fold(None, |a, x| Some(a.map_or(x, |a: f64| a.max(x))))
}

/// Peaks below the precursor - 0.5, ≥ `rel_min` of the base peak, without ¹³C isotope peaks, merge satellites and impossible mass defects.
/// Returns (m/z, intensity, index in the input).
pub fn prep_ms2(
    mz: &[f64],
    it: &[f64],
    prec: f64,
    rel_min: f64,
) -> (Vec<f64>, Vec<f64>, Vec<usize>) {
    let mut idx: Vec<usize> = (0..mz.len()).filter(|&i| mz[i] < prec - 0.5).collect();
    let Some(top) = max_of(idx.iter().map(|&i| it[i])) else {
        return (vec![], vec![], vec![]);
    };
    idx.retain(|&i| it[i] >= rel_min * top);
    idx.retain(|&i| !(frac(mz[i]) >= 0.40 && frac(mz[i]) <= 0.60 && mz[i] < 350.0));
    let (m, y): (Vec<f64>, Vec<f64>) = idx.iter().map(|&i| (mz[i], it[i])).unzip();
    let mut keep = Vec::new();
    for i in 0..m.len() {
        let tol = m[i] * 5e-6 + 0.0005;
        let iso = max_of(
            (0..m.len())
                .filter(|&j| (m[j] - (m[i] - C13)).abs() <= tol)
                .map(|j| y[j]),
        );
        let sat = max_of(
            (0..m.len())
                .filter(|&j| (m[j] - m[i]).abs() <= m[i] * 25e-6 && y[j] > y[i])
                .map(|j| y[j]),
        );
        let drop = iso.is_some_and(|t| y[i] < 0.5 * t) || sat.is_some_and(|t| y[i] < 0.3 * t);
        if !drop {
            keep.push(i);
        }
    }
    (
        keep.iter().map(|&i| m[i]).collect(),
        keep.iter().map(|&i| y[i]).collect(),
        keep.iter().map(|&i| idx[i]).collect(),
    )
}

/// `(index in the input, why)` for every peak `prep_ms2` drops, by index.
pub fn removed_reasons(mz: &[f64], it: &[f64], prec: f64, rel_min: f64) -> Vec<(usize, Removed)> {
    let kept = prep_ms2(mz, it, prec, rel_min).2;
    let base = max_of((0..mz.len()).filter(|&i| mz[i] < prec - 0.5).map(|i| it[i])).unwrap_or(0.0);
    let mut out = Vec::new();
    for i in 0..mz.len() {
        if kept.contains(&i) {
            continue;
        }
        let why = if mz[i] >= prec - 0.5 {
            Removed::Precursor
        } else if it[i] < rel_min * base {
            Removed::Below1Percent
        } else if (0.40..=0.60).contains(&frac(mz[i])) && mz[i] < 350.0 {
            Removed::MassDefect
        } else if mz
            .iter()
            .any(|&x| (x - (mz[i] - C13)).abs() <= mz[i] * 5e-6 + 0.0005)
        {
            Removed::Isotope13C
        } else {
            Removed::MergeSatellite
        };
        out.push((i, why));
    }
    out
}

// ------------------------------------------------------------------------------ the explanation

/// A peak assigned to a sub-formula of the ion.
#[derive(Debug, Clone, PartialEq)]
pub struct FragmentHit {
    pub formula: String,
    pub mz_th: f64,
    pub ppm: f64,
    pub loss: String,
}

/// The adduct and sub-formula explanation of a cleaned MS2 spectrum for one candidate formula.
#[derive(Debug, Clone, PartialEq)]
pub struct Explanation {
    pub adduct: Option<String>,
    pub ion: Option<Formula>,
    pub ion_formula: Option<String>,
    /// NaN without an adduct.
    pub ion_ppm: f64,
    /// One entry per peak.
    pub frags: Vec<Option<FragmentHit>>,
    pub n_expl: usize,
    pub frac_expl: f64,
    pub decoy_frac: f64,
}

impl Explanation {
    fn empty(n: usize) -> Explanation {
        Explanation {
            adduct: None,
            ion: None,
            ion_formula: None,
            ion_ppm: f64::NAN,
            frags: vec![None; n],
            n_expl: 0,
            frac_expl: 0.0,
            decoy_frac: 0.0,
        }
    }
    /// The precursor is explained within `ppm_ion`.
    pub fn fits(&self, s: &ReviewSettings) -> bool {
        self.adduct.is_some() && self.ion_ppm.abs() <= s.ppm_ion
    }
    /// Which peaks have a sub-formula.
    pub fn explained_mask(&self) -> Vec<bool> {
        self.frags.iter().map(Option::is_some).collect()
    }
}

/// Adduct and sub-formula explanation of the cleaned spectrum `(mz, it)` for one candidate formula (`None` = no formula).
pub fn explain(
    fd: Option<&Formula>,
    mz: f64,
    pol: &str,
    exp: (&[f64], &[f64]),
    s: &ReviewSettings,
) -> Explanation {
    let n = exp.0.len();
    let mut o = Explanation::empty(n);
    let Some(am) = fd.and_then(|f| match_adduct(f, mz, pol)) else {
        return o;
    };
    o.adduct = Some(am.adduct.clone());
    o.ion_ppm = am.ppm;
    o.ion_formula = Some(fmt_formula(&am.ion));
    o.ion = Some(am.ion.clone());
    if am.ppm.abs() > s.ppm_ion || n == 0 {
        return o;
    }
    let Some(table) = subformula_table(&am.ion, pol, &am.adduct, s.max_subformulas) else {
        return o;
    };
    let total: f64 = exp.1.iter().sum();
    for (i, h) in assign(exp.0, &table, s.ppm_frag).into_iter().enumerate() {
        let Some((row, ppm)) = h else { continue };
        let cnt = table.formula(row);
        let loss: Formula = am
            .ion
            .iter()
            .map(|(e, v)| (e.clone(), v - cnt.get(e).copied().unwrap_or(0)))
            .collect();
        let l = fmt_formula(&loss);
        o.frags[i] = Some(FragmentHit {
            formula: fmt_formula(&cnt),
            mz_th: table.mass(row),
            ppm,
            loss: if l.is_empty() { "-".into() } else { l },
        });
    }
    o.n_expl = o.frags.iter().flatten().count();
    let explained: f64 = o
        .frags
        .iter()
        .zip(exp.1)
        .filter(|(f, _)| f.is_some())
        .map(|(_, y)| *y)
        .sum();
    o.frac_expl = explained / total;
    let dec: Vec<f64> = s
        .decoy_shifts
        .iter()
        .map(|sh| {
            let shifted: Vec<f64> = exp.0.iter().map(|m| m + sh).collect();
            let hit: f64 = assign(&shifted, &table, s.ppm_frag)
                .iter()
                .zip(exp.1)
                .filter(|(h, _)| h.is_some())
                .map(|(_, y)| *y)
                .sum();
            hit / total
        })
        .collect();
    o.decoy_frac = dec.iter().sum::<f64>() / dec.len() as f64;
    o
}
