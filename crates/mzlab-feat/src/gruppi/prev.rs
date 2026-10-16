//! The suggestions "Prev." of a row in a condition, from the controls of that condition. A suggestion with its proof, never a confirmation and never
//! a removal of the row: no Schymanski level is assigned here.

use super::presenza::{detected, CondModel};
use super::{Ctx, Prev};

/// Largest area of the detected cells of the control files.
fn ctrl_area(cx: &Ctx, r: usize, files: &[usize]) -> f64 {
    let nf = cx.tb.n_files;
    files
        .iter()
        .filter(|&&f| detected(cx.tb.state[r * nf + f]))
        .map(|&f| cx.tb.area[r * nf + f])
        .fold(0.0, f64::max)
}

/// `pass` when the control has at most `ctrl_ratio` of the treated maximum, `fail` above, `na` without control files.
fn absent_in(cx: &Ctx, r: usize, files: &[usize], ymax: f64) -> &'static str {
    if files.is_empty() {
        "na"
    } else if ctrl_area(cx, r, files) <= cx.opts.ctrl_ratio * ymax {
        "pass"
    } else {
        "fail"
    }
}

fn in_std(cx: &Ctx, r: usize, files: &[usize], ymax: f64) -> &'static str {
    match absent_in(cx, r, files, ymax) {
        "pass" => "no",
        "fail" => "yes",
        _ => "na",
    }
}

fn ms2_status(cx: &Ctx, r: usize, c: &CondModel) -> &'static str {
    if !cx.has_ms2 {
        "na"
    } else if cx.tb.rows[r].ms2_files.iter().any(|f| c.files.contains(f)) {
        "linked"
    } else {
        "none"
    }
}

/// `Prev. unlikely` if the time zero or the blank has the ion; `Prev. possible` if the parent standard has it, or the time zero or the blank is
/// missing, or the MS2 is not linked (`none`) or not known (`na`); `Prev. likely` otherwise.
fn verdict(p: &Prev) -> &'static str {
    if p.absent_t0 == "fail" || p.absent_blank == "fail" {
        "Prev. unlikely"
    } else if p.in_parent_std == "yes"
        || p.absent_t0 == "na"
        || p.absent_blank == "na"
        || p.ms2 != "linked"
    {
        "Prev. possible"
    } else {
        "Prev. likely"
    }
}

/// The suggestions for a row present in condition `c`; `ymax` is its largest mean area there.
pub(crate) fn prev(cx: &Ctx, r: usize, c: &CondModel, ymax: f64) -> Prev {
    let above_min = if cx.opts.min_area > 0.0 {
        if ymax >= cx.opts.min_area {
            "pass"
        } else {
            "fail"
        }
    } else {
        "na"
    };
    let mut p = Prev {
        absent_t0: absent_in(cx, r, &c.ctrl.t0, ymax),
        absent_blank: absent_in(cx, r, &c.ctrl.blank, ymax),
        in_parent_std: in_std(cx, r, &c.ctrl.std, ymax),
        above_min,
        ms2: ms2_status(cx, r, c),
        verdict: "",
    };
    p.verdict = verdict(&p);
    p
}
