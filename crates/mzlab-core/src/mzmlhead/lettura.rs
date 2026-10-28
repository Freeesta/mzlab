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
        let expected = c.n.and_then(|n| usize::try_from(n).ok()).unwrap_or(0);
        let mut t = None;
        for (head, data) in arrays(&chunk) {
            if has_accession(&head, "MS:1000595") {
                t = Some(decode(&head, data, expected)?);
                break;
            }
        }
        let Some(mut t) = t else { return Ok(None) };
        if c.n != i64::try_from(t.len()).ok() {
            return Ok(None);
        }
        let list = find(&chunk, b"<binaryDataArrayList", 0).unwrap_or(chunk.len());
        let seconds =
            text(part(&chunk, 0, list)).contains("unitName=\"second\"") || time_in_seconds(&chunk);
        if seconds {
            t.iter_mut().for_each(|v| *v /= 60.0);
        }
        Ok(Some(t))
    }
}
