//! The cache of an index as entries of the one LRU of the engine (`routes::trim_cache`): the per-scan arrays of a section and the cached blocks of the peak arrays.

use crate::idxread::{Index, ReadAt, ScanArrays};

impl<R: ReadAt> Index<R> {
    /// Bytes held by the per-scan arrays and the cached blocks (what `evict_oldest` can give back).
    pub fn cache_bytes(&self) -> usize {
        let a: usize = self
            .arrays
            .iter()
            .flatten()
            .map(|x| {
                (x.rt.len() + x.sumint.len() + x.bp_mz.len() + x.bp_inten.len() + x.scan_ids.len())
                    * 8
            })
            .sum();
        a + self.cache.iter().map(|c| c.3.len()).sum::<usize>()
    }

    /// Tick of the least recently used cached entry (arrays of a section or block), `None` when nothing is cached.
    pub fn oldest_tick(&self) -> Option<u64> {
        let a = self
            .arrays
            .iter()
            .zip(&self.arrays_tick)
            .filter(|(x, _)| x.is_some())
            .map(|(_, t)| *t);
        a.chain(self.cache.iter().map(|c| c.4)).min()
    }

    /// The arrays of section `si` (already read), marked as just used.
    pub(crate) fn touch_arrays(&mut self, si: usize) -> &ScanArrays {
        if let Some(t) = self.arrays_tick.get_mut(si) {
            *t = crate::routes::next_tick();
        }
        self.arrays
            .get(si)
            .and_then(Option::as_ref)
            .expect("arrays read before use")
    }

    /// The cached block at position `at`, marked as just used.
    pub(crate) fn touch_block(&mut self, at: usize) -> &[u8] {
        let c = self.cache.get_mut(at).expect("block cached before use");
        c.4 = crate::routes::next_tick();
        &c.3
    }

    /// Drops the least recently used cached entry; returns false when there was none.
    pub fn evict_oldest(&mut self) -> bool {
        let Some(t) = self.oldest_tick() else {
            return false;
        };
        if let Some(i) = self.cache.iter().position(|c| c.4 == t) {
            self.cache.remove(i);
        } else if let Some((a, _)) = self
            .arrays
            .iter_mut()
            .zip(&self.arrays_tick)
            .find(|(a, at)| a.is_some() && **at == t)
        {
            *a = None;
        }
        true
    }
}
