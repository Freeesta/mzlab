//! Rows of one compound split by the alignment (same m/z within `merge_da`, RT within `merge_rt_min`) merged before the Dig reads the table, so
//! a kinetic is never cut into pieces (each piece has zeros where the other holds the peak, which lowers the fitted constants). Two rows stay apart
//! when one file holds a detected peak in both and the apexes are farther apart than half the wider peak: two resolved peaks (isomers).
//! Every merge is logged (`Merge`, the `merged` block of the summary) with the cells both rows had before, so the unmerged rows stay readable.

use mzlab_core::json::{round, Out};

use super::super::series::FileIn;
use crate::aligned::{Aligned, CELL_ABSENT, CELL_DETECTED};

/// The cells of one row before the merge: (area, state) per column.
#[derive(Debug, Clone, PartialEq)]
pub struct RowWas {
    pub row: usize,
    pub mz: f64,
    pub rt: f64,
    pub cells: Vec<(f64, u8)>,
}

/// One merge: `absorbed` went into `kept`; `taken` = the columns whose cell came from `absorbed`; `both` = columns with a detected peak in both.
#[derive(Debug, Clone, PartialEq)]
pub struct Merge {
    pub kept: RowWas,
    pub absorbed: RowWas,
    pub taken: Vec<usize>,
    pub both: usize,
}

fn was(t: &Aligned, r: usize) -> RowWas {
    RowWas {
        row: r,
        mz: t.mz[r],
        rt: t.rt[r],
        cells: (0..t.n_files)
            .map(|k| (t.area[t.cell(r, k)], t.state[t.cell(r, k)]))
            .collect(),
    }
}

fn detected_area(t: &Aligned, r: usize) -> f64 {
    (0..t.n_files)
        .map(|k| t.cell(r, k))
        .filter(|&c| t.state[c] == CELL_DETECTED && t.area[c].is_finite())
        .map(|c| t.area[c])
        .sum()
}

/// Whether a file holds two resolved peaks of rows `a` and `b`, and how many files hold a detected peak in both.
fn resolved(t: &Aligned, a: usize, b: usize) -> (bool, usize) {
    let mut both = 0;
    for k in 0..t.n_files {
        let (x, y) = (t.cell(a, k), t.cell(b, k));
        if t.state[x] != CELL_DETECTED || t.state[y] != CELL_DETECTED {
            continue;
        }
        both += 1;
        let d = (t.apex_rt[x] - t.apex_rt[y]).abs();
        let w = t.width[x].max(t.width[y]);
        if d.is_finite() && w.is_finite() && d > 0.5 * w {
            return (true, both);
        }
    }
    (false, both)
}

/// Merges the split rows of `t` in place (`da` Da, `rt` min; either at 0 switches it off) and returns the log. The row with the larger detected
/// area is kept; per column it takes the other row's cell when that one is detected and its own is not, or both are detected and the other's area
/// is larger. The absorbed row keeps its m/z and RT but holds no peak any more.
pub fn merge_split(t: &mut Aligned, da: f64, rt: f64) -> Vec<Merge> {
    if da <= 0.0 || rt <= 0.0 || t.n_files == 0 {
        return Vec::new();
    }
    let mut ord: Vec<usize> = (0..t.len()).collect();
    let tot: Vec<f64> = ord.iter().map(|&r| detected_area(t, r)).collect();
    mzlab_core::sortc::by(&mut ord, |&a, &b| tot[b].total_cmp(&tot[a]).then(a.cmp(&b)));
    let mut gone = vec![false; t.len()];
    let mut out = Vec::new();
    for (i, &a) in ord.iter().enumerate() {
        if gone[a] || tot[a] <= 0.0 {
            continue;
        }
        for &b in &ord[i + 1..] {
            if gone[b] || tot[b] <= 0.0 {
                continue;
            }
            if (t.mz[a] - t.mz[b]).abs() > da || (t.rt[a] - t.rt[b]).abs() > rt {
                continue;
            }
            let (apart, both) = resolved(t, a, b);
            if apart {
                continue;
            }
            let (ka, kb) = (was(t, a), was(t, b));
            let mut taken = Vec::new();
            for k in 0..t.n_files {
                let (x, y) = (t.cell(a, k), t.cell(b, k));
                let dx = t.state[x] == CELL_DETECTED;
                let dy = t.state[y] == CELL_DETECTED;
                if dy && (!dx || t.area[y] > t.area[x]) {
                    t.area[x] = t.area[y];
                    t.height[x] = t.height[y];
                    t.apex_rt[x] = t.apex_rt[y];
                    t.width[x] = t.width[y];
                    t.state[x] = t.state[y];
                    taken.push(k);
                }
                t.area[y] = 0.0;
                t.height[y] = 0.0;
                t.apex_rt[y] = f32::NAN;
                t.width[y] = f32::NAN;
                t.state[y] = CELL_ABSENT;
            }
            gone[b] = true;
            out.push(Merge {
                kept: ka,
                absorbed: kb,
                taken,
                both,
            });
        }
    }
    out
}

/// Why two rows were merged, in words.
pub fn reason(m: &Merge, files: &[FileIn]) -> String {
    let names: Vec<&str> = m
        .taken
        .iter()
        .filter_map(|&k| files.get(k).map(|f| f.label.as_str()))
        .collect();
    format!(
        "same ion split by the alignment: m/z {:.2} and {:.2} (Δ {:.2} Da), RT {:.2} and {:.2} min (Δ {:.2} min); detected in both in {} file(s) without two resolved peaks; cells taken from the absorbed row: {}",
        m.kept.mz,
        m.absorbed.mz,
        (m.kept.mz - m.absorbed.mz).abs(),
        m.kept.rt,
        m.absorbed.rt,
        (m.kept.rt - m.absorbed.rt).abs(),
        m.both,
        if names.is_empty() { "none".to_string() } else { names.join(", ") }
    )
}

fn row_json(o: &mut Out, r: &RowWas) {
    o.raw("{\"row\":")
        .int(r.row as i64)
        .raw(",\"mz\":")
        .num(round(r.mz, 4))
        .raw(",\"rt\":")
        .num(round(r.rt, 3))
        .raw(",\"areas\":[");
    for (i, c) in r.cells.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        if c.0.is_finite() {
            o.num(round(c.0, 1));
        } else {
            o.raw("null");
        }
    }
    o.raw("],\"states\":[");
    for (i, c) in r.cells.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(state_name(c.1));
    }
    o.raw("]}");
}

/// The state of a cell in words: `detected`, `forced` (area integrated where no peak was detected) or `absent`.
pub fn state_name(s: u8) -> &'static str {
    match s {
        CELL_DETECTED => "detected",
        x if crate::aligned::no_peak(x) => "absent",
        _ => "forced",
    }
}

/// The `merged` block: one object per merge, with both rows as they were.
pub fn json(o: &mut Out, ms: &[Merge], files: &[FileIn]) {
    o.raw("[");
    for (i, m) in ms.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"kept\":");
        row_json(o, &m.kept);
        o.raw(",\"absorbed\":");
        row_json(o, &m.absorbed);
        o.raw(",\"delta_mz\":")
            .num(round((m.kept.mz - m.absorbed.mz).abs(), 3))
            .raw(",\"delta_rt\":")
            .num(round((m.kept.rt - m.absorbed.rt).abs(), 3))
            .raw(",\"both_detected\":")
            .int(m.both as i64)
            .raw(",\"files_taken\":[");
        for (j, &k) in m.taken.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" })
                .str(files.get(k).map_or("", |f| f.name.as_str()));
        }
        o.raw("],\"reason\":").str(&reason(m, files)).raw("}");
    }
    o.raw("]");
}

#[cfg(test)]
#[path = "merge_tests.rs"]
mod tests;
