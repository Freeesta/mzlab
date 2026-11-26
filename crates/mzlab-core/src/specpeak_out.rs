//! The JSON of `/api/specpeak`: the head, the answers that make no spectrum, the answer that found a window, the background member and the lines.
//! Split from `specpeak.rs` for size; the members and their order are the contract in `docs/agenti/data-channels.md`.

use super::{ions_of, quote, rd, Cells, Geometry, Set, METHOD};
use crate::json::{round, Out};

/// The start of every answer: the level, the parameters in force, the range searched and the chromatogram behind it.
pub(super) fn head(
    o: &mut Out,
    level: u8,
    (lo, hi): (f64, f64),
    p: &Set,
    (n, in_range): (usize, usize),
    cells: Option<Cells>,
) {
    // the tolerance in force at a high-resolution level (the level's own when `--set` does not say), `null` where lines are not matched by ppm
    let tol = match cells {
        Some(Cells::Hr(t)) => t.to_string(),
        _ => "null".to_string(),
    };
    o.raw("{\"mode\":\"peak_auto\",\"ms_level\":")
        .int(i64::from(level));
    o.raw(&format!(
        ",\"params\":{{\"factor\":{},\"side_min\":{},\"gap_min\":{},\"min_sn\":{},\"min_points\":{},\"tol_ppm\":{}}}",
        p.factor, p.side, p.gap, p.min_sn, p.min_points, tol
    ));
    o.raw(&format!(",\"search_min\":[{lo},{hi}]"));
    let prec = if level > 1 { " and precursor" } else { "" };
    o.raw(&format!(",\"trace\":{{\"n_scans\":{n},\"n_scans_searched\":{in_range},\"basis\":\"summed intensity of every scan of the level{prec}\"}}"));
}

/// One side of the sketch: its window and the scans in it.
fn side_json(s: (f64, f64), v: (usize, f64)) -> String {
    format!(
        "{{\"window_min\":[{},{}],\"n_scans\":{}}}",
        rd(s.0),
        rd(s.1),
        v.0
    )
}

/// Everything a `peak` or `window_only` answer reports.
pub(super) struct Found<'a> {
    pub(super) g: &'a Geometry,
    pub(super) apex_json: &'a str,
    pub(super) sides: ((usize, f64), (usize, f64)),
    pub(super) background: &'a str,
    pub(super) sum: &'a str,
    pub(super) flags: &'a [&'static str],
    pub(super) rule: &'a str,
    /// How the lines were matched; `None` = no spectrum was made (`window_only`).
    pub(super) cells: Option<Cells>,
    pub(super) n_win: usize,
    pub(super) lines: (&'a [f64], &'a [f64]),
}

/// The rest of an answer that found a window: verdict, geometry, sides, background, sum and, last, the lines.
pub(super) fn emit(mut o: Out, f: &Found) -> String {
    let (g, (side_l, side_r)) = (f.g, f.sides);
    let flags: Vec<String> = f.flags.iter().map(|x| format!("\"{x}\"")).collect();
    let verdict = if f.cells.is_some() {
        "peak"
    } else {
        "window_only"
    };
    o.raw(&format!(
        ",\"verdict\":\"{verdict}\",\"flags\":[{}]",
        flags.join(",")
    ));
    o.raw(&format!(
        ",\"apex\":{},\"fwhm_min\":{},\"half_height\":{},\"window_min\":[{},{}]",
        f.apex_json,
        rd(g.fwhm),
        rd(g.half),
        rd(g.window.0),
        rd(g.window.1)
    ));
    o.raw(&format!(
        ",\"sides\":{{\"left\":{},\"right\":{},\"anchored_min\":[{},{}]}}",
        side_json(g.left, side_l),
        side_json(g.right, side_r),
        rd(g.anchored.0),
        rd(g.anchored.1)
    ));
    o.raw(",\"background\":").raw(f.background);
    o.raw(",\"cell\":").raw(&f.cells.map_or("null".to_string(), Cells::json));
    o.raw(",\"spectrum\":")
        .raw(if f.cells.is_some() { "true" } else { "false" });
    o.raw(",\"n_scans\":").int(f.n_win as i64);
    // the ions the window rests on: counts x fill time, null until the fill time is stated or known (`evidence.sum.ions`)
    o.raw(",\"total_ions\":")
        .raw(&ions_of(f.sum))
        .raw(",\"sum\":")
        .raw(f.sum);
    // the lines last: a reader that wants the evidence alone cuts the answer at `,"mz":[`
    o.raw(",\"rule\":").str(f.rule);
    lines(&mut o, f.lines, f.cells.map_or(3, Cells::decimals));
    o.0
}

/// The rest of an answer that makes no spectrum: the verdict, its rule and the apex candidate (`null` when there is none).
pub(super) fn without(
    mut o: Out,
    verdict: &str,
    rule: &str,
    apex: &str,
    in_range: usize,
) -> String {
    o.raw(",\"verdict\":\"").raw(verdict);
    o.raw("\",\"flags\":[],\"apex\":").raw(apex);
    o.raw(",\"fwhm_min\":null,\"window_min\":null,\"sides\":null,\"background\":null,\"cell\":null,");
    o.raw("\"spectrum\":false,\"n_scans\":")
        .int(in_range as i64);
    o.raw(",\"total_ions\":null,\"sum\":null,\"rule\":")
        .str(rule)
        .raw(",\"mz\":[],\"y\":[]}");
    o.0
}

/// The `background` member: what was subtracted and its level in cps per scan (`none` when no side has a scan).
pub(super) fn background_json(n_side: usize, total: f64) -> String {
    if n_side == 0 {
        return "{\"applied\":\"none\",\"method\":null,\"n_side_scans\":0,\"per_scan_cps\":null}"
            .to_string();
    }
    format!(
        "{{\"applied\":\"sides\",\"method\":{},\"n_side_scans\":{n_side},\"per_scan_cps\":{}}}",
        quote(METHOD),
        rd(total / n_side as f64)
    )
}

/// The lines of the spectrum, the last two members of the answer, and the closing brace.
pub(super) fn lines(o: &mut Out, (mz, y): (&[f64], &[f64]), nd: usize) {
    o.raw(",\"mz\":[");
    for (i, v) in mz.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).num(round(*v, nd));
    }
    o.raw("],\"y\":[");
    for (i, v) in y.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).num(round(*v, 1));
    }
    o.raw("]}");
}
