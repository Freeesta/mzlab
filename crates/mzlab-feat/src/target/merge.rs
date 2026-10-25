//! Targeted rows that name the same ion: rows within `ppm` in m/z whose RT windows overlap become one row. The merged row keeps the first
//! hypothesis (the lower m/z, then the registry order) and lists the others in `merged`; `TargetResult::name` gives the combined label.

use crate::axis::half;
use crate::hypo::Registry;
use crate::table::{CELL_ABSENT, CELL_DETECTED};

use super::{median, Cell, TargetParams, TargetResult, TargetRow};

/// Whether the cell `b` is a better pick than `a`: detected over targeted over absent, then the larger area.
fn better(a: &Cell, b: &Cell) -> bool {
    let rank = |c: &Cell| match c.state {
        CELL_ABSENT => 0,
        CELL_DETECTED => 2,
        _ => 1,
    };
    rank(b) > rank(a) || (rank(b) == rank(a) && b.area > a.area)
}

fn fold(into: &mut TargetRow, from: TargetRow) {
    into.merged.push(from.hyp);
    into.merged.extend(from.merged);
    into.window = (
        into.window.0.min(from.window.0),
        into.window.1.max(from.window.1),
    );
    for (a, b) in into.cells.iter_mut().zip(from.cells) {
        if better(a, &b) {
            *a = b;
        }
    }
    let count = |r: &TargetRow, s: u8| r.cells.iter().filter(|c| c.state == s).count() as u16;
    into.n_detected = count(into, CELL_DETECTED);
    into.n_targeted = count(into, crate::table::CELL_TARGETED);
    into.ms2_total = into.cells.iter().map(|c| c.ms2_n).sum::<u16>() as u32;
    let mut rts: Vec<f32> = into
        .cells
        .iter()
        .filter(|c| c.state != CELL_ABSENT)
        .map(|c| c.apex_rt)
        .collect();
    into.rt = median(&mut rts);
    into.coincidence = into.coincidence.max(from.coincidence);
}

/// Folds the rows of `rows` (in the order of the registry, by m/z) that are the same ion into the first of them.
pub(crate) fn merge_overlapping(rows: &mut Vec<TargetRow>, p: &TargetParams) {
    let mut kept: Vec<TargetRow> = Vec::with_capacity(rows.len());
    for row in rows.drain(..) {
        let tol = half(row.mz, p.ppm);
        let twin = kept.iter().rposition(|k| {
            (row.mz - k.mz).abs() <= tol && row.window.0 <= k.window.1 && k.window.0 <= row.window.1
        });
        match twin {
            Some(i) => fold(&mut kept[i], row),
            None => kept.push(row),
        }
    }
    *rows = kept;
}

impl TargetResult {
    /// The label of `row`: the labels of its hypothesis and of the ones merged into it, joined by « / ».
    pub fn name(&self, reg: &Registry, row: usize) -> String {
        let r = &self.rows[row];
        std::iter::once(r.hyp)
            .chain(r.merged.iter().copied())
            .map(|h| reg.hyps[h as usize].label.as_str())
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::CELL_TARGETED;

    fn cell(state: u8, area: f64) -> Cell {
        Cell {
            state,
            area,
            height: 1.0,
            apex_rt: 5.0,
            width: 0.1,
            snr: 5.0,
            ms2_n: 0,
            feature: 0,
        }
    }

    fn row(hyp: u32, mz: f64, win: (f32, f32), cells: Vec<Cell>) -> TargetRow {
        TargetRow {
            hyp,
            mz,
            rt: 5.0,
            window: win,
            cells,
            n_detected: 0,
            n_targeted: 0,
            ms2_total: 0,
            coincidence: 0.5,
            merged: Vec::new(),
        }
    }

    #[test]
    fn overlapping_windows_at_the_same_mass_become_one_row() {
        let p = TargetParams::default();
        let mut rows = vec![
            row(
                0,
                200.0,
                (1.0, 5.0),
                vec![cell(CELL_TARGETED, 10.0), cell(CELL_ABSENT, f64::NAN)],
            ),
            row(
                1,
                200.0005,
                (4.0, 8.0),
                vec![cell(CELL_DETECTED, 5.0), cell(CELL_TARGETED, 7.0)],
            ),
            row(
                2,
                200.0005,
                (6.0, 9.0),
                vec![cell(CELL_TARGETED, 1.0), cell(CELL_TARGETED, 1.0)],
            ),
            row(
                3,
                300.0,
                (1.0, 5.0),
                vec![cell(CELL_TARGETED, 1.0), cell(CELL_TARGETED, 1.0)],
            ),
        ];
        merge_overlapping(&mut rows, &p);
        assert_eq!(
            rows.len(),
            2,
            "rows 0, 1 and 2 are one chain; 3 has another mass"
        );
        assert_eq!(rows[0].merged, vec![1, 2]);
        assert_eq!(rows[0].window, (1.0, 9.0));
        assert_eq!((rows[0].n_detected, rows[0].n_targeted), (1, 1));
        assert_eq!(rows[0].cells[0].state, CELL_DETECTED);
        assert_eq!(rows[1].merged, Vec::<u32>::new());
    }

    #[test]
    fn disjoint_windows_stay_apart() {
        let mut rows = vec![
            row(0, 200.0, (1.0, 2.0), vec![cell(CELL_TARGETED, 1.0)]),
            row(1, 200.0, (3.0, 4.0), vec![cell(CELL_TARGETED, 1.0)]),
        ];
        merge_overlapping(&mut rows, &TargetParams::default());
        assert_eq!(rows.len(), 2);
    }
}
