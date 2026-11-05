//! Whole-header and time-axis readers of `Mzml` that stream or decode only what they need (polarity of the header, time array of a chromatogram).

use super::*;

impl<R: ReadAt> Mzml<R> {
    /// Polarity written in the header (used when a file has no scans, e.g. MRM): positive, negative, mixed or unknown.
    pub fn header_polarity(&mut self) -> Result<String> {
        const POS: &[u8] = b"accession=\"MS:1000130\"";
        const NEG: &[u8] = b"accession=\"MS:1000129\"";
        const BLOCK: u64 = 4 << 20;
        let total = self.size.min(MAX_REGION);
        let (mut pos, mut neg) = (false, false);
        let mut buf: Vec<u8> = vec![];
        let mut at = 0u64;
        // Blocks overlap by the longest needle minus one byte, so a match across a block edge is seen; the scan stops at `</run>` or when both are found.
        while at < total && !(pos && neg) {
            let end = (at + BLOCK).min(total);
            buf.extend_from_slice(&self.read(at, end)?);
            at = end;
            let mut lim = buf.len();
            let mut last = false;
            if let Some(e) = find(&buf, b"</run>", 0) {
                (lim, last) = (e, true);
            } else if at == total {
                (lim, last) = (buf.len().saturating_sub(1), true);
            }
            let h = part(&buf, 0, lim);
            pos |= find(h, POS, 0).is_some();
            neg |= find(h, NEG, 0).is_some();
            if last {
                break;
            }
            let keep = (POS.len() - 1).max(5).min(buf.len());
            buf.drain(..buf.len() - keep);
        }
        Ok(match (pos, neg) {
            (true, false) => "positive",
            (false, true) => "negative",
            (true, true) => "mixed",
            _ => "unknown",
        }
        .into())
    }

    /// The time axis (minutes) of a chromatogram, decoding only the time array; `None` when the declared length does not equal the decoded one (the caller then reads both arrays).
    pub fn chromatogram_times(&mut self, c: &Chrom) -> Result<Option<Vec<f64>>> {
        let chunk = self.read(c.start, c.end)?;
        times_of(&chunk, c)
    }

    /// The time axis of each chromatogram of `cs` (in file order), as `chromatogram_times` with the full read as fallback, handed to `each`.
    pub fn chromatograms_times(
        &mut self,
        cs: &[&Chrom],
        mut each: impl FnMut(Vec<f64>),
    ) -> Result<()> {
        self.in_spans(cs, |c, chunk| {
            each(match times_of(chunk, c)? {
                Some(t) => t,
                None => pair_of(chunk, c)?.0,
            });
            Ok(())
        })
    }

    /// (time in minutes, intensity) of each chromatogram of `cs` (in file order), as `chromatogram`, handed to `each`.
    pub fn chromatograms_each(
        &mut self,
        cs: &[&Chrom],
        mut each: impl FnMut(&Chrom, (Vec<f64>, Vec<f64>)),
    ) -> Result<()> {
        self.in_spans(cs, |c, chunk| {
            each(c, pair_of(chunk, c)?);
            Ok(())
        })
    }

    /// Calls `f` with each chromatogram of `cs` and its bytes. Neighbouring chromatograms are read together in spans of at most `SPAN` bytes:
    /// a file of thousands of SRM traces costs a few reads, not one per trace (each read is a Blob slice in the site). A chromatogram longer
    /// than `SPAN`, or out of file order, is a span of its own, so it reads (and refuses) exactly as `read(c.start, c.end)`.
    fn in_spans(
        &mut self,
        cs: &[&Chrom],
        mut f: impl FnMut(&Chrom, &[u8]) -> Result<()>,
    ) -> Result<()> {
        const SPAN: u64 = 16 << 20;
        let mut rest = cs;
        while let Some((first, more)) = rest.split_first() {
            let (a, mut b, mut prev, mut n) = (first.start, first.end, first.end, 1);
            for c in more {
                if c.start < prev || c.end.saturating_sub(a) > SPAN {
                    break;
                }
                b = b.max(c.end);
                prev = c.end;
                n += 1;
            }
            let (group, tail) = rest.split_at_checked(n).unwrap_or((rest, &[]));
            let span = self.read(a, b)?;
            for c in group {
                let lo = usize::try_from(c.start.saturating_sub(a))
                    .map_or(span.len(), |v| v.min(span.len()));
                let hi = usize::try_from(c.end.min(b).saturating_sub(a))
                    .map_or(span.len(), |v| v.clamp(lo, span.len()));
                f(c, span.get(lo..hi).unwrap_or_default())?;
            }
            rest = tail;
        }
        Ok(())
    }
}

/// The time axis of one chromatogram from its bytes (`chunk` = start..end of the element).
fn times_of(chunk: &[u8], c: &Chrom) -> Result<Option<Vec<f64>>> {
    let expected = c.n.and_then(|n| usize::try_from(n).ok()).unwrap_or(0);
    let mut t = None;
    for (head, data) in arrays(chunk) {
        if has_accession(&head, "MS:1000595") {
            t = Some(decode(&head, data, expected)?);
            break;
        }
    }
    let Some(mut t) = t else { return Ok(None) };
    if c.n != i64::try_from(t.len()).ok() {
        return Ok(None);
    }
    let list = find(chunk, b"<binaryDataArrayList", 0).unwrap_or(chunk.len());
    let seconds =
        text(part(chunk, 0, list)).contains("unitName=\"second\"") || time_in_seconds(chunk);
    if seconds {
        t.iter_mut().for_each(|v| *v /= 60.0);
    }
    Ok(Some(t))
}

/// (time in minutes, intensity) of a chromatogram from its bytes (`chunk` = start..end of the element).
pub(super) fn pair_of(chunk: &[u8], c: &Chrom) -> Result<(Vec<f64>, Vec<f64>)> {
    let (mut t, mut y): (Option<Vec<f64>>, Option<Vec<f64>>) = (None, None);
    for (head, data) in arrays(chunk) {
        let a = decode(
            &head,
            data,
            c.n.and_then(|n| usize::try_from(n).ok()).unwrap_or(0),
        )?;
        if head.contains("MS:1000595") {
            t.get_or_insert(a);
        } else {
            y.get_or_insert(a);
        }
    }
    let (mut t, mut y) = (t.unwrap_or_default(), y.unwrap_or_default());
    let n = t.len().min(y.len());
    let list = find(chunk, b"<binaryDataArrayList", 0).unwrap_or(chunk.len());
    let seconds =
        text(part(chunk, 0, list)).contains("unitName=\"second\"") || time_in_seconds(chunk);
    if seconds {
        t.iter_mut().for_each(|v| *v /= 60.0);
    }
    t.truncate(n);
    y.truncate(n);
    crate::mzmlarray::punti_finiti(&c.id, &t, &y).map(|()| (t, y))
}

#[cfg(test)]
#[path = "lettura_tests.rs"]
mod tests;
