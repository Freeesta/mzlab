//! Queries and the byte layout of `Aligned`.

use crate::aligned::{Aligned, RowQ, CELL_DETECTED, CELL_FILLED, CELL_TARGETED};
use crate::table::ROLE_M;

impl Aligned {
    /// Stable id of `row`: its 1-based position in the final order of the table (m/z, then RT, then position). It is the `id` of the exports and of `mzid/1`;
    /// it holds for one recipe and one set of inputs (the same bytes of the `.mzfeat`), not across recipes.
    pub fn id(&self, row: usize) -> u32 {
        row as u32 + 1
    }

    /// Row with the given `id` (None when out of range).
    pub fn row_of_id(&self, id: u32) -> Option<usize> {
        (id >= 1 && (id as usize) <= self.len()).then(|| id as usize - 1)
    }

    /// `(detected, targeted, filled)` cells of `row`; the rest of the files are `CELL_ABSENT`. Presence is read from `state`, never from `area > 0`.
    pub fn state_counts(&self, row: usize) -> (u16, u16, u16) {
        let cells = &self.state[row * self.n_files..(row + 1) * self.n_files];
        let n = |s: u8| cells.iter().filter(|&&c| c == s).count() as u16;
        (n(CELL_DETECTED), n(CELL_TARGETED), n(CELL_FILLED))
    }

    /// Whether the rows are in the order that defines the ids: m/z ascending, ties by RT ascending.
    pub fn is_id_ordered(&self) -> bool {
        (1..self.len()).all(|i| (self.mz[i - 1], self.rt[i - 1]) <= (self.mz[i], self.rt[i]))
    }

    /// Copies the cells of row `r` of `src` to the end of `self` (columns only, not the members).
    fn push_cells(&mut self, src: &Aligned, r: usize) {
        let c = r * src.n_files..(r + 1) * src.n_files;
        self.area.extend_from_slice(&src.area[c.clone()]);
        self.height.extend_from_slice(&src.height[c.clone()]);
        self.apex_rt.extend_from_slice(&src.apex_rt[c.clone()]);
        self.width.extend_from_slice(&src.width[c.clone()]);
        self.state.extend_from_slice(&src.state[c]);
    }

    /// Whether a row has the m/z (within `ppm`, as reference ion or member) and the RT (within `rt_tol` minutes) of an ion.
    pub fn holds(&self, mz: f64, rt: f64, ppm: f64, rt_tol: f64) -> bool {
        self.rows_near(mz, ppm)
            .iter()
            .any(|&i| (self.rt[i] - rt).abs() <= rt_tol)
    }

    /// Appends the rows of `other`, a table of the same files in "per ion" mode (the slabs of an alignment, one after the other).
    pub fn append(&mut self, other: &Aligned) {
        debug_assert!(
            self.meta.is_empty() && other.meta.is_empty(),
            "the summaries are computed after the alignment"
        );
        self.mz.extend_from_slice(&other.mz);
        self.rt.extend_from_slice(&other.rt);
        self.area.extend_from_slice(&other.area);
        self.height.extend_from_slice(&other.height);
        self.apex_rt.extend_from_slice(&other.apex_rt);
        self.width.extend_from_slice(&other.width);
        self.state.extend_from_slice(&other.state);
    }

    /// Appends the rows of `other` (a table of ions of the same files) that no row of `self` holds; returns how many. In "per family" mode the
    /// ion of the new row is its own member.
    pub fn append_unheld(&mut self, other: &Aligned, ppm: f64, rt_tol: f64) -> usize {
        let mut added = 0;
        let new: Vec<usize> = (0..other.len())
            .filter(|&r| !self.holds(other.mz[r], other.rt[r], ppm, rt_tol))
            .collect();
        for r in new {
            self.mz.push(other.mz[r]);
            self.rt.push(other.rt[r]);
            self.push_cells(other, r);
            if !self.meta.is_empty() {
                self.meta.push(&if other.meta.len() == other.len() {
                    other.meta.get(r)
                } else {
                    RowQ::NONE
                });
            }
            if !self.member_start.is_empty() {
                self.member_mz.push(other.mz[r]);
                self.member_rt.push(other.rt[r] as f32);
                self.member_role.push(ROLE_M);
                self.member_start.push(self.member_mz.len() as u32);
            }
            added += 1;
        }
        added
    }

    /// Keeps the rows that look like a product of a series: not detected in the first file (the untreated reference) and detected in one unbroken run of at
    /// least `files` files (noise and constant background do not appear and disappear only once).
    pub fn retain_recurring(&mut self, files: usize) {
        let nf = self.n_files;
        let mut out = Aligned {
            n_files: nf,
            ..Aligned::default()
        };
        let had_members = !self.member_start.is_empty();
        if had_members {
            out.member_start.push(0);
        }
        for r in 0..self.len() {
            let cells = &self.state[r * nf..(r + 1) * nf];
            let on: Vec<usize> = (0..nf).filter(|&k| cells[k] == CELL_DETECTED).collect();
            let run = on.len() >= files && on[on.len() - 1] - on[0] + 1 == on.len();
            if !run || on[0] == 0 {
                continue;
            }
            out.mz.push(self.mz[r]);
            out.rt.push(self.rt[r]);
            out.push_cells(self, r);
            if !self.meta.is_empty() {
                out.meta.push(&self.meta.get(r));
            }
            for m in self.members(r) {
                out.member_mz.push(self.member_mz[m]);
                out.member_rt.push(self.member_rt[m]);
                out.member_role.push(self.member_role[m]);
            }
            if had_members {
                out.member_start.push(out.member_mz.len() as u32);
            }
        }
        *self = out;
    }

    /// Puts the rows in the order that defines the ids: m/z ascending, ties by RT, then by the order they had (stable). The cells and the members follow their row.
    pub fn sort_rows(&mut self) {
        if self.is_id_ordered() {
            return;
        }
        let mut order: Vec<u32> = (0..self.len() as u32).collect();
        crate::sortc::by(&mut order, |&a, &b| {
            let (a, b) = (a as usize, b as usize);
            self.mz[a]
                .total_cmp(&self.mz[b])
                .then(self.rt[a].total_cmp(&self.rt[b]))
        });
        let nf = self.n_files;
        let mut out = Aligned::with_capacity(nf, self.len());
        let had_members = !self.member_start.is_empty();
        if had_members {
            out.member_start.push(0);
        }
        for &r in &order {
            let r = r as usize;
            out.mz.push(self.mz[r]);
            out.rt.push(self.rt[r]);
            out.push_cells(self, r);
            if !self.meta.is_empty() {
                out.meta.push(&self.meta.get(r));
            }
            for m in self.members(r) {
                out.member_mz.push(self.member_mz[m]);
                out.member_rt.push(self.member_rt[m]);
                out.member_role.push(self.member_role[m]);
            }
            if had_members {
                out.member_start.push(out.member_mz.len() as u32);
            }
        }
        *self = out;
    }

    /// Rows with the reference ion or any member within `ppm` of `mz`, ascending (the way to find an ion that is not the reference of its family).
    pub fn rows_near(&self, mz: f64, ppm: f64) -> Vec<usize> {
        let tol = mz * ppm * 1e-6;
        (0..self.len())
            .filter(|&r| {
                (self.mz[r] - mz).abs() <= tol
                    || self
                        .members(r)
                        .any(|m| (self.member_mz[m] - mz).abs() <= tol)
            })
            .collect()
    }

    /// Bytes of the table in a fixed layout (little-endian, column by column): the input of the digest used to compare two runs.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(self.heap_bytes() + 32);
        b.extend_from_slice(&(self.len() as u64).to_le_bytes());
        b.extend_from_slice(&(self.n_files as u64).to_le_bytes());
        macro_rules! put {
            ($($c:ident),*) => { $( for v in &self.$c { b.extend_from_slice(&v.to_le_bytes()); } )* };
        }
        put!(
            mz,
            rt,
            area,
            height,
            apex_rt,
            width,
            member_start,
            member_mz,
            member_rt
        );
        b.extend_from_slice(&self.state);
        b.extend_from_slice(&self.member_role);
        let m = &self.meta;
        for v in [
            &m.rt_lo,
            &m.rt_hi,
            &m.q_shape,
            &m.q_snr,
            &m.q_fwhm,
            &m.iso_m1,
            &m.ms2_purity,
        ] {
            for x in v {
                b.extend_from_slice(&x.to_le_bytes());
            }
        }
        b.extend(m.charge.iter().map(|&c| c as u8));
        b.extend_from_slice(&m.qreason);
        for x in &m.ms2_file {
            b.extend_from_slice(&x.to_le_bytes());
        }
        b
    }
}
