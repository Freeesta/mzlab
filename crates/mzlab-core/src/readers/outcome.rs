//! The account of a read (`Losses`): how many scans, points or chromatograms the file declares and how many were read, with the key of the cause, so
//! that a partial result is never silent: the conversion writes the counts in `conv/1`, the CLI exits with 3, the page shows them.

use super::{ChromatogramEntry, Reader, ScanEntry};
use crate::error::Result;
use crate::json::Value;
use crate::mzml::Spectrum;

/// What a read left out of a sample: how many items (scans, chromatograms, points) the file declares and how many were read, and the key of the cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Losses {
    pub declared: usize,
    pub read: usize,
    /// Catalogue key of why items are missing (`warn.file.truncated`); empty when the read is complete.
    pub cause: &'static str,
}

impl Losses {
    /// `n` items declared and all read.
    pub const fn complete(n: usize) -> Losses {
        Losses {
            declared: n,
            read: n,
            cause: "",
        }
    }
    /// `read` items of `declared`, missing for `cause` when fewer.
    pub fn of(declared: usize, read: usize, cause: &'static str) -> Losses {
        let declared = declared.max(read);
        Losses {
            declared,
            read,
            cause: if read < declared { cause } else { "" },
        }
    }
    pub fn partial(&self) -> bool {
        self.read < self.declared
    }
    pub fn lost(&self) -> usize {
        self.declared - self.read
    }
    /// `{declared, read, cause}`: what the `source` of an index (`lost`) and the note of a file in the record of a run (`files`) say.
    pub fn note(&self) -> Value {
        Value::Obj(vec![
            ("declared".into(), Value::Num(self.declared as f64)),
            ("read".into(), Value::Num(self.read as f64)),
            ("cause".into(), Value::Str(self.cause.into())),
        ])
    }
    /// The losses an index says in `source.lost` (`note`), when its read was partial; a cause this build does not know reads as `TRUNCATED`.
    pub fn of_source(source: &Value) -> Option<Losses> {
        let lost = source.get("lost")?;
        let n = |k: &str| lost.get(k).and_then(Value::f64).map(|v| v as usize);
        let cause = match lost.get("cause").and_then(Value::str) {
            Some(DAMAGED) => DAMAGED,
            Some(EVENTS_REFUSED) => EVENTS_REFUSED,
            _ => TRUNCATED,
        };
        Some(Losses::of(n("declared")?, n("read")?, cause)).filter(Losses::partial)
    }
}

/// The data of a read with what it lost.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadOutcome<T> {
    pub data: T,
    pub losses: Losses,
}

/// Cause of the scans an index lists beyond the end of its data file (`Reader::missing`).
pub const TRUNCATED: &str = "warn.file.truncated";

/// Cause of the scan blocks of a `.wiff.scan` whose head disagrees with the index row (`wiff::Record::damaged`): left out, never read as other scans.
pub const DAMAGED: &str = "warn.wiff.damaged";

/// Cause of the scans a run lists without their scan event: the table of events was refused, the scans are read with level and polarity unknown.
pub const EVENTS_REFUSED: &str = "warn.raw.eventsRefused";

/// `Reader::losses` of a reader that does not override it: the rows of `index_without_tic` plus the scans `missing` counts.
pub(super) fn default_losses<R: Reader + ?Sized>(r: &mut R, c: usize) -> Result<Losses> {
    let lacking = r.missing(c);
    let n = r.index_without_tic(c)?.len();
    Ok(Losses::of(n + lacking, n, TRUNCATED))
}

/// The reads of a `Reader` with what they lost.
pub trait ReadOutcomes {
    /// `index` with the losses of the sample.
    fn index_outcome(&mut self, c: usize) -> Result<ReadOutcome<&[ScanEntry]>>;
    /// `scan` with its points, all read.
    fn scan_outcome(&mut self, c: usize, k: usize) -> Result<ReadOutcome<Spectrum>>;
    /// `chromatograms` with the ones listed, all read.
    fn chromatograms_outcome(&mut self, c: usize) -> Result<ReadOutcome<Vec<ChromatogramEntry>>>;
}

impl<R: Reader + ?Sized> ReadOutcomes for R {
    fn index_outcome(&mut self, c: usize) -> Result<ReadOutcome<&[ScanEntry]>> {
        let losses = self.losses(c)?;
        Ok(ReadOutcome {
            data: self.index(c)?,
            losses,
        })
    }
    fn scan_outcome(&mut self, c: usize, k: usize) -> Result<ReadOutcome<Spectrum>> {
        let data = self.scan(c, k)?;
        Ok(ReadOutcome {
            losses: Losses::complete(data.mz.len()),
            data,
        })
    }
    fn chromatograms_outcome(&mut self, c: usize) -> Result<ReadOutcome<Vec<ChromatogramEntry>>> {
        let data = self.chromatograms(c)?;
        Ok(ReadOutcome {
            losses: Losses::complete(data.len()),
            data,
        })
    }
}
