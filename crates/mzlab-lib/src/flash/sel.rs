//! Which spectra a search may touch.

use std::ops::Range;

/// Membership test of the spectrum ranges of a search, decided once per search instead of once per index entry.
pub(super) enum Sel<'a> {
    All,
    One(u32, u32),
    Many(&'a [Range<u32>]),
}

impl<'a> Sel<'a> {
    pub(super) fn new(r: &'a [Range<u32>], n_spectra: u32) -> Sel<'a> {
        match r {
            [x] if x.start == 0 && x.end >= n_spectra => Sel::All,
            [x] => Sel::One(x.start, x.end),
            _ if r.iter().any(|x| x.start == 0 && x.end >= n_spectra) => Sel::All,
            _ => Sel::Many(r),
        }
    }

    #[inline]
    pub(super) fn has(&self, i: u32) -> bool {
        match *self {
            Sel::All => true,
            Sel::One(a, b) => a <= i && i < b,
            Sel::Many(r) => r.iter().any(|x| x.contains(&i)),
        }
    }
}
