//! The checks of `tabcheck/1` that read the table as a whole: homologous series (Kendrick mass defect) and the structure of the injections (PCA).

use super::*;
use crate::stat::{analyse, units, Ion, KmdOpts};
use mzlab_core::pca::{pca_analysis, PcaOpts};

/// A series needs this many members, steps of one unit in this share of its gaps, and a retention trend with this |rho| to be reported.
const MIN_MEMBERS: usize = 5;
const MIN_CONSECUTIVE: f64 = 0.8;
const MIN_RHO: f64 = 0.8;
/// Width of the window of KMD, mDa.
const KMD_TOL_MDA: f64 = 3.0;
/// Rows (the most intense) the PCA is made of.
const MAX_PCA_ROWS: usize = 2000;
/// |r| of the first component with the injection order from which a batch effect is suspected.
const ORDER_R: f64 = 0.7;
/// QC spread on the first two components over the spread of all the injections above which the QC are scattered.
const QC_SPREAD: f64 = 0.5;

/// Series of the repeating units of the Kendrick analysis with at least `MIN_MEMBERS` members that step by one unit and elute in order; they are candidates
/// (polymers, surfactants, perfluorinated chains). Needs the accurate m/z of a high-resolution table.
pub(super) fn homologous(e: &Env) -> Check {
    if e.unit_resolution() {
        return Check::missing("homologous", "high_resolution");
    }
    let ions: Vec<Ion> = e
        .rows
        .iter()
        .map(|&r| Ion {
            mz: e.t.mz[r],
            rt: Some(e.t.rt[r]),
        })
        .collect();
    let mut found: Vec<(&'static str, usize, f64, f64, usize)> = Vec::new();
    let mut members: Vec<Vec<usize>> = Vec::new();
    for u in units() {
        let out = analyse(
            &ions,
            &KmdOpts {
                unit: u.id,
                tol_mda: KMD_TOL_MDA,
                min_members: MIN_MEMBERS,
            },
        );
        for s in out.series.iter().filter(|s| {
            s.members.len() >= MIN_MEMBERS
                && s.consecutive >= MIN_CONSECUTIVE
                && s.rho.is_some_and(|r| r.abs() >= MIN_RHO)
        }) {
            found.push((
                u.id,
                s.members.len(),
                s.consecutive,
                s.rho.unwrap_or(0.0),
                members.len(),
            ));
            members.push(s.members.iter().map(|&i| e.rows[i]).collect());
        }
    }
    crate::sortc::by(&mut found, |a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let mut list = Out::default();
    list.raw("[");
    for (i, s) in found.iter().take(e.caps.pairs).enumerate() {
        list.raw(if i > 0 { ",{\"unit\":" } else { "{\"unit\":" })
            .str(s.0)
            .raw(",\"members\":")
            .int(s.1 as i64);
        list.raw(",\"consecutive\":")
            .num(round(s.2, 3))
            .raw(",\"rho\":")
            .num(round(s.3, 3))
            .raw("}");
    }
    list.raw("]");
    let mut p = Proof::new();
    p.i("series", found.len())
        .i("min_members", MIN_MEMBERS)
        .n("tol_mda", KMD_TOL_MDA)
        .raw("list", &list.0);
    let rows: Vec<usize> = found
        .first()
        .map(|s| members[s.4].clone())
        .unwrap_or_default();
    Check {
        id: "homologous",
        value: Some(found.len() as f64),
        threshold: Some(MIN_MEMBERS as f64),
        flag: if found.is_empty() {
            Flag::Ok
        } else {
            Flag::Note
        },
        rows: e.ids(&rows),
        files: Vec::new(),
        proof: p.done(),
    }
}

fn column(scores: &[mzlab_core::json::Value], j: usize) -> Vec<f64> {
    scores
        .iter()
        .map(|r| r.arr().get(j).and_then(|v| v.f64()).unwrap_or(0.0))
        .collect()
}

/// PCA of the injections (`table.pca`, the most intense rows): the QC should stay together and the first component should not follow the injection order.
pub(super) fn structure(e: &Env) -> Check {
    let mut by: Vec<(usize, f64)> = e
        .rows
        .iter()
        .map(|&r| {
            (
                r,
                e.areas(r, &(0..e.t.n_files).collect::<Vec<_>>())
                    .into_iter()
                    .fold(0.0, f64::max),
            )
        })
        .collect();
    rank(&mut by, true);
    by.truncate(MAX_PCA_ROWS);
    let all: Vec<usize> = (0..e.t.n_files).collect();
    let matrix: Vec<Vec<f64>> = by.iter().map(|x| e.areas(x.0, &all)).collect();
    let roles: Vec<String> = e.s.iter().map(|s| s.kind.clone()).collect();
    let out = match pca_analysis(&matrix, &roles, None, &PcaOpts::default()) {
        Ok(v) => v,
        Err(err) => return Check::missing("structure", err.key.unwrap_or("pca")),
    };
    let ids = |k: &str| -> Vec<usize> {
        out.get(k)
            .map(|v| {
                v.arr()
                    .iter()
                    .filter_map(|x| x.f64())
                    .map(|x| x as usize)
                    .collect()
            })
            .unwrap_or_default()
    };
    let files = ids("samples");
    let scores = out
        .get("scores")
        .map(|v| v.arr().to_vec())
        .unwrap_or_default();
    if scores.len() != files.len() || files.is_empty() {
        return Check::missing("structure", "pca");
    }
    let order: Vec<f64> = files.iter().map(|&k| f64::from(e.s[k].order)).collect();
    let (pc1, pc2) = (column(&scores, 0), column(&scores, 1));
    let r1 = pearson(&order, &pc1);
    let spread = |idx: &[usize]| -> f64 {
        let (cx, cy) = (
            mean(&idx.iter().map(|&j| pc1[j]).collect::<Vec<_>>()),
            mean(&idx.iter().map(|&j| pc2[j]).collect::<Vec<_>>()),
        );
        (idx.iter()
            .map(|&j| (pc1[j] - cx).powi(2) + (pc2[j] - cy).powi(2))
            .sum::<f64>()
            / idx.len() as f64)
            .sqrt()
    };
    let qc: Vec<usize> = (0..files.len())
        .filter(|&j| e.s[files[j]].kind == "qc")
        .collect();
    let everything: Vec<usize> = (0..files.len()).collect();
    let qc_ratio = (qc.len() >= 2)
        .then(|| spread(&qc) / spread(&everything))
        .filter(|x| x.is_finite());
    let mut p = Proof::new();
    p.i("injections", files.len())
        .i("rows", matrix.len())
        .i("qc_files", qc.len())
        .n("qc_limit", QC_SPREAD);
    if let Some(x) = qc_ratio {
        p.n("qc_spread_ratio", x);
    }
    if let Some(r) = pearson(&order, &pc2) {
        p.n("r_pc2_order", r);
    }
    if let Some(x) = out.get("explained").map(|v| {
        v.arr()
            .iter()
            .filter_map(|x| x.f64())
            .take(2)
            .collect::<Vec<_>>()
    }) {
        p.raw(
            "explained",
            &Out::default()
                .floats(&x.iter().map(|v| round(*v, 4)).collect::<Vec<_>>())
                .0
                .clone(),
        );
    }
    let bad = r1.is_some_and(|r| r.abs() > ORDER_R) || qc_ratio.is_some_and(|x| x > QC_SPREAD);
    Check {
        id: "structure",
        value: r1.map(f64::abs),
        threshold: Some(ORDER_R),
        flag: if bad { Flag::Warn } else { Flag::Ok },
        rows: Vec::new(),
        files: Vec::new(),
        proof: p.done(),
    }
}
