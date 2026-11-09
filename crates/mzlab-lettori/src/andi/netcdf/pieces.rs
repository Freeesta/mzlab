//! Reading the bytes of a variable of a classic NetCDF file in windows: record variables interleaved with the others are read one window at a time, not one block per record.

use super::*;

impl Cdf {
    /// The bytes of elements `da..da + n` of `v`.
    pub(super) fn raw_values(
        &self,
        data: &mut dyn ReadAt,
        limits: &Limits,
        v: &Var,
        da: u64,
        n: u64,
    ) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let total = self.check(limits, v, da, n)?;
        out.try_reserve_exact(total)
            .map_err(|_| Error::memory_budget())?;
        self.to_pieces(data, v, da, n, |b| out.extend_from_slice(b))?;
        Ok(out)
    }

    /// The window `da..da + n` lies inside `v` and its bytes fit the budget: the number of bytes.
    pub(super) fn check(&self, limits: &Limits, v: &Var, da: u64, n: u64) -> Result<usize> {
        let fine = da.checked_add(n).ok_or_else(Error::memory_budget)?;
        if fine > self.elements(v) {
            return Err(broken("read past the end of a variable"));
        }
        let total = n
            .checked_mul(v.tipo.width())
            .filter(|&t| t <= limits.byte)
            .ok_or_else(Error::memory_budget)?;
        usize::try_from(total).map_err(|_| Error::memory_budget())
    }

    /// Hands `f` the bytes of elements `da..da + n` of `v`, in order and in pieces of whole elements. A variable that is not a record one is read in pieces of `PEZZO` bytes. A record
    /// variable is spread over the records (`epr` elements in each, one record every `recsize` bytes): when its elements are not far apart the records of a piece are read at once, as
    /// one window of the file, and its elements picked out of it; otherwise one block per record is read, as the format lays it out.
    pub(super) fn to_pieces(
        &self,
        data: &mut dyn ReadAt,
        v: &Var,
        da: u64,
        n: u64,
        mut f: impl FnMut(&[u8]),
    ) -> Result<()> {
        let w = v.tipo.width();
        let fine = da.saturating_add(n);
        let epr = if v.record {
            per_record(&self.dim, v).max(1)
        } else {
            u64::MAX
        };
        let (rs, block_b) = (self.recsize, epr.saturating_mul(w));
        let window = v.record && block_b <= rs && rs <= PIECE as u64 && rs <= 16 * block_b;
        let mut buf = Vec::new();
        let mut gat = Vec::new();
        let read = |data: &mut dyn ReadAt, pos: u64, len: usize, buf: &mut Vec<u8>| -> Result<()> {
            if pos.checked_add(len as u64).is_none_or(|f| f > self.size) {
                return Err(broken("variable data past the end of the file"));
            }
            buf.clear();
            buf.resize(len, 0);
            data.read_at(pos, buf)
        };
        let mut e = da;
        while e < fine {
            if window {
                let (r0, i0) = (e / epr, e % epr);
                // the records of this piece (as many as fit in `PEZZO` bytes, not past the last element asked for) and where its last element ends
                let r1 = ((fine - 1) / epr).min(r0 + (PIECE as u64 / rs).max(1) - 1);
                let e_fine = fine.min((r1 + 1) * epr);
                let fine_r = (e_fine - 1) % epr + 1;
                let start = v.begin.saturating_add(r0 * rs).saturating_add(i0 * w);
                let len = ((r1 - r0) * rs + fine_r * w - i0 * w) as usize;
                read(data, start, len, &mut buf)?;
                if rs == block_b {
                    f(&buf);
                } else {
                    gat.clear();
                    for r in r0..=r1 {
                        let (a, b) = (
                            if r == r0 { i0 } else { 0 },
                            if r == r1 { fine_r } else { epr },
                        );
                        let off = ((r - r0) * rs + a * w - i0 * w) as usize;
                        gat.extend_from_slice(&buf[off..off + ((b - a) * w) as usize]);
                    }
                    f(&gat);
                }
                e = e_fine;
                continue;
            }
            let (pos, block) = if v.record {
                let (r, i) = (e / epr, e % epr);
                (
                    v.begin
                        .saturating_add(r.saturating_mul(rs))
                        .saturating_add(i * w),
                    (epr - i).min(fine - e),
                )
            } else {
                (v.begin.saturating_add(e * w), fine - e)
            };
            let blocco = block.min((PIECE as u64 / w).max(1));
            read(data, pos, (blocco * w) as usize, &mut buf)?;
            f(&buf);
            e += blocco;
        }
        Ok(())
    }
}
