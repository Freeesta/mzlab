//! The targeted rows next to the detection: a hypothesis the detection never found in any file (every cell targeted or absent) becomes a row of the
//! aligned table, so the proofs, the kinetics and the list of candidates treat it as any other row.

use super::TargetResult;
use crate::table::{Aligned, CELL_ABSENT};

/// Window of the match with a row that already exists, and the largest chance of coincidence of a row that is added.
#[derive(Debug, Clone, Copy)]
pub struct JoinParams {
    pub ppm: f64,
    pub rt_tol: f64,
    pub max_coincidence: f64,
}

impl Default for JoinParams {
    fn default() -> Self {
        JoinParams {
            ppm: 5.0,
            rt_tol: 0.1,
            max_coincidence: 0.05,
        }
    }
}

impl TargetResult {
    /// The rows without a detected cell that no row of `a` holds (same m/z and RT) and whose chance of a coincidence is at most
    /// `p.max_coincidence`, as a table of ions; in registry order, so the result repeats.
    pub fn missing_rows(&self, a: &Aligned, p: &JoinParams) -> Aligned {
        let nf = a.n_files;
        let mut out = Aligned {
            n_files: nf,
            ..Aligned::default()
        };
        for r in self.rows.iter().filter(|r| {
            r.n_detected == 0 && r.cells.len() == nf && r.coincidence <= p.max_coincidence
        }) {
            out.mz.push(r.mz);
            out.rt.push(r.rt);
            for c in &r.cells {
                let absent = c.state == CELL_ABSENT;
                out.area.push(if absent { 0.0 } else { c.area });
                out.height.push(if absent { 0.0 } else { c.height });
                out.apex_rt.push(c.apex_rt);
                out.width.push(c.width);
                out.state.push(c.state);
            }
        }
        out
    }

    /// Appends to `a` the rows of `missing_rows` that `a` does not hold; returns how many.
    pub fn add_missing_to(&self, a: &mut Aligned, p: &JoinParams) -> usize {
        let missing = self.missing_rows(a, p);
        a.append_unheld(&missing, p.ppm, p.rt_tol)
    }
}
