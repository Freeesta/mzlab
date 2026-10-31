//! Adduct catalogue of the m/z calculator: shifts, ion m/z, ppm against an observed m/z, the neutral mass each adduct implies and the
//! mass differences between adducts. The one source of these numbers for the page (`api/adduct_table`) and for the search box.

use crate::elements::{fmt, mass, parse_formula, round_half_up, ELECTRON};
use crate::error::Result;
use crate::json::{round, Out};

/// `id` is the suffix of the page's short note key (`tbl.add.note.<id>`); `common` rows are shown before the fold.
struct Def {
    name: &'static str,
    id: &'static str,
    k: u8,
    add: &'static str,
    sub: &'static str,
    z: i8,
    common: bool,
}

const fn d(
    name: &'static str,
    id: &'static str,
    k: u8,
    add: &'static str,
    sub: &'static str,
    z: i8,
    common: bool,
) -> Def {
    Def {
        name,
        id,
        k,
        add,
        sub,
        z,
        common,
    }
}

/// Most frequent first, per polarity.
const CATALOG: [Def; 20] = [
    d("[M+H]+", "mh", 1, "H", "", 1, true),
    d("[M+NH4]+", "mnh4", 1, "NH4", "", 1, true),
    d("[M+Na]+", "mna", 1, "Na", "", 1, true),
    d("[M+K]+", "mk", 1, "K", "", 1, true),
    d("[M+H-H2O]+", "mhh2o", 1, "H", "H2O", 1, false),
    d("[2M+H]+", "2mh", 2, "H", "", 1, false),
    d("[2M+Na]+", "2mna", 2, "Na", "", 1, false),
    d("[M+H+ACN]+", "mhacn", 1, "C2H4N", "", 1, false),
    d("[M+2H]2+", "m2h", 1, "H2", "", 2, false),
    d("[M+CH3OH+H]+", "mch3oh", 1, "CH5O", "", 1, false),
    d("[M]+", "mcat", 1, "", "", 1, false),
    d("[M-H]-", "mminush", 1, "", "H", -1, true),
    d("[M+HCOO]-", "mhcoo", 1, "CHO2", "", -1, true),
    d("[M+CH3COO]-", "mch3coo", 1, "C2H3O2", "", -1, true),
    d("[M+Cl]-", "mcl", 1, "Cl", "", -1, true),
    d("[M-H-H2O]-", "mmhh2o", 1, "", "H3O", -1, false),
    d("[2M-H]-", "2mminush", 2, "", "H", -1, false),
    d("[M-2H]2-", "mm2h", 1, "", "H2", -2, false),
    d("[M+Na-2H]-", "mna2h", 1, "Na", "H2", -1, false),
    d("[M]-", "man", 1, "", "", -1, false),
];

/// Composition of a catalogue adduct: molecules `k` of M, the formula added, the formula removed (both may be empty) and the charge.
pub fn composition(name: &str) -> Option<(u8, &'static str, &'static str, i8)> {
    CATALOG
        .iter()
        .find(|a| a.name == name)
        .map(|a| (a.k, a.add, a.sub, a.z))
}

/// Pairs worth knowing by heart (same compound, same RT): label, adduct, reference adduct.
const PAIRS: [(&str, &str, &str); 7] = [
    ("Na − H", "[M+Na]+", "[M+H]+"),
    ("NH4 − H", "[M+NH4]+", "[M+H]+"),
    ("K − H", "[M+K]+", "[M+H]+"),
    ("ACN", "[M+H+ACN]+", "[M+H]+"),
    ("HCOO", "[M+HCOO]-", "[M-H]-"),
    ("CH3COO", "[M+CH3COO]-", "[M-H]-"),
    ("Cl", "[M+Cl]-", "[M-H]-"),
];

/// Reference m/z of the tolerance when two peaks give only a difference and no observed m/z is typed.
const DELTA_REF_MZ: f64 = 400.0;

fn part(f: &str) -> Result<f64> {
    if f.is_empty() {
        Ok(0.0)
    } else {
        Ok(mass(&parse_formula(f)?))
    }
}

/// Mass added to `k` neutral molecules to make the ion: what is added, minus what is lost, minus the electrons (plus, for anions).
fn shift(a: &Def) -> Result<f64> {
    Ok(part(a.add)? - part(a.sub)? - f64::from(a.z) * ELECTRON)
}

/// A typed number (comma or point); anything unreadable counts as not given, the page checks what the student typed.
fn number(t: &str) -> Option<f64> {
    t.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

/// `api/adduct_table`: the rows of every adduct (shift, and with `f` or `obs` the m/z, ppm and implied neutral mass), the characteristic
/// differences and, with `delta`, the pairs of adducts that explain it. Empty text = not given; `tol` is in ppm (default 5).
pub fn table(f: &str, obs: &str, tol: &str, delta: &str) -> Result<String> {
    let parsed = if f.trim().is_empty() {
        None
    } else {
        Some(parse_formula(f)?)
    };
    let m = parsed.as_ref().map(mass);
    let (obs, tol, delta) = (number(obs), number(tol).unwrap_or(5.0).abs(), number(delta));
    let shifts = CATALOG.iter().map(shift).collect::<Result<Vec<_>>>()?;
    let mut o = Out::default();
    o.raw("{\"formula\":");
    match &parsed {
        Some(p) => o.str(&fmt(p)),
        None => o.raw("null"),
    };
    o.raw(",\"neutral\":")
        .opt(m.map(|x| round(x, 4)))
        .raw(",\"tol\":")
        .num(tol)
        .raw(",\"rows\":[");
    let best = rows_json(&mut o, m, obs, tol, &shifts);
    o.raw("],\"best\":")
        .opt(best.map(|i| i as f64))
        .raw(",\"diffs\":[");
    diffs_json(&mut o, &shifts);
    o.raw("]");
    if let Some(dv) = delta {
        delta_json(&mut o, dv.abs(), obs, tol, &shifts);
    }
    o.raw("}");
    Ok(o.0)
}

/// The body of `rows`; returns the index of the row nearest to `obs` within `tol` ppm.
fn rows_json(
    o: &mut Out,
    m: Option<f64>,
    obs: Option<f64>,
    tol: f64,
    shifts: &[f64],
) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, (a, &s)) in CATALOG.iter().zip(shifts).enumerate() {
        let (k, z) = (f64::from(a.k), f64::from(a.z.abs()));
        let mz = m.map(|x| (k * x + s) / z);
        let ppm = mz.zip(obs).map(|(c, v)| (v - c) / c * 1e6);
        if let Some(p) = ppm.filter(|p| p.abs() <= tol) {
            if best.is_none_or(|(_, b)| p.abs() < b.abs()) {
                best = Some((i, p));
            }
        }
        o.raw(if i > 0 { ",{" } else { "{" })
            .raw("\"n\":")
            .str(a.name)
            .raw(",\"id\":")
            .str(a.id);
        o.raw(",\"pol\":")
            .int(i64::from(a.z.signum()))
            .raw(",\"common\":")
            .raw(if a.common { "true" } else { "false" });
        o.raw(",\"k\":")
            .int(i64::from(a.k))
            .raw(",\"z\":")
            .int(i64::from(a.z));
        o.raw(",\"add\":")
            .str(a.add)
            .raw(",\"sub\":")
            .str(a.sub)
            .raw(",\"shift\":")
            .num(round(s, 6));
        o.raw(",\"mz4\":")
            .opt(mz.map(|c| round(c, 4)))
            .raw(",\"mz5\":")
            .opt(mz.map(|c| round(c, 5)));
        o.raw(",\"mz1\":")
            .opt(mz.map(|c| round_half_up(c, 1)))
            .raw(",\"nominal\":")
            .opt(mz.map(|c| round_half_up(c, 0)));
        o.raw(",\"ppm\":").opt(ppm.map(|p| round(p, 2)));
        o.raw(",\"m_if\":")
            .opt(obs.map(|v| round((v * z - s) / k, 4)))
            .raw("}");
    }
    best.map(|(i, _)| i)
}

/// The body of `diffs`: the characteristic differences between adducts.
fn diffs_json(o: &mut Out, shifts: &[f64]) {
    let sh = |n: &str| {
        CATALOG
            .iter()
            .position(|x| x.name == n)
            .map(|i| shifts[i])
            .unwrap_or(0.0)
    };
    for (j, (label, a, b)) in PAIRS.iter().enumerate() {
        o.raw(if j > 0 { ",{" } else { "{" })
            .raw("\"pair\":")
            .str(label)
            .raw(",\"a\":")
            .str(a)
            .raw(",\"b\":")
            .str(b);
        o.raw(",\"pol\":")
            .int(if a.ends_with('+') { 1 } else { -1 })
            .raw(",\"d\":")
            .num(round(sh(a) - sh(b), 6))
            .raw("}");
    }
}

/// The `delta` object: the adduct pairs that explain the difference `dv` between two peaks.
fn delta_json(o: &mut Out, dv: f64, obs: Option<f64>, tol: f64, shifts: &[f64]) {
    o.raw(",\"delta\":{\"value\":").num(dv).raw(",\"hits\":[");
    for (j, (a, b, e)) in delta_hits(dv, obs.unwrap_or(DELTA_REF_MZ), tol, shifts)
        .iter()
        .enumerate()
    {
        o.raw(if j > 0 { ",{" } else { "{" })
            .raw("\"a\":")
            .str(CATALOG[*a].name)
            .raw(",\"b\":")
            .str(CATALOG[*b].name);
        o.raw(",\"d\":")
            .num(round(shifts[*a] - shifts[*b], 6))
            .raw(",\"err_mda\":")
            .num(round(e * 1000.0, 2))
            .raw("}");
    }
    o.raw("]}");
}

/// Pairs of singly charged adducts of one polarity whose shifts differ by `delta` within `tol` ppm of `ref_mz`; nearest first.
fn delta_hits(delta: f64, ref_mz: f64, tol: f64, shifts: &[f64]) -> Vec<(usize, usize, f64)> {
    let win = tol * 1e-6 * ref_mz;
    let single: Vec<usize> = (0..CATALOG.len())
        .filter(|&i| CATALOG[i].k == 1 && CATALOG[i].z.abs() == 1)
        .collect();
    let mut hits = Vec::new();
    for &a in &single {
        for &b in &single {
            if a == b || CATALOG[a].z != CATALOG[b].z || shifts[a] <= shifts[b] {
                continue;
            }
            let e = (shifts[a] - shifts[b]) - delta;
            if e.abs() <= win {
                hits.push((a, b, e));
            }
        }
    }
    crate::sortc::slice_by(&mut hits, |x, y| x.2.abs().total_cmp(&y.2.abs()));
    hits
}
