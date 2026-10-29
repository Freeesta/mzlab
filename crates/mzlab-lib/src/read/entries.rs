//! Reading the Flash index pages: one page through the cache, and the entries of an m/z window.

use super::*;

impl<S: Source> Library<S> {
    pub(super) fn page(&self, nl: bool, p: u32) -> Result<Rc<Vec<u8>>> {
        let kind = if nl { Kind::Nl } else { Kind::Frag };
        if let Some(v) = self.cache.borrow_mut().get((kind, p)) {
            return Ok(v);
        }
        let d = self.index_dir(nl);
        let (_, count, crc) = *d
            .pages
            .get(p as usize)
            .ok_or_else(|| Error::corrupt("page"))?;
        let bytes = read_vec(
            &self.src,
            d.data_off + p as u64 * PAGE_BYTES as u64,
            count as u64 * ENTRY_BYTES as u64,
        )?;
        self.check_crc((kind, p), &bytes, crc)?;
        let v = Rc::new(bytes);
        self.block_reads.set(self.block_reads.get() + 1);
        self.cache.borrow_mut().put((kind, p), v.clone());
        Ok(v)
    }

    /// Calls `f(mz_q, spectrum, intensity_q)` for every entry of an index with `lo <= m/z <= hi` (quantised units), in m/z order.
    pub fn for_each_entry(
        &self,
        nl: bool,
        lo: u32,
        hi: u32,
        mut f: impl FnMut(u32, u32, u16),
    ) -> Result<()> {
        let d = self.index_dir(nl);
        if d.pages.is_empty() || lo > hi {
            return Ok(());
        }
        let first = d.pages.partition_point(|p| p.0 < lo).saturating_sub(1);
        for (p, pg) in d.pages.iter().enumerate().skip(first) {
            if pg.0 > hi {
                break;
            }
            let b = self.page(nl, to_u32(p)?)?;
            let c = pg.1 as usize;
            let (Some(mzs), Some(sps), Some(qis)) =
                (b.get(..c * 4), b.get(c * 4..c * 8), b.get(c * 8..c * 10))
            else {
                return Err(Error::corrupt("index"));
            };
            let (mut start, mut end) = (0, if pg.0 >= lo { 0 } else { c });
            while start < end {
                let mid = (start + end) / 2;
                if le_u32(mzs, mid * 4) < lo {
                    start = mid + 1;
                } else {
                    end = mid;
                }
            }
            let rows = mzs
                .chunks_exact(4)
                .zip(sps.chunks_exact(4))
                .zip(qis.chunks_exact(2))
                .skip(start);
            for ((m, s), qi) in rows {
                let mz = le_u32(m, 0);
                if mz > hi {
                    break;
                }
                f(mz, le_u32(s, 0), le_u16(qi, 0));
            }
        }
        Ok(())
    }
}
