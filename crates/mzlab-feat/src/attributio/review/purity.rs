// Ported from Attributio src/attributio/review/purity.py.
//! Is the MS2 spectrum of a feature chimeric, and does a match rest on real information? Every signal is evidence shown with its numbers and a reason;
//! none changes a level. A pure isolation window does not make the match right, a co-isolated ion that does not co-elute is not a chimera risk, a high
//! reverse cosine only says the reference peaks are there, and the chance probability assumes peaks spread evenly over the m/z range.

use super::explain::atom_mass;
use super::isotopes::round_to;
use super::matching::{match_spectra, Ms2};
use std::sync::OnceLock;

const C13: f64 = 1.0033548;
const POS_STEPS: [(&str, f64); 6] = [
    ("Na-H", 21.981944),
    ("NH4-H", 17.026549),
    ("K-H", 37.955882),
    ("H-Na", -21.981944),
    ("H-NH4", -17.026549),
    ("M+H-H2O", -18.010565),
];
const NEG_STEPS: [(&str, f64); 4] = [
    ("Cl+H", 35.976678),
    ("FA", 46.005479),
    ("Cl+H (rev)", -35.976678),
    ("M-H-H2O", -18.010565),
];
pub const INFORMATIVE_MZ: f64 = 100.0;
pub const INFORMATIVE_FRAC: f64 = 0.30;

fn tol_of(m: f64, ppm: f64) -> f64 {
    f64::max(0.003, m * ppm * 1e-6)
}

/// A co-isolated ion of the isolation window.
#[derive(Clone, Debug, PartialEq)]
pub struct CoIon {
    pub mz: f64,
    pub rel: f64,
    pub kind: String,
}

/// Precursor-ion fraction of the isolation window.
#[derive(Clone, Debug, PartialEq)]
pub struct IsolationPurity {
    pub purity: Option<f64>,
    pub prec_int: f64,
    pub co: Vec<CoIon>,
}

/// `float(f"{x:.4g}")`.
pub fn sig4(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    format!("{:.3e}", x).parse().unwrap_or(x)
}

/// Precursor-ion fraction of the isolation window `[target - lo, target + hi]` in an MS1 scan. The precursor's own isotopologues are those of
/// `iso_sticks` (the theoretical pattern) or, without a formula, the 13C spacing. The other ions >= `rel_list` of the precursor are classified.
#[allow(clippy::too_many_arguments)]
pub fn isolation_purity(
    mz: &[f64],
    it: &[f64],
    prec: f64,
    target: f64,
    lo: f64,
    hi: f64,
    iso_sticks: Option<&[(f64, f64)]>,
    pol: &str,
    ppm: f64,
    rel_list: f64,
) -> Option<IsolationPurity> {
    let win: Vec<bool> = mz
        .iter()
        .map(|&m| m >= target - lo && m <= target + hi)
        .collect();
    if !win.iter().any(|&w| w) {
        return None;
    }
    let near: Vec<bool> = mz
        .iter()
        .map(|&m| (m - prec).abs() <= tol_of(prec, ppm))
        .collect();
    let pint = (0..mz.len())
        .filter(|&i| near[i])
        .map(|i| it[i])
        .fold(f64::NEG_INFINITY, f64::max);
    if !pint.is_finite() {
        return None;
    }
    let own: Vec<bool> = (0..mz.len())
        .map(|i| {
            near[i]
                || iso_sticks
                    .is_some_and(|s| s.iter().any(|&(m, _)| (mz[i] - m).abs() <= tol_of(m, ppm)))
                || (1..7).any(|k| (mz[i] - (prec + k as f64 * C13)).abs() <= tol_of(prec, ppm))
        })
        .collect();
    let tot: f64 = (0..mz.len()).filter(|&i| win[i]).map(|i| it[i]).sum();
    let mine: f64 = (0..mz.len())
        .filter(|&i| win[i] && own[i])
        .map(|i| it[i])
        .sum();
    let steps: &[(&str, f64)] = match pol {
        "POS" => &POS_STEPS,
        "NEG" => &NEG_STEPS,
        _ => &[],
    };
    let mut co: Vec<CoIon> = Vec::new();
    for i in (0..mz.len()).filter(|&i| win[i] && !own[i]) {
        let m = mz[i];
        let rel = it[i] / pint;
        if rel < rel_list {
            continue;
        }
        let mut kind = "contaminant".to_string();
        for (lab, d) in steps {
            if (m - (prec + d)).abs() <= tol_of(m, ppm) {
                kind = format!("adduct:{lab}");
            }
        }
        if kind == "contaminant"
            && [1.0, 2.0]
                .iter()
                .any(|k| (m - (prec - k * C13)).abs() <= tol_of(m, ppm))
        {
            kind = "isotope (M-1/M-2: the precursor is itself an isotopologue?)".into();
        }
        co.push(CoIon {
            mz: round_to(m, 5),
            rel: round_to(rel, 3),
            kind,
        });
    }
    co.sort_by(|a, b| {
        b.rel
            .partial_cmp(&a.rel)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    co.truncate(6);
    Some(IsolationPurity {
        purity: (tot > 0.0).then(|| round_to(mine / tot, 3)),
        prec_int: sig4(pint),
        co,
    })
}

const LOSS_ELEMENTS: [(&str, i64); 8] = [
    ("C", 7),
    ("N", 4),
    ("O", 5),
    ("S", 2),
    ("P", 2),
    ("Cl", 2),
    ("F", 4),
    ("Br", 2),
];

fn loss_table() -> Vec<f64> {
    // (mass, [C, N, O, S, P, Cl, F, Br] counts)
    let mut heavy: Vec<(f64, [i64; 8])> = vec![(0.0, [0; 8])];
    for (e, (el, n_max)) in LOSS_ELEMENTS.iter().enumerate() {
        let am = atom_mass(el).unwrap_or(0.0);
        let mut new = Vec::with_capacity(heavy.len() * *n_max as usize);
        for (m, d) in &heavy {
            for n in 0..*n_max {
                let mut d2 = *d;
                d2[e] = n;
                new.push((m + n as f64 * am, d2));
            }
        }
        heavy = new;
    }
    let h_mass = atom_mass("H").unwrap_or(0.0);
    let mut out = Vec::new();
    for (m, d) in &heavy {
        let (c, n, x) = (d[0], d[1] + d[4], d[5] + d[6] + d[7]);
        let empty = d.iter().all(|&v| v == 0);
        for h in 0..=(2 * c + n + 3 - x) {
            let rdbe = c as f64 - (h + x) as f64 / 2.0 + n as f64 / 2.0 + 1.0;
            if rdbe < -0.5 || (empty && h < 1) {
                continue;
            }
            out.push(m + h as f64 * h_mass);
        }
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    out
}

/// True when a neutral loss (Da) has an elemental composition of up to C6 N3 O4 S P Cl F3 Br with RDBE >= -0.5 (radicals included) within `tol_da`.
/// Only small losses (<= 60 Da) are tested; larger ones count as plausible.
pub fn plausible_loss(loss: f64, tol_da: f64) -> bool {
    static LOSSES: OnceLock<Vec<f64>> = OnceLock::new();
    if loss > 60.0 {
        return true;
    }
    if loss < 0.9 {
        return false;
    }
    let t = LOSSES.get_or_init(loss_table);
    let j = t.partition_point(|&v| v < loss);
    [j as i64 - 1, j as i64]
        .iter()
        .any(|&k| k >= 0 && (k as usize) < t.len() && (t[k as usize] - loss).abs() <= tol_da)
}

/// A peak that cannot come from this precursor.
#[derive(Clone, Debug, PartialEq)]
pub struct Impossible {
    pub mz: f64,
    pub rel: f64,
    pub loss: f64,
    pub why: &'static str,
}

/// Peaks (>= `rel_min` of the base peak) whose loss from the precursor is not plausible (`loss`) or that lie above the precursor (`above`).
pub fn impossible_losses(
    mz: &[f64],
    it: &[f64],
    prec: f64,
    rel_min: f64,
    tol_da: f64,
    base: Option<f64>,
) -> Vec<Impossible> {
    if mz.is_empty() {
        return Vec::new();
    }
    let base = base
        .filter(|b| *b != 0.0)
        .unwrap_or_else(|| it.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
    let mut out = Vec::new();
    for (&m, &i) in mz.iter().zip(it) {
        if i < rel_min * base {
            continue;
        }
        let loss = prec - m;
        let why = if loss < -0.5 {
            "above"
        } else if loss > 0.5 && !plausible_loss(loss, tol_da) {
            "loss"
        } else {
            continue;
        };
        out.push(Impossible {
            mz: round_to(m, 5),
            rel: round_to(i / base, 3),
            loss: round_to(loss, 4),
            why,
        });
    }
    out
}

/// Share of the MS2 signal (sqrt intensities) in peaks explained neither by the formula nor by the matched reference.
pub fn unexplained(it: &[f64], explained: &[bool], matched: Option<&[bool]>) -> f64 {
    let w: Vec<f64> = it.iter().map(|v| v.sqrt()).collect();
    let tot: f64 = w.iter().sum();
    if w.is_empty() || tot <= 0.0 {
        return 0.0;
    }
    let miss: f64 = (0..w.len())
        .filter(|&i| {
            !(explained.get(i).copied().unwrap_or(false)
                || matched.is_some_and(|m| m.get(i).copied().unwrap_or(false)))
        })
        .map(|i| w[i])
        .sum();
    miss / tot
}

/// P(>= `matched` chance matches) when `n_ref` reference peaks fall at random over `span` Da and each matches if an experimental peak lies within +- `tol`.
pub fn chance_probability(n_exp: i64, n_ref: i64, matched: i64, tol: f64, span: f64) -> f64 {
    if matched <= 0 || n_exp <= 0 || n_ref <= 0 || span <= 0.0 {
        return 1.0;
    }
    let p1 = f64::min(1.0, n_exp as f64 * 2.0 * tol / span);
    let lam = n_ref as f64 * p1;
    let mut cdf = 0.0;
    let mut fact = 1.0;
    for k in 0..matched {
        if k > 0 {
            fact *= k as f64;
        }
        cdf += (-lam).exp() * lam.powi(k as i32) / fact;
    }
    f64::max(0.0, 1.0 - cdf)
}

/// The agreement of a candidate's reference with the experimental spectrum.
#[derive(Clone, Debug, PartialEq)]
pub struct CandidateScores {
    pub fwd: f64,
    pub rev: f64,
    pub presence: f64,
    pub n_main: i64,
    pub cos_expl: f64,
    pub matched_expl: i64,
    pub informative: i64,
    pub p_chance: f64,
    pub matched: i64,
}

fn max_where(v: &[f64], keep: &[bool]) -> f64 {
    (0..v.len())
        .filter(|&i| keep[i])
        .map(|i| v[i])
        .fold(0.0, f64::max)
}

/// Forward / reverse cosine and presence (on the reference peaks the scan can acquire), cosine and matches on the explained peaks only, informative matched
/// ions and the chance probability. `exp` and `reference` are spectra with their precursor.
pub fn candidate_scores(
    exp: &Ms2,
    reference: &Ms2,
    tol: f64,
    low: Option<f64>,
    shift: f64,
    explained: &[bool],
) -> CandidateScores {
    let m = match_spectra(exp, reference, tol, shift, low);
    let low = low.filter(|l| *l != 0.0);
    let keep: Vec<bool> = reference
        .mz
        .iter()
        .map(|&r| r < reference.prec - 0.5 && low.is_none_or(|l| r >= l))
        .collect();
    let rmax = max_where(&reference.it, &keep);
    let main: Vec<usize> = (0..keep.len())
        .filter(|&j| keep[j] && reference.it[j] >= 0.10 * rmax)
        .collect();
    let matched_ref: std::collections::HashSet<usize> = m.pairs.iter().map(|p| p.1).collect();
    let presence = if main.is_empty() {
        0.0
    } else {
        main.iter().filter(|j| matched_ref.contains(j)).count() as f64 / main.len() as f64
    };
    let (cos_ex, n_ex) = if explained.iter().any(|&e| e) {
        let sel: Vec<usize> = (0..exp.mz.len())
            .filter(|&i| explained.get(i).copied().unwrap_or(false))
            .collect();
        let sub = Ms2 {
            mz: sel.iter().map(|&i| exp.mz[i]).collect(),
            it: sel.iter().map(|&i| exp.it[i]).collect(),
            prec: exp.prec,
        };
        let me = match_spectra(&sub, reference, tol, shift, low);
        (me.cosine, me.matched as i64)
    } else {
        (0.0, 0)
    };
    let thr = f64::max(INFORMATIVE_MZ, INFORMATIVE_FRAC * exp.prec);
    let informative = m.pairs.iter().filter(|p| exp.mz[p.0] >= thr).count() as i64;
    let below: Vec<bool> = exp.mz.iter().map(|&x| x < exp.prec - 0.5).collect();
    let lo_ = low.unwrap_or_else(|| {
        let v = (0..exp.mz.len())
            .filter(|&i| below[i])
            .map(|i| exp.mz[i])
            .fold(f64::INFINITY, f64::min);
        if v.is_finite() {
            v
        } else {
            0.0
        }
    });
    let span = f64::max(1.0, exp.prec - lo_);
    let n_e = below.iter().filter(|&&b| b).count() as i64;
    let n_r = (0..keep.len())
        .filter(|&j| keep[j] && reference.it[j] >= 0.01 * rmax)
        .count() as i64;
    CandidateScores {
        fwd: round_to(m.cosine, 3),
        rev: round_to(m.reverse, 3),
        presence: round_to(presence, 2),
        n_main: main.len() as i64,
        cos_expl: round_to(cos_ex, 3),
        matched_expl: n_ex,
        informative,
        p_chance: round_to(chance_probability(n_e, n_r, m.matched as i64, tol, span), 4),
        matched: m.matched as i64,
    }
}

/// The co-elution of a co-isolated ion, as `badge` reads it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CoelRow {
    pub mz: f64,
    pub rel: f64,
    pub r: Option<f64>,
    pub offset: Option<f64>,
}

/// clean / possible / likely chimeric, with one reason per signal that fired.
#[derive(Clone, Debug, PartialEq)]
pub struct Badge {
    pub state: &'static str,
    pub reasons: Vec<String>,
}

pub fn badge(
    iso: Option<&IsolationPurity>,
    coel: &[CoelRow],
    unexpl: Option<f64>,
    imposs: &[Impossible],
    best: Option<&CandidateScores>,
) -> Badge {
    let (mut possible, mut likely): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    if let Some(p) = iso.and_then(|i| i.purity) {
        if p < 0.5 {
            likely.push(format!(
                "precursor-ion fraction of the isolation window {p:.2} (< 0.50)"
            ));
        } else if p < 0.8 {
            possible.push(format!(
                "precursor-ion fraction of the isolation window {p:.2} (< 0.80)"
            ));
        }
    }
    for c in coel {
        if c.r.is_some_and(|r| r >= 0.8) && c.offset.unwrap_or(1.0).abs() <= 0.1 {
            let s = format!(
                "co-isolated ion {:.4} ({:.0} %) co-elutes (r {:.2}, apex offset {:+.2} min)",
                c.mz,
                100.0 * c.rel,
                c.r.unwrap(),
                c.offset.unwrap_or(1.0)
            );
            if c.rel >= 0.5 {
                likely.push(s)
            } else {
                possible.push(s)
            }
        }
    }
    if let Some(u) = unexpl.filter(|u| *u > 0.5) {
        possible.push(format!(
            "{:.0} % of the MS2 signal is explained neither by the formula nor by the reference",
            100.0 * u
        ));
    }
    let big: Vec<&Impossible> = imposs.iter().filter(|x| x.rel >= 0.10).collect();
    if !big.is_empty() {
        let list: Vec<String> = big
            .iter()
            .take(3)
            .map(|x| format!("{:.4}: loss {:.4}", x.mz, x.loss))
            .collect();
        possible.push(format!(
            "{} peak(s) ≥ 10 % cannot come from this precursor ({})",
            big.len(),
            list.join(", ")
        ));
    }
    if let Some(b) = best.filter(|b| b.rev - b.fwd >= 0.30) {
        possible.push(format!(
            "reverse cosine {:.2} >> forward {:.2}: the reference peaks are there, the rest of the spectrum is something else",
            b.rev, b.fwd
        ));
    }
    let state = if !likely.is_empty() || possible.len() >= 2 {
        "likely chimeric"
    } else if !possible.is_empty() {
        "possible"
    } else {
        "clean"
    };
    likely.extend(possible);
    Badge {
        state,
        reasons: likely,
    }
}

/// Does the best match rest on enough informative ions?
#[derive(Clone, Debug, PartialEq)]
pub struct LowInformation {
    pub low: bool,
    pub why: String,
}

pub fn low_information(
    best: Option<&CandidateScores>,
    min_informative: i64,
) -> Option<LowInformation> {
    let b = best?;
    Some(LowInformation {
        low: b.informative < min_informative,
        why: format!(
            "{} informative matched ion(s) (m/z ≥ {:.0} or ≥ {:.0} % of the precursor), chance probability {:.3}",
            b.informative,
            INFORMATIVE_MZ,
            100.0 * INFORMATIVE_FRAC,
            b.p_chance
        ),
    })
}
