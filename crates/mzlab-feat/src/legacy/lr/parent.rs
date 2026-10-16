//! The parent of a low-resolution Dig: its row and the check that it is a chromatographic peak of the samples whose area falls (`parent_check`).

use super::super::series::FileIn;
use super::labels::Crit;
use super::plan::{ParentLr, Settings};
use super::{cond, evaluate, kin, labels, peakchk, rows, Entry};
use crate::aligned::Aligned;

/// The row of the parent (`prow`, after the calibration) with the criteria of `parent_check` and of its chromatographic peak; true when
/// `parent_check` passes.
#[allow(clippy::too_many_arguments)]
pub(super) fn parent_entry(
    t: &Aligned,
    lc: &[FileIn],
    parent: &ParentLr,
    hyps: &[rows::Hyp],
    s: &Settings,
    chk: &peakchk::Checks,
    prow: Option<usize>,
    off: f64,
) -> (Entry, bool) {
    let mut p = Entry {
        name: "parent".into(),
        formula: parent.formula.clone(),
        mz: parent.mz,
        mz_x: parent.mz + off,
        neutral_mass: Some(parent.neutral_mass),
        alternatives: rows::parent_alternatives(hyps, parent, s.tol_da),
        row: prow,
        ..Entry::default()
    };
    evaluate(&mut p, t, lc, s.persist());
    let (parent_ok, pc) = parent_check(&p, lc, s);
    (p.score, p.label, p.criteria) = (None, "progenitore", pc);
    let labels: Vec<String> = lc.iter().map(|f| f.label.clone()).collect();
    if let Some(k) = peakchk::criterion(t, chk, p.row, s, &labels) {
        p.criteria.insert(1, k);
    }
    (p, parent_ok)
}

/// `parent_check`: the parent must be a chromatographic peak in the samples and, with at least two timed samples, its area must fall: the maximum
/// before the last time and the last area at most half of it. A series where it does not is not the degradation of this parent, and the Dig
/// proposes no product (`Settings::parent_decay` = false switches the fall off).
fn parent_check(p: &Entry, lc: &[FileIn], s: &Settings) -> (bool, Vec<Crit>) {
    let peak = p
        .cells
        .iter()
        .zip(lc)
        .any(|(c, f)| c.detected && f.kind == "sample");
    let mut out = vec![labels::crit(
        "parent found",
        if peak { "pass" } else { "fail" },
        if peak {
            "the ion of the parent is a chromatographic peak in the samples".into()
        } else if p.row.is_none() {
            format!(
                "no ion within {} Da of m/z {:.4} in the samples",
                rows::OFFSET_MAX,
                p.mz
            )
        } else {
            "the signal at the m/z of the parent is not a chromatographic peak in any sample".into()
        },
    )];
    // with several conditions (a dark control, another catalyst) it is enough that the parent falls in one of them
    let conds = cond::conditions(lc);
    let views: Vec<String> = if conds.len() < 2 {
        vec![String::new()]
    } else {
        conds
    };
    let kins: Vec<(String, Vec<kin::Point>)> = views
        .into_iter()
        .map(|c| {
            let k = cond::kinetics_in(lc, &p.cells, &c);
            (c, k)
        })
        .filter(|(_, k)| k.len() >= 2)
        .collect();
    let falls = if !s.parent_decay {
        labels::crit("parent falls over time", "n/a", "switched off".into())
    } else if kins.is_empty() || !peak {
        labels::crit(
            "parent falls over time",
            "n/a",
            "at least two timed samples with a peak are required".into(),
        )
    } else {
        let judged: Vec<(bool, String)> = kins.iter().map(|(c, k)| falls_in(c, k)).collect();
        let best = judged.iter().find(|j| j.0).unwrap_or(&judged[0]);
        labels::crit(
            "parent falls over time",
            if best.0 { "pass" } else { "fail" },
            if best.0 {
                best.1.clone()
            } else {
                format!(
                    "{}: no product is proposed for a parent that does not degrade",
                    best.1
                )
            },
        )
    };
    let ok = peak && falls.status != "fail";
    out.push(falls);
    (ok, out)
}

/// Whether the area of the parent falls in the kinetics `kin` of condition `cond` (empty = the whole series), with the text of the proof.
fn falls_in(cond: &str, kin: &[kin::Point]) -> (bool, String) {
    let im = (0..kin.len()).fold(0, |m, i| if kin[i].area > kin[m].area { i } else { m });
    let last = kin[kin.len() - 1].area;
    let ok = im + 1 < kin.len() && last <= 0.5 * kin[im].area;
    let pct = if kin[im].area > 0.0 {
        100.0 * last / kin[im].area
    } else {
        0.0
    };
    let place = if cond.is_empty() {
        String::new()
    } else {
        format!("{cond}: ")
    };
    (
        ok,
        format!(
            "{place}maximum at {} min, last area {pct:.0} % of it (needs a maximum before the last time and at most 50 %)",
            labels::g(kin[im].time)
        ),
    )
}
