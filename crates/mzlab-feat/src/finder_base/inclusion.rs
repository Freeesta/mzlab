//! The row and the header of the inclusion list of mzFinder (m/z, charge, RT window, note), with the numbers printed as `str(float)` of Python.
//! Written by the list of the Dig (`legacy::out`) and by the step `export.inclusion` (`export_inclusion`).

use mzlab_core::json::round;

/// Header of the inclusion list.
pub const INCLUSION_HEADER: &str = "m/z,charge,RT_start_min,RT_end_min,note";

/// One row of the inclusion list: m/z, charge 1, RT window in minutes, formula and priority as the note.
pub fn inclusion_row(mz: f64, rt_start: f64, rt_end: f64, formula: &str, priority: f64) -> String {
    format!(
        "{},1,{},{},\"{} priority {}\"",
        num(round(mz, 4)),
        num(round(rt_start, 2)),
        num(round(rt_end, 2)),
        formula,
        num(round(priority, 1))
    )
}

/// `str(float)` of Python for the numbers of the list (`5.0`, `254.0597`).
fn num(v: f64) -> String {
    let s = format!("{v}");
    if s.contains(['.', 'e', 'N', 'i']) {
        s
    } else {
        s + ".0"
    }
}
