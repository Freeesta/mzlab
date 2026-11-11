//! Every count and offset of the header checked against a second source before the data are read: the `vsize` of each variable against the lengths of its
//! dimensions, the data of the variables against each other and the header (the classic layout: header, the other variables, then the records), and `numrecs`
//! against the records the bytes of the file hold. A header that disagrees is `err.file.cdfDamaged`, never a file read with other values.

use super::*;

/// The `vsize` of a variable of 4 GiB or more (the classic format cannot write it).
const VSIZE_HUGE: u64 = 0xFFFF_FFFF;

impl Cdf {
    /// `header`: the byte after the header.
    pub(super) fn counts_agree(&self, header: u64) -> Result<()> {
        let rec_start = self.var.iter().filter(|v| v.record).map(|v| v.begin).min();
        let mut spans = Vec::new();
        for v in &self.var {
            let n = if v.record {
                per_record(&self.dim, v)
            } else {
                self.elements(v)
            };
            let need = n.saturating_mul(v.tipo.width());
            // writers pad `vsize` to 4 bytes, except some for the only record variable
            if v.vsize != VSIZE_HUGE && !(need..=need.saturating_add(3) & !3).contains(&v.vsize) {
                return Err(broken(
                    "the vsize of a variable does not match its dimensions",
                ));
            }
            let end = v.begin.saturating_add(need);
            let records = rec_start.map_or(u64::MAX, |r| {
                if v.record {
                    r.saturating_add(self.recsize)
                } else {
                    r
                }
            });
            if need > 0 && (v.begin < header || end > records) {
                return Err(broken(
                    "the data of a variable overlap the header or the records",
                ));
            }
            spans.extend((need > 0).then_some((v.begin, end)));
        }
        spans.sort_unstable();
        if spans.windows(2).any(|w| w[1].0 < w[0].1) {
            return Err(broken("the data of two variables overlap"));
        }
        if let (Some(start), false) = (rec_start, self.streaming) {
            let held = self.size.saturating_sub(start).checked_div(self.recsize);
            // fewer records than the bytes hold hides scans as if the copy were cut; more is a cut copy, counted by the reader
            if held.is_some_and(|h| h > self.numrecs) {
                return Err(broken("numrecs counts fewer records than the file holds"));
            }
        }
        Ok(())
    }
}
