//! The guard every reader of the module is opened through (`open_bare`): a retention time, TIC, m/z, intensity or chromatogram point that is NaN or infinite is a damaged file
//! (`err.file.nonFinite`, `err.file.nonFiniteTrace`, with the scan or trace and what is wrong), never a number passed on to the index, the charts or the mzML. A finite but odd value (negative RT, RT out
//! of order) is the file's own and goes through.
use mzlab_core::error::{Error, Result};
use mzlab_core::mzml::{Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};
use mzlab_core::readers::{ChromatogramEntry, Field, Format, Losses, Reader, Sample, ScanEntry};

pub struct Finished {
    inner: Box<dyn Reader>,
    /// The samples whose index has been checked (an index is read once and asked for many times).
    seen: Vec<bool>,
}

impl Finished {
    pub fn new(inner: Box<dyn Reader>) -> Self {
        Finished {
            inner,
            seen: Vec::new(),
        }
    }
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
        spectrum(self.inner.scan(c, k), k)
    }
    fn scan_data(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        spectrum(self.inner.scan_data(c, k), k)
    }
    fn scan_profile(&mut self, c: usize, k: usize) -> Result<Option<Spectrum>> {
        self.inner
            .scan_profile(c, k)?
            .map(|s| spectrum(Ok(s), k))
            .transpose()
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
    fn dad_data(&mut self, c: usize) -> Result<Option<mzlab_core::wiff::dad::DadData>> {
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
    fn losses(&mut self, c: usize) -> Result<Losses> {
        self.inner.losses(c)
    }
    #[cfg(feature = "sciex")]
    fn variant(&mut self, c: usize) -> bool {
        self.inner.variant(c)
    }
}
