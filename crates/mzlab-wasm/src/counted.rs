//! The reader of a dataset that counts the scans it decodes for the page (`build_mzidx_native` in `natives.rs`).
use js_sys::Function;
use mzlab_core::lettori::{self, FileSet, Limits, Reader};
use mzlab_core::wiff;
use wasm_bindgen::JsValue;

/// The reader of a dataset that tells `step(done, total)` how many scans of sample `c` it has decoded, about once per 1 % of them: the page shows how far the index
/// of the sample has got by its own scans, not by the bytes of the members (a `.wiff.scan` holds every sample, the open reads the heads of all of them).
/// A later pass of the builder counts on from where the first one ended; the page counts from the start of each phase.
struct Counting {
    l: Box<dyn Reader>,
    step: Function,
    c: usize,
    n: u32,
    done: u32,
    told: u32,
}

impl Counting {
    fn new(mut l: Box<dyn Reader>, step: Function, c: usize) -> Counting {
        let n = l.index(c).map_or(0, |v| v.len() as u32);
        drop(step.call2(&JsValue::NULL, &0.into(), &n.into()));
        Counting {
            l,
            step,
            c,
            n,
            done: 0,
            told: 0,
        }
    }
    fn count(&mut self, c: usize) {
        if c != self.c {
            return;
        }
        self.done += 1;
        if self.done - self.told >= (self.n / 100).max(1) {
            self.told = self.done;
            drop(
                self.step
                    .call2(&JsValue::NULL, &self.done.into(), &self.n.into()),
            );
        }
    }
}

#[rustfmt::skip]
impl Reader for Counting {
    fn format(&self) -> lettori::Format { self.l.format() }
    fn samples(&self) -> &[lettori::Sample] { self.l.samples() }
    fn unindexed(&self) -> (usize, usize) { self.l.unindexed() }
    fn index(&mut self, c: usize) -> mzlab_core::Result<&[lettori::ScanEntry]> { self.l.index(c) }
    fn index_without_tic(&mut self, c: usize) -> mzlab_core::Result<&[lettori::ScanEntry]> { self.l.index_without_tic(c) }
    fn scan(&mut self, c: usize, k: usize) -> mzlab_core::Result<mzlab_core::mzml::Spectrum> { let s = self.l.scan(c, k); self.count(c); s }
    fn scan_data(&mut self, c: usize, k: usize) -> mzlab_core::Result<mzlab_core::mzml::Spectrum> { let s = self.l.scan_data(c, k); self.count(c); s }
    fn scan_header(&mut self, c: usize, k: usize) -> mzlab_core::Result<Vec<lettori::Field>> { self.l.scan_header(c, k) }
    fn chromatograms(&mut self, c: usize) -> mzlab_core::Result<Vec<lettori::ChromatogramEntry>> { self.l.chromatograms(c) }
    fn chromatogram(&mut self, c: usize, id: &str) -> mzlab_core::Result<mzlab_core::mzml::Chromatogram> { self.l.chromatogram(c, id) }
    fn method(&mut self, c: usize) -> mzlab_core::Result<String> { self.l.method(c) }
    fn missing(&mut self, c: usize) -> usize { self.l.missing(c) }
    fn mzml_header(&mut self, c: usize) -> mzlab_core::Result<Option<mzlab_core::mzmlwrite::Header>> { self.l.mzml_header(c) }
    fn extra_mzml(&self, s: &mzlab_core::mzml::Spectrum) -> mzlab_core::mzmlwrite::SpectrumExtra { self.l.extra_mzml(s) }
    fn scan_extra(&mut self, c: usize, k: usize, s: &mzlab_core::mzml::Spectrum) -> mzlab_core::mzmlwrite::SpectrumExtra { self.l.scan_extra(c, k, s) }
}

/// The reader of the dataset for the index of sample `c`: phase 4 (`tell`) begins, the dataset opens (a SCIEX pair reads only the blocks of sample `c`, `wiff::lettore::apri_solo`:
/// a container may hold dozens of samples, one builder each; any other set with `open_fs`), and it counts its scans for `step` when the page gave one.
pub(crate) fn open(
    mut fs: FileSet,
    open_fs: impl FnOnce(FileSet) -> mzlab_core::Result<Box<dyn Reader>>,
    tell: &dyn Fn(u32),
    step: Option<Function>,
    c: usize,
) -> Result<Box<dyn Reader>, JsValue> {
    tell(4);
    let l = if lettori::riconosci(&mut fs).is_some_and(|v| v.id == wiff::lettore::ENTRY.id) {
        wiff::lettore::open_only(fs, &Limits::sito(), Some(c))
    } else {
        open_fs(fs)
    }
    .map_err(|e| crate::error_json(&e))?;
    Ok(match step {
        Some(step) => Box::new(Counting::new(l, step, c)),
        None => l,
    })
}
