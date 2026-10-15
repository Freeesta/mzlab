//! The aligned list of the mzmine stages as the `Aligned` table of mzLab, so that tables, exports and statistics read it unchanged. Rewritten from
//! scratch after mzmine 3.9.0 (MIT, (c) 2004-2022 The mzmine Development Team; the columns follow what `align_join/JoinAlignerTask.java` and
//! `gapfill_peakfinder` leave on a row: m/z, RT and, per file, a feature with height, area and state); no code copied.
//! Paper: MZmine 3, Schmid et al., Nat. Biotechnol. 41:447 (2023), doi:10.1038/s41587-023-01690-2.
//!
//! One row per ion (the table is in "per ion" mode: `member_*` stay empty, as for `Mode::Ion`). Areas go from intensity x minutes to counts x seconds
//! (x 60). A cell with a feature gets `CELL_DETECTED`, or `CELL_FILLED` when the gap filler made it; an empty cell is `CELL_ABSENT` with area 0 and NaN
//! apex and width. `apex_rt` = RT of the feature, `width` = RT range of the feature in minutes. Fields that the join does not produce (`q3`,
//! `meta`, `ident`, `tp`, the family columns) stay empty; the shape shared with the other pipeline is unchanged. Rows keep the order of mzmine (RT).

use super::join::Rows;
use crate::table::{Aligned, CELL_ABSENT, CELL_DETECTED, CELL_FILLED};

/// Counts x seconds per intensity x minute.
const MIN_TO_S: f64 = 60.0;

/// The `Aligned` table of `rows`; `files` are the names of the files in the order of `rows.cells` (their number sets the width of the table).
pub fn to_aligned(rows: &Rows, files: &[String]) -> Aligned {
    let nf = files.len();
    let mut t = Aligned::with_capacity(nf, rows.rows.len());
    t.n_files = nf;
    for row in &rows.rows {
        t.mz.push(row.mz);
        t.rt.push(row.rt);
        for k in 0..nf {
            match row.cells.get(k).and_then(Option::as_ref) {
                Some(f) => {
                    t.area.push(f.area * MIN_TO_S);
                    t.height.push(f.height as f32);
                    t.apex_rt.push(f.rt as f32);
                    t.width.push((f.rt_end - f.rt_start) as f32);
                    t.state
                        .push(if f.filled { CELL_FILLED } else { CELL_DETECTED });
                }
                None => {
                    t.area.push(0.0);
                    t.height.push(0.0);
                    t.apex_rt.push(f32::NAN);
                    t.width.push(f32::NAN);
                    t.state.push(CELL_ABSENT);
                }
            }
        }
    }
    t
}
