// Ported from Attributio readers/__init__.py.
//! Readers for the exports of the annotation tools and the standards library. Each returns ONE row per feature keyed by (polarity, mzMine feature id of the
//! merged alignment of that polarity); a missing file gives no rows, never an error (a campaign can be analysed before SIRIUS or GNPS2 are exported).
//! Best-hit selection uses a STABLE sort, so ties are broken by the order of the export file.

pub mod feature_decisions;
pub mod gnps;
pub mod ms2;
pub mod mzmine;
pub mod sirius;
pub mod standards;
mod standards_cfg;

use super::error::Result;
use super::frame::Frame;
use super::io::Io;
use std::cmp::Ordering;

/// A table file read as pandas does, `None` when the file is missing.
pub(crate) fn read_table(io: &Io, path: &str, sep: char) -> Result<Option<Frame>> {
    io.text(path).map(|t| Frame::parse(&t, sep)).transpose()
}

/// Descending order of an optional number with the missing values last (`sort_values(ascending=False, na_position="last")`).
pub(crate) fn desc_na_last(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// The text of a cell by column name; `None` when pandas reads it as missing or the column is absent.
pub(crate) fn text_at(f: &Frame, r: usize, col: &str) -> Option<String> {
    f.col(col).and_then(|k| f.str_na(r, k)).map(String::from)
}

/// The number of a cell by column name (`pd.to_numeric(..., errors="coerce")`); `None` when missing or absent.
pub(crate) fn num_at(f: &Frame, r: usize, col: &str) -> Option<f64> {
    f.col(col).and_then(|k| f.num(r, k))
}
