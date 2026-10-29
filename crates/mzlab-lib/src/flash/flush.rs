//! Staging and scoring of the index entries of a Flash search: signatures, conflicts and the entropy sums per bucket.

use super::*;

impl FlashAcc {
    /// Counts one pair; a repeated query or library peak (or a signature collision) marks the spectrum as in conflict.
    #[inline]
    pub(super) fn mark(&mut self, sp: u32, qi: u16, lib_mz: u32) {
        let Some(s) = self.slots.get_mut(sp as usize) else {
            return;
        };
        let c = s.cnt;
        if c == 0 {
            self.touched.push(sp);
        }
        let bq = 1u64 << (qi & 63);
        let bl = 1u64 << ((lib_mz as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 58);
        let clash = (s.used_q & bq != 0) || (s.used_l & bl != 0);
        s.used_q |= bq;
        s.used_l |= bl;
        let n = if c & COUNT == COUNT {
            COUNT
        } else {
            (c & COUNT) + 1
        };
        s.cnt = n | (c & CONFLICT) | if clash || n == COUNT { CONFLICT } else { 0 };
    }

    pub(super) fn reset(&mut self) {
        for &sp in &self.touched {
            if let Some(s) = self.slots.get_mut(sp as usize) {
                *s = Slot::default();
            }
        }
        self.touched.clear();
        for (b, done) in self.buckets.iter_mut().zip(self.done.iter_mut()) {
            b.clear();
            *done = 0;
        }
        self.staged = 0;
        self.retained = 0;
        self.overflow = false;
        self.flushes = 0;
        self.conf.clear();
        self.conf_ready = false;
    }
}
