//! Decoding of the arrays of an mzML on spare cores: the parser thread cuts the file into spectra and hands each one to a helper, then gives the
//! results to the caller in file order (a reorder queue), so `parse` calls its callback with the same items in the same order as without helpers.

use super::{finish_chromatogram, finish_spectrum, Chrom, Item, Mobility, Spec, Spectrum};
use crate::error::Result;
use crate::mzmlarray::Inflate;
use std::io::Read;

/// How `parse` turns the pieces of the file into items.
pub(super) trait Decode {
    fn spectrum(&mut self, s: Spec, f: &mut dyn FnMut(Item)) -> Result<()>;
    fn chromatogram(&mut self, c: Chrom, f: &mut dyn FnMut(Item)) -> Result<()>;
    /// An item that needs no decoding, kept in its place in the order.
    fn item(&mut self, it: Item, f: &mut dyn FnMut(Item)) -> Result<()>;
    /// Everything handed so far has gone to `f`.
    fn finish(&mut self, f: &mut dyn FnMut(Item)) -> Result<()>;
}

/// One thread: the spectrum is decoded where it is cut.
pub(super) struct Plain(pub(super) Inflate);

fn deliver(sp: (Spectrum, Option<Mobility>), f: &mut dyn FnMut(Item)) {
    if let Some(m) = sp.1 {
        f(Item::Mobility(m));
    }
    f(Item::Spectrum(sp.0));
}

impl Decode for Plain {
    fn spectrum(&mut self, s: Spec, f: &mut dyn FnMut(Item)) -> Result<()> {
        deliver(finish_spectrum(s, &mut self.0)?, f);
        Ok(())
    }

    fn chromatogram(&mut self, c: Chrom, f: &mut dyn FnMut(Item)) -> Result<()> {
        f(Item::Chromatogram(finish_chromatogram(c, &mut self.0)?));
        Ok(())
    }

    fn item(&mut self, it: Item, f: &mut dyn FnMut(Item)) -> Result<()> {
        f(it);
        Ok(())
    }

    fn finish(&mut self, _: &mut dyn FnMut(Item)) -> Result<()> {
        Ok(())
    }
}

#[cfg(not(target_family = "wasm"))]
pub(crate) mod pool;

/// `parse` with the decoding done by `out`.
pub(super) fn parse_with<R: Read, D: Decode>(
    reader: R,
    f: &mut dyn FnMut(Item),
    out: &mut D,
) -> Result<()> {
    let r = super::parse_loop(reader, f, out);
    // what is in flight goes to `f` before an error is told, as without helpers
    out.finish(f).and(r)
}
