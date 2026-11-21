//! `mz_offset` of `features.align`: the m/z offset of each file (a number, or measured on a known ion from the file's index, `mzlab_core::mzcal`) is
//! subtracted from its features' m/z before the grouping, so the table holds corrected m/z, and added back to the windows the gap filling reads raw
//! scans with. It comes before `recenter`, which then works on the corrected m/z.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::mzcal::{Offset, Spec};

use crate::lrscreen::choose;
use crate::recipe::Recipe;
use crate::slabs::Stored;
use crate::store_read::Want;

/// The offset of each of the `n` files (empty when the recipe has none), the measures going to the evidence; `open(k)` is the index of file `k`.
pub(crate) fn resolve<R: ReadAt>(
    n: usize,
    r: &Recipe,
    open: &mut impl FnMut(usize) -> Result<Index<R>>,
) -> Result<Vec<f64>> {
    let spec = r.screen.mz_offset;
    if spec.is_zero() {
        r.screen.offsets.set(Vec::new());
        return Ok(Vec::new());
    }
    let mut all: Vec<Offset> = Vec::with_capacity(n);
    for k in 0..n {
        all.push(match spec {
            Spec::Auto { .. } => spec.resolve(&mut open(k)?, r.detect.polarity)?,
            Spec::Fixed(da) => Offset { da, auto: None },
            Spec::Zero => Offset::default(),
        });
        if let Some(e) = &all[k].auto {
            choose(
                r,
                "mz_offset",
                all[k].da.to_string(),
                format!("file {}: {}", k + 1, e.status),
            );
        }
    }
    if let Some(e) = &r.screen.evidence {
        e.with(|l| l.offsets = all.clone());
    }
    let das: Vec<f64> = all.iter().map(|o| o.da).collect();
    r.screen.offsets.set(das.clone());
    Ok(das)
}

/// `run(src, r, open)` on the files as they are, or with each file's offset subtracted on read (`Shifted`) and `r` carrying them.
pub(crate) fn apply<R: ReadAt, T>(
    src: &mut dyn Stored,
    r: &Recipe,
    open: &mut impl FnMut(usize) -> Result<Index<R>>,
    run: impl FnOnce(&mut dyn Stored, &Recipe, &mut dyn FnMut(usize) -> Result<Index<R>>) -> Result<T>,
) -> Result<T> {
    let offsets = resolve(src.count(), r, open)?;
    if offsets.is_empty() {
        return run(src, r, open);
    }
    run(
        &mut Shifted {
            inner: src,
            offsets,
        },
        r,
        open,
    )
}

/// The window `want` asks for in corrected m/z, as the raw stored rows hold it (offset `d`).
pub(crate) fn raw_want(want: Want, d: f64) -> Want {
    Want {
        mz: want.mz.map(|(a, b)| (a + d, b + d)),
        ..want
    }
}

/// `mz` from raw to corrected m/z (offset `d`).
pub(crate) fn correct(mz: &mut [f64], d: f64) {
    if d != 0.0 {
        mz.iter_mut().for_each(|m| *m -= d);
    }
}

/// The series with each file's m/z read minus its offset (`Stored::shift`).
pub(crate) struct Shifted<'a> {
    pub inner: &'a mut dyn Stored,
    pub offsets: Vec<f64>,
}

impl Stored for Shifted<'_> {
    fn count(&self) -> usize {
        self.inner.count()
    }
    fn bytes(&mut self, k: usize) -> Result<Vec<u8>> {
        self.inner.bytes(k)
    }
    fn weak(&mut self, k: usize) -> Result<Option<Vec<u8>>> {
        self.inner.weak(k)
    }
    fn part(&mut self, k: usize) -> Result<Option<(Box<dyn ReadAt + '_>, u64)>> {
        self.inner.part(k)
    }
    fn shift(&self, k: usize) -> f64 {
        self.offsets.get(k).copied().unwrap_or(0.0)
    }
}
