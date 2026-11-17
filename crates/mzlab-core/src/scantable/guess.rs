//! The guess of `Table::hr1` before the scan table exists, from the head of an mzML (its instrument configurations, before `<run `) and the filter
//! string of its first MS1 scan: how a one-pass build (`mzidx::build_mzml_streamed`) holds the MS1 peaks. The table checks the guess at the end.

use super::head_read::run_end;
use super::{find, Head};

/// The analyzers of the head of a file, for the guess.
pub struct HrGuess(Head);

impl HrGuess {
    /// From the first bytes of the file; an empty head when `<run ` is not in them (the guess then rests on the filter strings).
    pub fn of_head(first: &[u8]) -> HrGuess {
        let (head, run) = match find(first, b"<run ") {
            Some(lo) => (&first[..lo], &first[lo..run_end(first, lo)]),
            None => (&first[..0], &first[..0]),
        };
        HrGuess(Head::parse(
            &String::from_utf8_lossy(head),
            &String::from_utf8_lossy(run),
        ))
    }

    /// Whether an MS1 scan with this filter string is read by a high-resolution analyzer (`FTMS`, `TOFMS`), as a row of the table names it.
    pub fn hr(&self, filter: &str) -> bool {
        matches!(self.0.analyzer(filter, None), "FTMS" | "TOFMS")
    }
}
