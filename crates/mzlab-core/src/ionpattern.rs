//! Unit-resolution isotope pattern of an ion (`api/ion_pattern`): the isotopologues of a formula plus an adduct of the catalogue, grouped by
//! nominal mass offset (M, M+1, M+2...), each group with its total probability and probability-weighted m/z. Used by the spectrum view
//! (simulated pattern) of the page, the app and `mzlab-cli ion-pattern`; the fine structure of high resolution is `isoscore::fine_pattern`.

use crate::adduct_table::composition;
use crate::elements::{parse_formula, ELECTRON};
use crate::error::{Error, Result};
use crate::isoscore::isotopes_of;
use crate::json::Out;
use std::collections::BTreeMap;

const PRUNE: f64 = 1e-9;
const MAX_OFFSET: i32 = 40;
const MIN_REL: f64 = 0.05;

/// offset -> (probability, probability x mass)
type Dist = BTreeMap<i32, (f64, f64)>;

fn conv(a: &Dist, b: &Dist) -> Dist {
    let mut out = Dist::new();
    for (oa, (pa, ma)) in a {
        for (ob, (pb, mb)) in b {
            let (o, p) = (oa + ob, pa * pb);
            if o > MAX_OFFSET || p < PRUNE {
                continue;
            }
            let cur = out.entry(o).or_insert((0.0, 0.0));
            cur.0 += p;
            cur.1 += ma * pb + mb * pa;
        }
    }
    out
}

/// Distribution of one element, offsets counted from its lightest isotope.
fn element_dist(el: &str) -> Result<Dist> {
    let iso = isotopes_of(el).ok_or_else(|| Error::new("err.iso.element").with("symbol", el))?;
    let a0 = iso.iter().map(|i| i.0.round() as i32).min().unwrap_or(0);
    Ok(iso
        .iter()
        .filter(|i| i.1 > 0.0)
        .map(|&(m, p)| (m.round() as i32 - a0, (p, m * p)))
        .collect())
}

fn power(d: Dist, mut n: i64) -> Dist {
    let (mut res, mut base) = (Dist::from([(0, (1.0, 0.0))]), d);
    while n > 0 {
        if n & 1 == 1 {
            res = conv(&res, &base);
        }
        n >>= 1;
        if n > 0 {
            base = conv(&base, &base);
        }
    }
    res
}

/// One group of the pattern: nominal offset, m/z, % of the most intense group.
pub struct UnitPeak {
    pub off: i32,
    pub mz: f64,
    pub rel: f64,
}

/// Pattern of an ion given as element counts (the adduct already included) and its charge `z`; sorted by m/z.
pub fn unit_pattern(counts: &BTreeMap<String, i64>, z: i32) -> Result<Vec<UnitPeak>> {
    let mut dist = Dist::from([(0, (1.0, 0.0))]);
    for (el, &k) in counts {
        if k > 0 {
            dist = conv(&dist, &power(element_dist(el)?, k));
        }
    }
    let top = dist.values().map(|v| v.0).fold(f64::MIN, f64::max);
    let mut out: Vec<UnitPeak> = dist
        .iter()
        .map(|(&off, &(p, mp))| UnitPeak {
            off,
            mz: (mp / p - f64::from(z) * ELECTRON) / f64::from(z.abs()),
            rel: 100.0 * p / top,
        })
        .filter(|r| r.rel >= MIN_REL)
        .collect();
    crate::sortc::slice_by(&mut out, |a, b| a.mz.total_cmp(&b.mz));
    Ok(out)
}

fn add_formula(n: &mut BTreeMap<String, i64>, text: &str, times: i64) -> Result<()> {
    if !text.is_empty() {
        for (el, k) in parse_formula(text)? {
            *n.entry(el).or_insert(0) += times * k as i64;
        }
    }
    Ok(())
}

/// Element counts and charge of the ion `formula` + `adduct` (a name of the catalogue).
pub fn ion_counts(formula: &str, adduct: &str) -> Result<(BTreeMap<String, i64>, i32)> {
    let (k, add, sub, z) = composition(adduct)
        .ok_or_else(|| Error::new("err.adduct.unknown").with("adduct", adduct))?;
    let mut n = BTreeMap::new();
    add_formula(&mut n, formula, i64::from(k))?;
    add_formula(&mut n, add, 1)?;
    add_formula(&mut n, sub, -1)?;
    if n.values().any(|&k| k < 0) {
        return Err(Error::new("err.iso.adductTooMany"));
    }
    Ok((n, i32::from(z)))
}

/// `api/ion_pattern`: `{"n":{element:count},"z":charge,"peaks":[{"off","mz","rel"}]}` for a formula and an adduct name.
pub fn route(formula: &str, adduct: &str) -> Result<String> {
    let (n, z) = ion_counts(formula, adduct)?;
    let peaks = unit_pattern(&n, z)?;
    let mut o = Out::default();
    o.raw("{\"n\":{");
    for (i, (el, k)) in n.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(el).raw(":").int(*k);
    }
    o.raw("},\"z\":").int(i64::from(z)).raw(",\"peaks\":[");
    for (i, p) in peaks.iter().enumerate() {
        o.raw(if i > 0 { ",{\"off\":" } else { "{\"off\":" })
            .int(i64::from(p.off));
        o.raw(",\"mz\":")
            .num(p.mz)
            .raw(",\"rel\":")
            .num(p.rel)
            .raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peaks(f: &str, a: &str) -> Vec<(i32, f64, f64)> {
        let (n, z) = ion_counts(f, a).unwrap();
        unit_pattern(&n, z)
            .unwrap()
            .iter()
            .map(|p| (p.off, p.mz, p.rel))
            .collect()
    }

    #[test]
    fn protonated_glucose_has_m_plus_1_near_7_percent() {
        let p = peaks("C6H12O6", "[M+H]+");
        assert_eq!(p[0].0, 0);
        assert!((p[0].1 - 181.0707).abs() < 1e-3);
        assert!((p[1].2 - 6.9).abs() < 0.3, "{}", p[1].2);
    }

    #[test]
    fn chlorine_gives_a_third_of_m_plus_2() {
        let p = peaks("C6H5Cl", "[M]+");
        let m2 = p.iter().find(|r| r.0 == 2).unwrap();
        assert!((m2.2 / 100.0 - 0.32).abs() < 0.03, "{}", m2.2);
    }

    #[test]
    fn adduct_that_removes_too_much_is_refused() {
        assert_eq!(
            ion_counts("CH4", "[M-H-H2O]-").unwrap_err().key,
            "err.iso.adductTooMany"
        );
        assert_eq!(
            ion_counts("C", "[M+X]+").unwrap_err().key,
            "err.adduct.unknown"
        );
    }

    #[cfg(feature = "isotopi-tutti")]
    #[test]
    fn elements_beyond_the_formula_parser_are_in_the_table() {
        assert!(element_dist("Fe").is_ok() && element_dist("Hg").is_ok());
        assert_eq!(element_dist("Xx").unwrap_err().key, "err.iso.element");
    }
}
