//! The guard every reader is opened through (`readers::apri` of the engine, `open_bare` of mzlab-lettori, the single-sample `.wiff` opens of both
//! modules): a retention time, TIC, m/z, intensity or chromatogram point that is NaN or infinite is a damaged file
//! (`err.file.nonFinite`, `err.file.nonFiniteTrace`, with the scan or trace and what is wrong), never a number passed on to the index, the charts or the mzML. A finite but odd value (negative RT, RT out
//! of order) is the file's own and goes through. A scan whose data do not decode is noted and counted in `losses` (`readers::Skips` leaves it out).
use crate::error::{Error, Result};
use crate::mzml::{Chromatogram, Spectrum};
use crate::mzmlwrite::{Header, SpectrumExtra};
use crate::readers::{
    damages_one, ChromatogramEntry, Field, FileSet, Format, Limits, Losses, Reader, Sample,
    ScanEntry, SPECTRUM_DAMAGED,
};
use std::collections::BTreeSet;

pub struct Finished {
    inner: Box<dyn Reader>,
    /// The samples whose index has been checked (an index is read once and asked for many times).
    seen: Vec<bool>,
    /// (sample, scan) whose data did not decode (`readers::damages_one`): left out of a whole-sample read, counted in `losses`.
    damaged: BTreeSet<(usize, usize)>,
}

impl Finished {
    pub fn new(inner: Box<dyn Reader>) -> Self {
        Finished {
            inner,
            seen: Vec::new(),
            damaged: BTreeSet::new(),
        }
    }
    /// `s` as it is, noting scan `k` of sample `c` when its data do not decode.
    fn noted<T>(&mut self, (c, k): (usize, usize), s: Result<T>) -> Result<T> {
        if let Err(e) = &s
            && damages_one(e)
        {
            self.damaged.insert((c, k));
        }
        s
    }
}

/// A SCIEX pair opened for sample `c` only (`wiff::reader::open_only`), behind the guard: the index worker of the site's engine and of the readers
/// module open a `.wiff` this way, the registry (`readers::apri`, `open_bare`) for every sample.
pub fn wiff_sample(fs: FileSet, limits: &Limits, c: usize) -> Result<Box<dyn Reader>> {
    let l = crate::wiff::reader::open_only(fs, limits, Some(c))?;
    Ok(Box::new(Finished::new(l)))
}

fn broken(location: &str, who: impl ToString, what: &str, v: f64) -> Error {
    let (key, param) = if location == "scan" {
        ("err.file.nonFinite", "scan")
    } else {
        ("err.file.nonFiniteTrace", "trace")
    };
    Error::new(key)
        .with(param, who)
        .with("what", what)
        .with("value", v)
}

fn spectrum(s: Result<Spectrum>, k: usize) -> Result<Spectrum> {
    let s = s?;
    let who = || {
        if s.id.is_empty() {
            (k + 1).to_string()
        } else {
            s.id.clone()
        }
    };
    if !s.rt.is_finite() {
        return Err(broken("scan", who(), "rt", s.rt));
    }
    if let Some(v) = s.mz.iter().find(|x| !x.is_finite()) {
        return Err(broken("scan", who(), "mz", *v));
    }
    if let Some(v) = s.intensity.iter().find(|x| !x.is_finite()) {
        return Err(broken("scan", who(), "intensity", f64::from(*v)));
    }
    if let Some(v) = s.wide.iter().flatten().find(|x| !x.is_finite()) {
        return Err(broken("scan", who(), "intensity", *v));
    }
    Ok(s)
}

impl Reader for Finished {
    fn format(&self) -> Format {
        self.inner.format()
    }
    fn samples(&self) -> &[Sample] {
        self.inner.samples()
    }
    fn index(&mut self, c: usize) -> Result<&[ScanEntry]> {
        if !self.seen.get(c).copied().unwrap_or(false) {
            for v in self.inner.index(c)? {
                for (what, x) in [("rt", v.rt), ("tic", v.tic)] {
                    if !x.is_finite() {
                        return Err(broken("scan", &v.id, what, x));
                    }
                }
            }
            if self.seen.len() <= c {
                self.seen.resize(c + 1, false);
            }
            self.seen[c] = true;
        }
        self.inner.index(c)
    }
    // the same check on the rows the reader gives; `visti` stays unset, the full rows of `indice` are checked when asked for
    fn index_without_tic(&mut self, c: usize) -> Result<&[ScanEntry]> {
        if !self.seen.get(c).copied().unwrap_or(false) {
            for v in self.inner.index_without_tic(c)? {
                for (what, x) in [("rt", v.rt), ("tic", v.tic)] {
                    if !x.is_finite() {
                        return Err(broken("scan", &v.id, what, x));
                    }
                }
            }
        }
        self.inner.index_without_tic(c)
    }
    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let s = spectrum(self.inner.scan(c, k), k);
        self.noted((c, k), s)
    }
    fn scan_data(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let s = spectrum(self.inner.scan_data(c, k), k);
        self.noted((c, k), s)
    }
    fn scan_profile(&mut self, c: usize, k: usize) -> Result<Option<Spectrum>> {
        let s = self
            .inner
            .scan_profile(c, k)
            .and_then(|s| s.map(|s| spectrum(Ok(s), k)).transpose());
        self.noted((c, k), s)
    }
    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Field>> {
        self.inner.scan_header(c, k)
    }
    fn chromatograms(&mut self, c: usize) -> Result<Vec<ChromatogramEntry>> {
        self.inner.chromatograms(c)
    }
    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        let t = self.inner.chromatogram(c, id)?;
        if let Some(v) = t.time.iter().find(|x| !x.is_finite()) {
            return Err(broken("trace", id, "time", *v));
        }
        if let Some(v) = t.intensity.iter().find(|x| !x.is_finite()) {
            return Err(broken("trace", id, "intensity", *v));
        }
        Ok(t)
    }
    fn dad_map(&mut self, c: usize) -> Result<Option<String>> {
        self.inner.dad_map(c)
    }
    fn dad_spectrum(&mut self, c: usize, rt0: f64, rt1: f64) -> Result<Option<String>> {
        self.inner.dad_spectrum(c, rt0, rt1)
    }
    fn dad_wavelength(&mut self, c: usize, nm0: f64, nm1: f64) -> Result<Option<String>> {
        self.inner.dad_wavelength(c, nm0, nm1)
    }
    #[cfg(any(not(target_family = "wasm"), feature = "sciex"))]
    fn dad_data(&mut self, c: usize) -> Result<Option<crate::wiff::dad::DadData>> {
        self.inner.dad_data(c)
    }
    fn method(&mut self, c: usize) -> Result<String> {
        self.inner.method(c)
    }
    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> {
        self.inner.mzml_header(c)
    }
    fn table_header(&mut self, c: usize) -> Result<Option<Header>> {
        self.inner.table_header(c)
    }
    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        self.inner.extra_mzml(s)
    }
    fn scan_extra(&mut self, c: usize, k: usize, s: &Spectrum) -> SpectrumExtra {
        self.inner.scan_extra(c, k, s)
    }
    fn not_carried(&self) -> &'static [(&'static str, &'static str)] {
        self.inner.not_carried()
    }
    #[rustfmt::skip]    fn unindexed(&self) -> (usize, usize) { self.inner.unindexed() }
    fn missing(&mut self, c: usize) -> usize {
        self.inner.missing(c)
    }
    fn refused(&mut self, c: usize) -> Option<crate::Error> {
        self.inner.refused(c)
    }
    // the spectra left out because their data do not decode count as not read; a cut file keeps its own cause
    fn losses(&mut self, c: usize) -> Result<Losses> {
        let l = self.inner.losses(c)?;
        let bad = self.damaged.range((c, 0)..(c + 1, 0)).count();
        if bad == 0 {
            return Ok(l);
        }
        let cause = if l.partial() {
            l.cause
        } else {
            SPECTRUM_DAMAGED
        };
        Ok(Losses::of(l.declared, l.read.saturating_sub(bad), cause))
    }
    #[cfg(feature = "sciex")]
    fn variant(&mut self, c: usize) -> bool {
        self.inner.variant(c)
    }
}
