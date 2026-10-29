//! The searches of a `LibSet`: identity, analogs and open search over the libraries of a mask, fused into one ranked list.

use super::*;

impl<S: Source> LibSet<S> {
    fn tag<T>(&self, lib: usize, hit: T, index: u32, mask: u16) -> SetHit<T> {
        let global = self.base[lib] as usize + index as usize;
        let (group, libs) = match &self.sidecar {
            Some(s) => (
                s.group.get(global).copied(),
                s.libs.get(global).copied().unwrap_or(0) & mask,
            ),
            None => (None, 1 << lib),
        };
        SetHit {
            lib: lib as u16,
            index,
            hit,
            group,
            libs,
        }
    }

    /// Fuses per-library hits: ordered by `cmp`, then library, then spectrum; one hit per duplicate group (the best), at most `top`.
    fn fuse<T: Clone>(
        &self,
        mut all: Vec<SetHit<T>>,
        top: usize,
        cmp: impl Fn(&T, &T) -> Ordering,
    ) -> Vec<SetHit<T>> {
        all.sort_by(|a, b| {
            cmp(&a.hit, &b.hit)
                .then(a.lib.cmp(&b.lib))
                .then(a.index.cmp(&b.index))
        });
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        all.retain(|h| h.group.is_none_or(|g| seen.insert(g)));
        all.truncate(top);
        all
    }

    fn selected(&self, mask: u16) -> impl Iterator<Item = usize> + '_ {
        (0..self.libs.len()).filter(move |k| mask & (1 << k) != 0)
    }

    /// Identity search in the libraries of the mask: the best `o.top` hits by entropy, a duplicate group once.
    pub fn identity(
        &self,
        q: &Query,
        o: &IdentityOpts,
        frag: FragTol,
        mask: u16,
    ) -> Result<Vec<SetHit<Hit>>> {
        let mut all = Vec::new();
        for k in self.selected(mask) {
            for h in identity_with(&self.libs[k], q, o, frag)? {
                let i = h.index;
                all.push(self.tag(k, h, i, mask));
            }
        }
        Ok(self.fuse(all, o.top.max(1), |a, b| {
            b.entropy
                .total_cmp(&a.entropy)
                .then(b.cosine.total_cmp(&a.cosine))
        }))
    }

    /// Analogue search in the libraries of the mask (accumulators shared), best `o.top` by modified cosine.
    pub fn analogs(&self, q: &Query, o: &AnalogOpts, mask: u16) -> Result<Vec<SetHit<AnalogHit>>> {
        let mut all = Vec::new();
        let mut acc = self.acc.borrow_mut();
        for k in self.selected(mask) {
            for h in analogs_with(&self.libs[k], q, o, &mut acc)? {
                let i = h.index;
                all.push(self.tag(k, h, i, mask));
            }
        }
        Ok(self.fuse(all, o.top.max(1), |a, b| {
            b.mc.cosine.total_cmp(&a.mc.cosine)
        }))
    }

    /// Open / neutral-loss / hybrid search over the whole index of each library of the mask (spectra of polarity `pol`, 0 = any): the best `keep` by score.
    pub fn open(
        &self,
        q: &Query,
        mode: FlashMode,
        frag: FragTol,
        pol: i8,
        keep: usize,
        mask: u16,
    ) -> Result<Vec<SetHit<FlashScore>>> {
        let mut all = Vec::new();
        let mut acc = self.acc.borrow_mut();
        for k in self.selected(mask) {
            let lib = &self.libs[k];
            let ranges = lib.ranges(pol, f64::NEG_INFINITY, f64::INFINITY)?;
            for s in flash_top(lib, q, mode, frag, &ranges, &mut acc, Some(keep), |_| true)? {
                all.push(self.tag(k, s, s.index, mask));
            }
        }
        Ok(self.fuse(all, keep, |a, b| b.score.total_cmp(&a.score)))
    }
}
