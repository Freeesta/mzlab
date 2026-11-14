//! The memory base of the detection: allocations that refuse with `error.memoryBudget` instead of aborting, and the limits of a slab.
//!
//! It depends only on `mzlab_core::error`, so the modules that only allocate do not depend on `trace` and its peak models.

use mzlab_core::error::{Error, Result};

pub const FLOOR: f64 = 2e4;
pub const MIN_HEIGHT: f64 = 2e5;
pub const SLAB_PEAKS: usize = 1_500_000;
pub const TOUCH_HEIGHT: f64 = 2e6;
/// Bytes of working memory per peak of a slab (m/z, intensity, scan, log m/z, channel, ...).
pub(crate) const BYTES_PER_PEAK: usize = 64;

pub(crate) fn reserve<T>(v: &mut Vec<T>, extra: usize) -> Result<()> {
    v.try_reserve(extra).map_err(|_| Error::memory_budget())
}

/// `vec![value; n]` that refuses with `error.memoryBudget` instead of aborting when the allocation fails.
pub(crate) fn filled<T: Clone>(n: usize, value: T) -> Result<Vec<T>> {
    let mut v = Vec::new();
    reserve(&mut v, n)?;
    v.resize(n, value);
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::filled;

    #[test]
    fn filled_gives_the_vec_or_refuses_with_the_key() {
        assert_eq!(filled(3, 7u8).unwrap(), vec![7, 7, 7]);
        assert!(filled::<f64>(0, 0.0).unwrap().is_empty());
        let e = filled::<u64>(usize::MAX / 16, 0).unwrap_err();
        assert_eq!(e.key, "error.memoryBudget");
    }
}
