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
    /// What is counted: spectra (empty), or `CYCLES` for an MRM sample, which has no spectra.
    pub unit: &'static str,
}

impl Losses {
    /// `n` items declared and all read.
    pub const fn complete(n: usize) -> Losses {
        Losses {
            declared: n,
            read: n,
            cause: "",
            unit: "",
        }
    }
    /// `read` items of `declared`, missing for `cause` when fewer.
    pub fn of(declared: usize, read: usize, cause: &'static str) -> Losses {
        let declared = declared.max(read);
        Losses {
            declared,
            read,
            cause: if read < declared { cause } else { "" },
            unit: "",
        }
    }
    /// The same counts, of MRM cycles.
    pub fn in_cycles(self) -> Losses {
        Losses {
            unit: CYCLES,
            ..self
        }
    }
    /// The word of the unit in a line of text.
    pub fn unit_word(&self) -> &'static str {
        if !self.unit.is_empty() {
            "MRM cycles"
        } else {
            "spectra"
        }
    }
    pub fn partial(&self) -> bool {
        self.read < self.declared
    }
    pub fn lost(&self) -> usize {
        self.declared - self.read
    }
    /// The stderr line of a partial read of `file` (cause, file, spectra read of declared), none for a complete one.
    pub fn report(&self, file: &std::path::Path) -> Option<String> {
        let end = if self.cause == TRUNCATED {
            ", the file ends before the rest"
        } else {
            ""
        };
        self.partial().then(|| {
            format!(
                "{}: {}: {} of {} {} read{end}",
                self.cause,
                file.display(),
                self.read,
                self.declared,
                self.unit_word()
            )
        })
    }
    /// `{declared, read, cause}`: what the `source` of an index (`lost`) and the note of a file in the record of a run (`files`) say.
    pub fn note(&self) -> Value {
        Value::Obj(vec![
            ("declared".into(), Value::Num(self.declared as f64)),
            ("read".into(), Value::Num(self.read as f64)),
            ("cause".into(), Value::Str(self.cause.into())),
        ])
    }
}

/// The data of a read with what it lost.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadOutcome<T> {
    pub data: T,
    pub losses: Losses,
}

/// `Losses::unit` of an MRM sample: its blocks are cycles of transitions, not spectra.
pub const CYCLES: &str = "cycles";

/// Cause of the scans an index lists beyond the end of its data file (`Reader::missing`).
pub const TRUNCATED: &str = "warn.file.truncated";

/// Cause of the scan blocks of a `.wiff.scan` whose head disagrees with the index row (`wiff::Record::damaged`): left out, never read as other scans.
pub const DAMAGED: &str = "warn.wiff.damaged";

/// Cause of the scans of a Thermo `.raw` whose packet cannot be read (an index entry past the end of the file, a packet that does not decode): each keeps
/// its row with no points, the other scans are read.
pub const RAW_DAMAGED: &str = "warn.raw.damaged";

/// Cause of the scans a run lists without their scan event: the table of events was refused, the scans are read with level and polarity unknown.
pub const EVENTS_REFUSED: &str = "warn.raw.eventsRefused";

/// Cause of the scans of a Waters scanning function whose `_FUNCnnn.DAT` is not in the folder.
pub const MEMBER_MISSING: &str = "warn.file.memberMissing";

/// Cause of the scans of a Waters scanning function whose DAT record width is none of the decoded ones.
pub const RECORD_WIDTH: &str = "warn.file.recordWidth";

/// Cause of the Waters scans left out as damaged: a record no acquisition writes, an index that does not match its DAT, a function the list or the
/// calibration lines of `_HEADER.TXT` lost.
pub const WATERS_DAMAGED: &str = "warn.waters.damaged";

/// The cause keys a reader may give: a key that crosses a boundary as text (the readers module of the site) is given back as its constant.
pub fn cause_key(s: &str) -> Option<&'static str> {
    [
        TRUNCATED,
        DAMAGED,
        EVENTS_REFUSED,
        RAW_DAMAGED,
        MEMBER_MISSING,
        RECORD_WIDTH,
        WATERS_DAMAGED,
    ]
    .into_iter()
    .find(|k| *k == s)
}

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
