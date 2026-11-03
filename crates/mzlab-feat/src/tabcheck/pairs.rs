//! The checks of `tabcheck/1` that look for pairs of co-eluting rows: duplicates, adducts, isotopes and in-source fragments. A pair is a difference of m/z
//! that fits, at the same RT, with areas that move together across the files; it is a candidate, never an identification.

use super::*;

/// Differences of m/z (Da) between an adduct and the protonated molecule in ESI+: [M+Na]+ - [M+H]+ = Na - H, [M+NH4]+ - [M+H]+ = NH3, [M+K]+ - [M+H]+ = K - H.
const ADDUCTS_POS: [(&str, f64); 3] = [
    ("[M+Na]+", 21.98194),
    ("[M+NH4]+", 17.02655),
    ("[M+K]+", 37.95588),
];
/// In ESI-: [M+Cl]- - [M-H]- = HCl, [M+HCOO]- - [M-H]- = HCOOH.
const ADDUCTS_NEG: [(&str, f64); 2] = [("[M+Cl]-", 35.97668), ("[M+HCOO]-", 46.00548)];
/// 13C - 12C, Da (HR); at unit resolution the step is one Da.
const ISO_STEP_HR: f64 = 1.00336;
const ISO_STEP_UNIT: f64 = 1.0;
/// M+1/M of one carbon (natural abundance of 13C), and the carbons per Da of an organic molecule used to expect the ratio (60 % C by mass / 12).
const ISO_PER_CARBON: f64 = 0.0107;
const CARBON_PER_DA: f64 = 0.05;
/// The observed M+1/M is read against the expected one inside this interval of it.
const ISO_BAND: (f64, f64) = (0.5, 2.0);
/// Correlation of the areas over the files from which an adduct, respectively an in-source fragment, is proposed.
const ADDUCT_R: f64 = 0.9;
const INSOURCE_R: f64 = 0.95;
/// CV of the ratio of the areas over the files up to which the ratio counts as constant.
const INSOURCE_RATIO_CV: f64 = 0.2;
/// Files a correlation needs, and the smallest drop of m/z of a fragment (below it is an isotope or an adduct).
const MIN_FILES_ADDUCT: usize = 3;
const MIN_FILES_INSOURCE: usize = 5;
const FRAGMENT_MIN_DA: f64 = 3.0;
/// Rows (the most intense) the in-source search compares with each other.
const MAX_CORR_ROWS: usize = 2000;

#[derive(Debug, Clone, Copy)]
struct Pair {
    lo: usize,
    hi: usize,
    tag: &'static str,
    /// Correlation (adducts, fragments) or the observed over the expected ratio (isotopes).
    x: f64,
}

fn adducts_of(e: &Env) -> &'static [(&'static str, f64)] {
    if e.opts.polarity < 0 {
        &ADDUCTS_NEG
    } else {
        &ADDUCTS_POS
    }
}

fn iso_step(e: &Env) -> f64 {
    if e.unit_resolution() {
        ISO_STEP_UNIT
    } else {
        ISO_STEP_HR
    }
}

/// Rows of `by_mz` (sorted by m/z) with an m/z within `tol` of `target`.
fn window<'a>(e: &Env, by_mz: &'a [usize], target: f64, tol: f64) -> &'a [usize] {
    let from = by_mz.partition_point(|&r| e.t.mz[r] < target - tol);
    let to = by_mz.partition_point(|&r| e.t.mz[r] <= target + tol);
    &by_mz[from..to.max(from)]
}

fn corr(e: &Env, a: usize, b: usize) -> Option<f64> {
    pearson(&e.areas(a, &e.live), &e.areas(b, &e.live))
}

/// Ids of the rows of `pairs` (the heavier first), once each, within the example cap.
fn pair_rows(e: &Env, pairs: &[Pair]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for p in pairs {
        for r in [p.hi, p.lo] {
            if out.len() < e.caps.rows && !out.contains(&e.id(r)) {
                out.push(e.id(r));
            }
        }
    }
    out
}

fn pair_list(e: &Env, pairs: &[Pair], key: &str) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, p) in pairs.iter().take(e.caps.pairs).enumerate() {
        o.raw(if i > 0 { ",{\"lo\":" } else { "{\"lo\":" })
            .int(i64::from(e.id(p.lo)))
            .raw(",\"hi\":")
            .int(i64::from(e.id(p.hi)));
        o.raw(",\"dmz\":")
            .num(round(e.t.mz[p.hi] - e.t.mz[p.lo], 5))
            .raw(",\"drt\":")
            .num(round(e.t.rt[p.hi] - e.t.rt[p.lo], 4));
        o.raw(",\"kind\":")
            .str(p.tag)
            .raw(",\"")
            .raw(key)
            .raw("\":")
            .num(round(p.x, 4))
            .raw("}");
    }
    o.raw("]");
    o.0
}

fn sorted(mut v: Vec<Pair>) -> Vec<Pair> {
    crate::sortc::by(&mut v, |a, b| {
        a.lo.cmp(&b.lo).then(a.hi.cmp(&b.hi)).then(a.tag.cmp(b.tag))
    });
    v
}

/// Rows within the m/z tolerance of the alignment and the RT window of one peak: the same signal split in two.
pub(super) fn duplicates(e: &Env) -> Check {
    let by = e.by_mz();
    let mut found = Vec::new();
    for (i, &a) in by.iter().enumerate() {
        for &b in by[i + 1..]
            .iter()
            .take_while(|&&b| e.t.mz[b] - e.t.mz[a] <= e.opts.mz_tol_da)
        {
            if e.rt_close(a, b) {
                found.push(Pair {
                    lo: a,
                    hi: b,
                    tag: "duplicate",
                    x: 0.0,
                });
            }
        }
    }
    let found = sorted(found);
    let mut p = Proof::new();
    p.n("mz_tol_da", e.opts.mz_tol_da)
        .n("rt_window_min", e.fwhm)
        .i("pairs", found.len())
        .raw("list", &pair_list(e, &found, "x"));
    Check {
        id: "duplicates",
        value: Some(found.len() as f64),
        threshold: Some(1.0),
        flag: if found.is_empty() {
            Flag::Ok
        } else {
            Flag::Warn
        },
        rows: pair_rows(e, &found),
        files: Vec::new(),
        proof: p.done(),
    }
}

/// Pairs that co-elute with a difference of m/z of a known adduct and areas that correlate over the files.
fn adduct_pairs(e: &Env, by: &[usize]) -> Vec<Pair> {
    let mut found = Vec::new();
    for &a in by {
        for &(name, d) in adducts_of(e) {
            for &b in window(e, by, e.t.mz[a] + d, e.opts.mz_tol_da) {
                if e.rt_close(a, b)
                    && let Some(r) = corr(e, a, b).filter(|r| *r >= ADDUCT_R)
                {
                    found.push(Pair {
                        lo: a,
                        hi: b,
                        tag: name,
                        x: r,
                    });
                }
            }
        }
    }
    sorted(found)
}

pub(super) fn adducts(e: &Env) -> Check {
    if e.live.len() < MIN_FILES_ADDUCT {
        return Check::missing("adducts", "files");
    }
    let found = adduct_pairs(e, &e.by_mz());
    let mut p = Proof::new();
    p.s(
        "mode",
        if e.opts.polarity < 0 {
            "negative"
        } else {
            "positive"
        },
    )
    .n("mz_tol_da", e.opts.mz_tol_da)
    .i("pairs", found.len())
    .raw("list", &pair_list(e, &found, "r"));
    Check {
        id: "adducts",
        value: Some(found.len() as f64),
        threshold: Some(ADDUCT_R),
        flag: if found.is_empty() {
            Flag::Ok
        } else {
            Flag::Note
        },
        rows: pair_rows(e, &found),
        files: Vec::new(),
        proof: p.done(),
    }
}

/// Pairs one 13C step apart that co-elute, with the observed M+1/M (median over the files that have both) against the one expected for the m/z.
pub(super) fn isotopes(e: &Env) -> Check {
    let by = e.by_mz();
    let (mut all, mut outside) = (Vec::new(), Vec::new());
    for &a in &by {
        for &b in window(e, &by, e.t.mz[a] + iso_step(e), e.opts.mz_tol_da) {
            if !e.rt_close(a, b) {
                continue;
            }
            let ratios: Vec<f64> = e
                .live
                .iter()
                .filter_map(|&k| Some(e.area(b, k)? / e.area(a, k)?))
                .collect();
            if ratios.is_empty() {
                continue;
            }
            let expected = ISO_PER_CARBON * (e.t.mz[a] * CARBON_PER_DA).round().max(1.0);
            let x = median(ratios) / expected;
            let pair = Pair {
                lo: a,
                hi: b,
                tag: "M+1",
                x,
            };
            all.push(pair);
            if x < ISO_BAND.0 || x > ISO_BAND.1 {
                outside.push(pair);
            }
        }
    }
    let (all, outside) = (sorted(all), sorted(outside));
    let mut p = Proof::new();
    p.n("step_da", iso_step(e))
        .i("pairs", all.len())
        .n("band_low", ISO_BAND.0)
        .n("band_high", ISO_BAND.1);
    p.raw("outside", &pair_list(e, &outside, "obs_over_expected"));
    let shown = if outside.is_empty() { &all } else { &outside };
    Check {
        id: "isotopes",
        value: Some(outside.len() as f64),
        threshold: Some(0.0),
        flag: if outside.is_empty() {
            Flag::Ok
        } else {
            Flag::Note
        },
        rows: pair_rows(e, shown),
        files: Vec::new(),
        proof: p.done(),
    }
}

/// Pairs of rows (sorted by RT) that co-elute, differ by a fragment-sized step and keep a constant area ratio.
fn coeluting_fragments(e: &Env, rows: &[usize]) -> Vec<Pair> {
    let known = |d: f64| {
        adducts_of(e)
            .iter()
            .any(|x| (d - x.1).abs() <= e.opts.mz_tol_da)
            || (d - iso_step(e)).abs() <= e.opts.mz_tol_da
    };
    let mut found = Vec::new();
    for (i, &a) in rows.iter().enumerate() {
        for &b in rows[i + 1..]
            .iter()
            .take_while(|&&b| e.t.rt[b] - e.t.rt[a] <= e.fwhm)
        {
            let (lo, hi) = if e.t.mz[a] < e.t.mz[b] {
                (a, b)
            } else {
                (b, a)
            };
            let d = e.t.mz[hi] - e.t.mz[lo];
            if d < FRAGMENT_MIN_DA || known(d) {
                continue;
            }
            let Some(r) = corr(e, lo, hi).filter(|r| *r >= INSOURCE_R) else {
                continue;
            };
            let ratios: Vec<f64> = e
                .live
                .iter()
                .filter_map(|&k| Some(e.area(lo, k)? / e.area(hi, k)?))
                .collect();
            if ratios.len() >= MIN_FILES_INSOURCE && cv(&ratios) <= INSOURCE_RATIO_CV {
                found.push(Pair {
                    lo,
                    hi,
                    tag: "fragment",
                    x: r,
                });
            }
        }
    }
    found
}

/// JSON list of the parents (heavier rows) with the ids of their fragments, cut to the example caps.
fn parents_list(e: &Env, found: &[Pair], parents: &[usize]) -> String {
    let mut list = Out::default();
    list.raw("[");
    for (i, &h) in parents.iter().take(e.caps.pairs).enumerate() {
        list.raw(if i > 0 {
            ",{\"parent\":"
        } else {
            "{\"parent\":"
        })
        .int(i64::from(e.id(h)))
        .raw(",\"fragments\":[");
        for (j, p) in found
            .iter()
            .filter(|p| p.hi == h)
            .take(e.caps.rows)
            .enumerate()
        {
            list.raw(if j > 0 { "," } else { "" })
                .int(i64::from(e.id(p.lo)));
        }
        list.raw("]}");
    }
    list.raw("]");
    list.0
}

/// Co-eluting rows whose areas keep a constant ratio over the files, the lighter one a fragment of the heavier; adduct and isotope differences are left out.
pub(super) fn insource(e: &Env) -> Check {
    if e.live.len() < MIN_FILES_INSOURCE {
        return Check::missing("insource", "files");
    }
    let mut cand: Vec<(usize, f64)> = e
        .rows
        .iter()
        .map(|&r| (r, e.areas(r, &e.live).into_iter().fold(0.0, f64::max)))
        .filter(|x| {
            x.1 > 0.0
                && e.live.iter().filter(|&&k| e.area(x.0, k).is_some()).count()
                    >= MIN_FILES_INSOURCE
        })
        .collect();
    let total = cand.len();
    rank(&mut cand, true);
    cand.truncate(MAX_CORR_ROWS);
    let mut rows: Vec<usize> = cand.iter().map(|x| x.0).collect();
    crate::sortc::by(&mut rows, |&a, &b| {
        e.t.rt[a].total_cmp(&e.t.rt[b]).then(a.cmp(&b))
    });
    let found = sorted(coeluting_fragments(e, &rows));
    let mut parents: Vec<usize> = found.iter().map(|p| p.hi).collect();
    crate::sortc::by(&mut parents, |a, b| a.cmp(b));
    parents.dedup();
    let list = parents_list(e, &found, &parents);
    let mut p = Proof::new();
    p.i("rows_compared", cand.len())
        .i("rows_eligible", total)
        .i("pairs", found.len())
        .i("parents", parents.len())
        .n("ratio_cv_max", INSOURCE_RATIO_CV)
        .raw("list", &list);
    Check {
        id: "insource",
        value: Some(found.len() as f64),
        threshold: Some(INSOURCE_R),
        flag: if found.is_empty() {
            Flag::Ok
        } else {
            Flag::Note
        },
        rows: pair_rows(e, &found),
        files: Vec::new(),
        proof: p.done(),
    }
}
