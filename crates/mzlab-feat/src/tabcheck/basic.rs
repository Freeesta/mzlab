//! The checks of `tabcheck/1` that read the cells file by file: size, blanks, contaminants, QC, RT drift, long traces, missing cells, polarity and units.

use super::*;
use crate::aligned::{no_peak, CELL_DETECTED};

/// A file with fewer rows than this share of the median of the samples is flagged.
const SIZE_MIN_SHARE: f64 = 0.5;
/// Sample median over the highest blank area below which a row is as intense in the blank as in the samples.
const BLANK_RATIO: f64 = 3.0;
/// Share of the rows with such a ratio from which the blank check warns.
const BLANK_WARN_SHARE: f64 = 0.05;
/// A row in every file is a contaminant when its areas vary less than this CV, and among this many of the most intense rows.
const CONST_CV: f64 = 0.3;
const TOP_ROWS: usize = 20;
/// QC: median CV above `QC_CV_MEDIAN`, or more than `QC_CV_SHARE` of the rows above `QC_CV_ROW`.
const QC_CV_MEDIAN: f64 = 0.2;
const QC_CV_ROW: f64 = 0.3;
const QC_CV_SHARE: f64 = 0.25;
/// Change of the QC level between the first and the last QC that warns.
const QC_LEVEL_CHANGE: f64 = 0.2;
/// RT shift between files (range of the per-file medians) that warns, in FWHM; share of the files a row must be detected in to be common.
const RT_DRIFT_FWHM: f64 = 2.0;
const COMMON_SHARE: f64 = 0.8;
const COMMON_ROWS: usize = 5;

/// Rows with a peak in each file; a file with far fewer rows than the median of the samples was a blank, or failed.
pub(super) fn size(e: &Env) -> Check {
    let counts: Vec<usize> = (0..e.t.n_files)
        .map(|k| e.rows.iter().filter(|&&r| e.area(r, k).is_some()).count())
        .collect();
    let pool: Vec<usize> = e
        .live
        .iter()
        .copied()
        .filter(|k| !e.blanks.contains(k))
        .collect();
    if pool.is_empty() {
        return Check::missing("size", "files");
    }
    let med = median(pool.iter().map(|&k| counts[k] as f64).collect());
    let mut low: Vec<(usize, f64)> = pool
        .iter()
        .map(|&k| (k, counts[k] as f64 / med))
        .filter(|x| x.1 < SIZE_MIN_SHARE)
        .collect();
    rank(&mut low, false);
    let worst = pool
        .iter()
        .map(|&k| counts[k] as f64 / med)
        .fold(f64::INFINITY, f64::min);
    let cells = e.rows.len() * e.live.len();
    let empty = e
        .rows
        .iter()
        .map(|&r| {
            e.live
                .iter()
                .filter(|&&k| no_peak(e.t.state[r * e.t.n_files + k]))
                .count()
        })
        .sum::<usize>();
    let mut p = Proof::new();
    p.i("rows", e.rows.len())
        .i("files", e.t.n_files)
        .n("median_rows_per_file", med)
        .n("empty_cell_share", empty as f64 / cells.max(1) as f64);
    p.i("low_files", low.len());
    Check {
        id: "size",
        value: worst.is_finite().then_some(worst),
        threshold: Some(SIZE_MIN_SHARE),
        flag: if low.is_empty() { Flag::Ok } else { Flag::Warn },
        rows: Vec::new(),
        files: e.names(&low.iter().map(|x| x.0).collect::<Vec<_>>()),
        proof: p.done(),
    }
}

/// Rows with a peak in a blank that are not at least `BLANK_RATIO` times more intense in the samples than in the blanks.
pub(super) fn blank(e: &Env) -> Check {
    if e.blanks.is_empty() {
        return Check::missing("blank", "blank");
    }
    if e.samples.is_empty() {
        return Check::missing("blank", "samples");
    }
    let mut hit: Vec<(usize, f64)> = Vec::new();
    let mut with_blank = 0;
    for &r in &e.rows {
        let top = e.areas(r, &e.blanks).into_iter().fold(0.0, f64::max);
        if top <= 0.0 {
            continue;
        }
        with_blank += 1;
        let ratio = median(e.areas(r, &e.samples)) / top;
        if ratio < BLANK_RATIO {
            hit.push((r, ratio));
        }
    }
    rank(&mut hit, false);
    let share = hit.len() as f64 / e.rows.len() as f64;
    let mut p = Proof::new();
    p.i("blank_files", e.blanks.len())
        .i("rows_with_blank_peak", with_blank)
        .i("rows_below", hit.len())
        .n("warn_share", BLANK_WARN_SHARE);
    let flag = if hit.is_empty() {
        Flag::Ok
    } else if share >= BLANK_WARN_SHARE {
        Flag::Warn
    } else {
        Flag::Note
    };
    Check {
        id: "blank",
        value: Some(share),
        threshold: Some(BLANK_RATIO),
        flag,
        rows: e.ids(&hit.iter().map(|x| x.0).collect::<Vec<_>>()),
        files: Vec::new(),
        proof: p.done(),
    }
}

/// Rows in every file with an almost constant area among the most intense rows: a contaminant of the system, not of the samples.
pub(super) fn contaminants(e: &Env) -> Check {
    if e.live.len() < 3 {
        return Check::missing("contaminants", "files");
    }
    let mut top: Vec<(usize, f64)> = e
        .rows
        .iter()
        .map(|&r| (r, median(e.areas(r, &e.live))))
        .collect();
    rank(&mut top, true);
    top.truncate(TOP_ROWS);
    let found: Vec<usize> = top
        .iter()
        .map(|x| x.0)
        .filter(|&r| e.present_in_all(r, &e.live) && cv(&e.areas(r, &e.live)) <= CONST_CV)
        .collect();
    let mut p = Proof::new();
    p.i("blank_files", e.blanks.len())
        .i("top_rows", top.len())
        .n("max_cv", CONST_CV);
    let flag = match (found.is_empty(), e.blanks.is_empty()) {
        (true, _) => Flag::Ok,
        (false, true) => Flag::Note,
        (false, false) => Flag::Warn,
    };
    Check {
        id: "contaminants",
        value: Some(found.len() as f64),
        threshold: Some(1.0),
        flag,
        rows: e.ids(&found),
        files: Vec::new(),
        proof: p.done(),
    }
}

/// Median CV of the areas over the QC files, for the rows with a peak in all of them.
pub(super) fn qc_cv(e: &Env) -> Check {
    if e.qcs.len() < 2 {
        return Check::missing("qc_cv", "qc");
    }
    let mut cvs: Vec<(usize, f64)> = e
        .rows
        .iter()
        .filter(|&&r| e.present_in_all(r, &e.qcs))
        .map(|&r| (r, cv(&e.areas(r, &e.qcs))))
        .filter(|x| x.1.is_finite())
        .collect();
    if cvs.is_empty() {
        let mut c = Check::missing("qc_cv", "common_rows");
        c.proof = "{\"missing\":\"common_rows\",\"qc_files\":".to_string()
            + &e.qcs.len().to_string()
            + "}";
        return c;
    }
    let med = median(cvs.iter().map(|x| x.1).collect());
    let share = cvs.iter().filter(|x| x.1 > QC_CV_ROW).count() as f64 / cvs.len() as f64;
    rank(&mut cvs, true);
    let mut p = Proof::new();
    p.i("qc_files", e.qcs.len())
        .i("rows", cvs.len())
        .n("share_above_row_cv", share)
        .n("row_cv", QC_CV_ROW)
        .n("share_limit", QC_CV_SHARE);
    let flag = if med > QC_CV_MEDIAN || share > QC_CV_SHARE {
        Flag::Warn
    } else {
        Flag::Ok
    };
    Check {
        id: "qc_cv",
        value: Some(med),
        threshold: Some(QC_CV_MEDIAN),
        flag,
        rows: e.ids(&cvs.iter().map(|x| x.0).collect::<Vec<_>>()),
        files: Vec::new(),
        proof: p.done(),
    }
}

/// Level of each QC (median of the area over the mean of the row, for the rows in all QC) along the injection order; the change from the first to the last.
pub(super) fn qc_intensity(e: &Env) -> Check {
    if e.qcs.len() < 2 {
        return Check::missing("qc_intensity", "qc");
    }
    let common: Vec<usize> = e
        .rows
        .iter()
        .copied()
        .filter(|&r| e.present_in_all(r, &e.qcs))
        .collect();
    if common.is_empty() {
        return Check::missing("qc_intensity", "common_rows");
    }
    let mut ord = e.qcs.clone();
    crate::sortc::by(&mut ord, |&a, &b| {
        e.s[a].order.cmp(&e.s[b].order).then(a.cmp(&b))
    });
    let level: Vec<f64> = ord
        .iter()
        .map(|&k| {
            median(
                common
                    .iter()
                    .map(|&r| {
                        let all = e.areas(r, &e.qcs);
                        e.area(r, k).unwrap_or(0.0) / mean(&all)
                    })
                    .collect(),
            )
        })
        .collect();
    let (first, last) = (level[0], level[level.len() - 1]);
    let change = last / first - 1.0;
    let order: Vec<f64> = ord.iter().map(|&k| f64::from(e.s[k].order)).collect();
    let mut p = Proof::new();
    p.i("qc_files", ord.len())
        .i("rows", common.len())
        .n("first_level", first)
        .n("last_level", last);
    if let Some(sl) = slope(&order, &level) {
        p.n("level_slope_per_injection", sl);
    }
    let flag = if change.abs() > QC_LEVEL_CHANGE {
        Flag::Warn
    } else {
        Flag::Ok
    };
    Check {
        id: "qc_intensity",
        value: change.is_finite().then_some(change),
        threshold: Some(QC_LEVEL_CHANGE),
        flag,
        rows: Vec::new(),
        files: e.names(&[ord[0], ord[ord.len() - 1]]),
        proof: p.done(),
    }
}

/// Shift of the apex RT of the common rows file by file (median over the rows, against the median of each row); its range against twice the FWHM.
pub(super) fn rt_drift(e: &Env) -> Check {
    let inj: Vec<usize> = e
        .live
        .iter()
        .copied()
        .filter(|k| !e.blanks.contains(k))
        .collect();
    if inj.len() < 3 {
        return Check::missing("rt_drift", "files");
    }
    let need = (COMMON_SHARE * inj.len() as f64).ceil() as usize;
    let apex = |r: usize, k: usize| -> Option<f64> {
        let c = r * e.t.n_files + k;
        (e.t.state[c] == CELL_DETECTED && e.t.apex_rt[c].is_finite())
            .then(|| f64::from(e.t.apex_rt[c]))
    };
    let mut shifts: Vec<Vec<f64>> = vec![Vec::new(); inj.len()];
    let mut common = 0;
    for &r in &e.rows {
        let v: Vec<Option<f64>> = inj.iter().map(|&k| apex(r, k)).collect();
        if v.iter().flatten().count() < need {
            continue;
        }
        common += 1;
        let m = median(v.iter().flatten().copied().collect());
        for (j, x) in v.iter().enumerate() {
            if let Some(x) = x {
                shifts[j].push(x - m);
            }
        }
    }
    if common < COMMON_ROWS {
        return Check::missing("rt_drift", "common_rows");
    }
    let per_file: Vec<f64> = shifts.into_iter().map(median).collect();
    let live: Vec<usize> = (0..inj.len())
        .filter(|&j| per_file[j].is_finite())
        .collect();
    let (lo, hi) = (
        live.iter()
            .copied()
            .min_by(|&a, &b| per_file[a].total_cmp(&per_file[b]))
            .unwrap_or(0),
        live.iter()
            .copied()
            .max_by(|&a, &b| per_file[a].total_cmp(&per_file[b]))
            .unwrap_or(0),
    );
    let range = per_file[hi] - per_file[lo];
    let order: Vec<f64> = live.iter().map(|&j| f64::from(e.s[inj[j]].order)).collect();
    let sh: Vec<f64> = live.iter().map(|&j| per_file[j]).collect();
    let mut p = Proof::new();
    p.i("common_rows", common)
        .n("fwhm_min", e.fwhm)
        .n("max_shift_min", per_file[hi])
        .n("min_shift_min", per_file[lo]);
    if let Some(sl) = slope(&order, &sh) {
        p.n("slope_min_per_injection", sl);
    }
    if let Some(r) = pearson(&order, &sh) {
        p.n("r_order", r);
    }
    let thr = RT_DRIFT_FWHM * e.fwhm;
    Check {
        id: "rt_drift",
        value: Some(range),
        threshold: Some(thr),
        flag: if range > thr { Flag::Warn } else { Flag::Ok },
        rows: Vec::new(),
        files: e.names(&[inj[lo], inj[hi]]),
        proof: p.done(),
    }
}
