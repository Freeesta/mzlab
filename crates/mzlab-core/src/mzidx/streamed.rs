//! One pass over an mzML whose bytes arrive in order while they are copied (the site's copy to OPFS): pass 1 of the builder runs on `stream`
//! with the level and polarity of each spectrum and the `hr1` the head and the first MS1 scan suggest (`scantable::HrGuess`); the scan table is asked for at the
//! end (`table`) and checked against what pass 1 saw. A difference or a failure of pass 1: today's build from `again` (the same file, read again), so the
//! bytes are always those of `build_mzml_corriere`.

use std::cell::Cell;
use std::io::{Read, Seek, SeekFrom};

use super::late::Fermabile;
use super::{build_after, build_mzml_corriere, io_error, First, Output, ScanTable, Stats};
use crate::error::Result;
use crate::mzidx_spill::Spill;
use crate::mzml::{self, Corriere, Item, Spectrum};

/// Bytes of the head of the stream read before parsing, for the `hr1` guess (the instrument configurations come before `<run `).
const HEAD: u64 = 1 << 20;

/// The `.mzidx` of the mzML in `stream` (read once, front to back) with the table `table` gives once the stream is read; `again` is the same file
/// for the rounds that decode again and for the fallback. Returns the statistics and whether the one pass held (false = built again from `again`).
pub fn build_mzml_streamed<R: Read, S: Read + Seek>(
    mut stream: R,
    mut again: S,
    table: &mut dyn FnMut() -> Result<ScanTable>,
    out: &mut dyn Output,
    budget: usize,
    mut spill: Option<&mut dyn Spill>,
    c: &mut dyn Corriere,
) -> Result<(Stats, bool)> {
    // the head from `again` (the dropped file, at hand from the start): the stream then goes to the parser as `with_parse` gives it, one copy of the parser in the .wasm
    let mut head = Vec::new();
    (&mut again)
        .take(HEAD)
        .read_to_end(&mut head)
        .map_err(io_error)?;
    let guess = crate::scantable::HrGuess::of_head(&head);
    drop(head);
    let never = Cell::new(false);
    // the guess of the first MS1 scan: only MS1 groups hold `f32` peaks
    let mut hr1: Option<bool> = None;
    let mut seen: Vec<(u8, i8)> = Vec::new();
    // pass 1 borrows the scratch file through `Again`, so that the fallback can still hand it to today's build
    let mut lent = spill.as_deref_mut().map(Again);
    let first = {
        let mut before = First::new(budget, lent.as_mut().map(|s| s as &mut dyn Spill));
        let mut failure = None;
        let parsed = mzml::parse_corriere(
            Fermabile {
                inner: &mut stream,
                stop: &never,
            },
            |item| {
                if let Item::Spectrum(sp) = item
                    && failure.is_none()
                {
                    if sp.level == 1 && hr1.is_none() {
                        let h = guess.hr(&sp.filter);
                        hr1 = Some(h);
                        before.guess(h);
                    }
                    seen.push((sp.level, sp.polarity));
                    failure = before.add(sp.level, sp.polarity, &sp).err();
                }
            },
            c,
        );
        (parsed.is_ok() && failure.is_none()).then_some(before)
    };
    let t = table()?;
    let agree = agrees(&t, hr1, &seen);
    match first {
        Some(before) if agree => {
            let mut pass = |on: &mut dyn FnMut(Spectrum)| {
                again.seek(SeekFrom::Start(0)).map_err(io_error)?;
                mzml::parse_corriere(
                    Fermabile {
                        inner: &mut again,
                        stop: &never,
                    },
                    |item| {
                        if let Item::Spectrum(s) = item {
                            on(s)
                        }
                    },
                    &mut *c,
                )
            };
            build_after(&t, before, &mut pass, out, budget, false, None).map(|st| (st, true))
        }
        other => {
            drop(other);
            again.seek(SeekFrom::Start(0)).map_err(io_error)?;
            build_mzml_corriere(&t, again, out, budget, spill, c).map(|st| (st, false))
        }
    }
}

/// Whether pass 1 saw what the scan table says: the guessed `hr1` (when an MS1 scan came) and the level and polarity of every scan, in order.
fn agrees(t: &ScanTable, hr1: Option<bool>, seen: &[(u8, i8)]) -> bool {
    hr1.is_none_or(|h| h == t.hr1)
        && t.level.len() == seen.len()
        && t.polarity.len() == seen.len()
        && seen
            .iter()
            .zip(t.level.iter().zip(&t.polarity))
            .all(|(&(l, p), (&tl, &tp))| l == tl && p == tp)
}

/// The scratch file lent to pass 1 for a shorter time than the caller's borrow.
struct Again<'s, 'd>(&'s mut (dyn Spill + 'd));

impl Spill for Again<'_, '_> {
    fn put(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        self.0.put(offset, data)
    }
    fn get(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.0.get(offset, buf)
    }
}
