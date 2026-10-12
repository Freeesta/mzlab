//! Table of the features of a series aligned across files.

pub use crate::aligned_meta::{RowMeta, RowQ, NO_FILE};
pub use crate::rowident::RowIdent;

/// Cell states of `Aligned::state` (one per row and file; never read `area > 0` as presence).
/// A peak was found in the file by the detection.
pub const CELL_DETECTED: u8 = 0;
/// Found by the targeted pass (reserved).
pub const CELL_TARGETED: u8 = 1;
/// Area re-extracted by forced integration where the detection found nothing.
pub const CELL_FILLED: u8 = 2;
/// Below the threshold: the file has no peak for the row.
pub const CELL_ABSENT: u8 = 3;

/// Features of a series aligned across files: one row per group, cells are `row * n_files + file`.
/// `apex_rt`/`width` are NaN where the file has no detected peak; `width` = right - left (minutes).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Aligned {
    pub n_files: usize,
    pub mz: Vec<f64>,
    pub rt: Vec<f64>,
    pub area: Vec<f64>,
    pub height: Vec<f32>,
    pub apex_rt: Vec<f32>,
    pub width: Vec<f32>,
    pub state: Vec<u8>,
    /// "Per family" mode only: the ions of each row, `member_start[row]..member_start[row + 1]` (empty in "per ion" mode). The first member of a
    /// row is its reference ion, the one whose cells the row holds; `member_role` = most frequent `ROLE_*` of the ion over the files.
    pub member_start: Vec<u32>,
    pub member_mz: Vec<f64>,
    pub member_rt: Vec<f32>,
    pub member_role: Vec<u8>,
    /// The summaries of the rows (`rowmeta::compute`); empty when not computed.
    pub meta: RowMeta,
    /// Neutral mass, adduct and source of each row (`RowIdent::compute`, after the last sort); empty when not computed.
    pub ident: RowIdent,
    /// The view of the transformation products (`tp::build`); empty when not computed.
    pub tp: crate::tp::TpBlock,
}

impl Aligned {
    /// An empty table with room for `rows` rows of `n_files` cells (no reallocation while rows are pushed).
    pub fn with_capacity(n_files: usize, rows: usize) -> Aligned {
        let cells = rows * n_files;
        Aligned {
            n_files,
            mz: Vec::with_capacity(rows),
            rt: Vec::with_capacity(rows),
            area: Vec::with_capacity(cells),
            height: Vec::with_capacity(cells),
            apex_rt: Vec::with_capacity(cells),
            width: Vec::with_capacity(cells),
            state: Vec::with_capacity(cells),
            ..Aligned::default()
        }
    }

    pub fn len(&self) -> usize {
        self.mz.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mz.is_empty()
    }

    /// Index of the cell of `row` in `file`.
    pub fn cell(&self, row: usize, file: usize) -> usize {
        row * self.n_files + file
    }

    /// Members of `row` as indices into the `member_*` columns (just the row itself in "per ion" mode: empty range).
    pub fn members(&self, row: usize) -> std::ops::Range<usize> {
        if self.member_start.is_empty() {
            return 0..0;
        }
        self.member_start[row] as usize..self.member_start[row + 1] as usize
    }

    /// Bytes of the heap held by the columns.
    pub fn heap_bytes(&self) -> usize {
        self.mz.capacity() * 8
            + self.rt.capacity() * 8
            + self.area.capacity() * 8
            + (self.height.capacity() + self.apex_rt.capacity() + self.width.capacity()) * 4
            + self.state.capacity()
            + self.member_start.capacity() * 4
            + self.member_mz.capacity() * 8
            + self.member_rt.capacity() * 4
            + self.member_role.capacity()
            + self.meta.heap_bytes()
            + self.ident.heap_bytes()
            + self.tp.heap_bytes()
    }
}
