//! The JSON pieces of the review of one feature: the RT-logP check, the proofs of the checklist, the alternatives, the decisions already written and the libraries.

use super::alternatives::{same_formula, Alt, AltSources};
use super::banco::*;
use super::libraries::LibraryIndex;
use super::matching::Ms2;
use super::proposal::{Proposal, J};
use super::propose::MgfBlock;
use super::rtlogp::{calibrate, check, organic_fraction, CalPoint, LogPValues, MIN_N};
use super::rules::{page_level, Current};
use super::sources::{csi_top5, SourceTexts, StructureCandidate};
use super::structures::GraphStructures;
use crate::attributio::annotate::Feature;
use crate::attributio::readers::feature_decisions::DecisionTable;

pub(super) fn num_or_null(v: Option<f64>) -> J {
    J::on(v)
}

/// The retention-time versus logP check of the feature, calibrated on the Level 1 and robust 2a features of its polarity that the sources give a logP for.
pub(super) fn rt_logp(b: &Bench, f: &Feature) -> Option<(J, Option<bool>)> {
    let s = &b.settings;
    let grad = &s.lc_gradient;
    let floor = if s.rt_logp_floor > 0.0 {
        s.rt_logp_floor
    } else if grad.is_empty() {
        1.0
    } else {
        3.0
    };
    let y_of = |rt: f64| {
        if grad.is_empty() {
            rt
        } else {
            organic_fraction(rt, grad, s.lc_dwell_min)
        }
    };
    let values = |g: &Feature| LogPValues {
        xlogp3: g.csi.xlogp.filter(|v| v.is_finite()),
        ..Default::default()
    };
    let pts: Vec<CalPoint> = b
        .features
        .iter()
        .filter(|g| g.pol == f.pol && (g.level == 1 || (g.level == 2 && g.level_sub == "a")))
        .filter_map(|g| {
            Some(CalPoint {
                logp: values(g),
                y: Some(y_of(g.rt?)),
            })
        })
        .collect();
    let model = calibrate(&pts, s.rt_logp_k, floor, MIN_N);
    let y = f.rt.map(y_of);
    let c = check(&model, &values(f), y, s.rt_logp_k, floor);
    let j = J::Obj(vec![
        ("n_calibrants", J::Num(model.n as f64)),
        ("y", num_or_null(y)),
        (
            "y_label",
            J::Str(
                if grad.is_empty() {
                    "RT (min)"
                } else {
                    "organic fraction at elution (%B)"
                }
                .into(),
            ),
        ),
        ("logp", num_or_null(c.logp)),
        ("pred", num_or_null(c.pred)),
        ("resid", num_or_null(c.resid)),
        ("window", num_or_null(model.window)),
        ("ok", c.ok.map_or(J::Null, J::Bool)),
        ("note", J::Str(model.note.clone())),
    ]);
    Some((j, c.ok))
}

/// The proof the program calculated for each box of the checklist (`None`: not available); the chemist ticks the boxes, the program never does.
pub(super) fn proofs(p: &Proposal, alts: &[Alt], rt: Option<bool>) -> Vec<Option<String>> {
    let e = &p.evidence;
    let view = e.view.as_ref();
    let frags: Vec<_> = view
        .map(|v| v.peaks.iter().filter_map(|k| k.frag.as_ref()).collect())
        .unwrap_or_default();
    let losses: Vec<&str> = frags
        .iter()
        .map(|f| f.loss.as_str())
        .filter(|l| !l.is_empty())
        .collect();
    let iso = e
        .raw
        .as_ref()
        .and_then(|r| r.ms1.as_ref())
        .and_then(|m| m.iso_ok);
    let pur = e
        .purity
        .as_ref()
        .and_then(|u| u.purity.map(|v| (v, u.badge.clone())));
    vec![
        (e.n_peaks > 0).then(|| {
            format!(
                "{} of {} peaks explained ({:.0}% of the intensity)",
                e.n_explained,
                e.n_peaks,
                e.frac_explained * 100.0
            )
        }),
        (!losses.is_empty()).then(|| format!("losses: {}", losses.join(", "))),
        iso.map(|ok| {
            format!(
                "isotope pattern {}",
                if ok { "consistent" } else { "not consistent" }
            )
        }),
        pur.map(|(v, b)| format!("isolation purity {v:.2} ({b})")),
        rt.map(|ok| {
            format!(
                "RT versus logP: {}",
                if ok {
                    "within the window"
                } else {
                    "outside the window"
                }
            )
        }),
        Some(format!(
            "{} alternative(s) with the same formula listed",
            alts.len()
        )),
    ]
}

pub(super) fn alt_json(a: &Alt, best_ik1: Option<&str>) -> J {
    J::Obj(vec![
        ("name", J::s(&a.name)),
        ("source", J::s(&a.source)),
        ("entry", J::os(&a.entry)),
        ("ik1", J::os(&a.ik1)),
        ("smiles", J::os(&a.smiles)),
        ("score", J::on(a.score)),
        ("matched", a.matched.map_or(J::Null, |m| J::Num(m as f64))),
        ("logp", J::on(a.logp)),
        (
            "is_best",
            J::Bool(a.ik1.is_some() && a.ik1.as_deref() == best_ik1),
        ),
    ])
}

/// The last decision row of the feature, as `{column: value}`, and how many rows the feature has.
pub(super) fn decision_json(d: &DecisionTable, f: &Feature) -> (J, usize) {
    let mut last = None;
    let mut n = 0;
    for r in 0..d.len() {
        let same_pol = d.get(r, "pol").trim().eq_ignore_ascii_case(&f.pol);
        let same_id = d.get(r, "feature_id").trim().parse::<f64>().ok() == Some(f.id as f64);
        if same_pol && same_id {
            last = Some(r);
            n += 1;
        }
    }
    let j = last.map_or(J::Null, |r| {
        J::Arr(
            d.columns
                .iter()
                .map(|c| J::Arr(vec![J::s(c), J::s(d.get(r, c))]))
                .collect(),
        )
    });
    (j, n)
}

pub(super) fn checklist_json(marks: Vec<Option<String>>) -> J {
    J::Arr(
        CHECKLIST
            .iter()
            .zip(marks)
            .map(|((key, label), proof)| {
                J::Obj(vec![
                    ("key", J::s(key)),
                    ("label", J::s(label)),
                    ("proof", proof.map_or(J::Null, J::Str)),
                ])
            })
            .collect(),
    )
}

pub(super) fn current_json(f: &Feature, cur: &Current) -> J {
    J::Obj(vec![
        (
            "level",
            J::Str(page_level(cur.level, cur.level_sub, cur.decision_2b)),
        ),
        ("sub", J::s(&f.level_sub)),
        ("basis", J::s(&f.level_basis)),
        ("name", cur.name.map_or(J::Null, J::s)),
        ("identity", f.identity().map_or(J::Null, J::s)),
        ("group", cur.group.map_or(J::Null, J::s)),
    ])
}

pub(super) fn libraries_json(lib: &LibraryIndex) -> J {
    J::Arr(
        lib.summary()
            .into_iter()
            .map(|(l, n)| J::Arr(vec![J::Str(l), J::Num(n as f64)]))
            .collect(),
    )
}

/// The alternatives with the formula of the best proposal: the library entries and the CSI:FingerID candidates of the feature.
pub(super) fn same_formula_alts(
    b: &Bench,
    f: &Feature,
    p: &Proposal,
    texts: &SourceTexts,
    lib: &LibraryIndex,
    mgf: Option<&MgfBlock>,
) -> Vec<Alt> {
    let (pol, id) = (f.pol.as_str(), f.id);
    let csi: Vec<StructureCandidate> = csi_top5(texts)
        .into_iter()
        .find(|(k, _)| k.0 == pol && k.1 == id)
        .map(|x| x.1)
        .unwrap_or_default();
    let exp = p.evidence.view.as_ref().map(|v| Ms2 {
        mz: v.peaks.iter().map(|k| k.mz).collect(),
        it: v.peaks.iter().map(|k| k.rel).collect(),
        prec: mgf.map_or(f.mz.unwrap_or(0.0), |m| m.prec),
    });
    match p.best.formula.as_deref() {
        Some(formula) => same_formula(
            formula,
            lib,
            &AltSources {
                csi: &csi,
                exp: exp.as_ref(),
                tol: b.settings.tol_da,
                st: &GraphStructures,
            },
        ),
        None => Vec::new(),
    }
}

/// The chemical classes of the MS2 block from the rules table (`classi/1`), or null without MS2 or when the rules cannot run.
pub(super) fn classes_json(b: &Bench, f: &Feature, mgf: Option<&MgfBlock>) -> J {
    use crate::classi::{evaluate, to_json, Polarity, Table};
    let Some(m) = mgf else { return J::Null };
    let pol = if f.pol == "POS" {
        Polarity::Pos
    } else {
        Polarity::Neg
    };
    let peaks: Vec<(f64, f64)> = m.mz.iter().copied().zip(m.it.iter().copied()).collect();
    let Some(prec) = Some(m.prec).filter(|p| *p > 0.0).or(f.mz) else {
        return J::Null;
    };
    let report =
        Table::bundled().and_then(|t| evaluate(&t, &peaks, prec, pol, b.settings.ppm_frag));
    report.map_or(J::Null, |r| J::Raw(to_json(&r)))
}
