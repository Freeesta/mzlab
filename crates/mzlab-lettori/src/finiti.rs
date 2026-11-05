//! The guard every reader of the module is opened through (`apri_nudo`): a retention time, TIC, m/z, intensity or chromatogram point that is NaN or infinite is a damaged file
//! (`err.file.nonFinite`, `err.file.nonFiniteTrace`, with the scan or trace and what is wrong), never a number passed on to the index, the charts or the mzML. A finite but odd value (negative RT, RT out
//! of order) is the file's own and goes through.
use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{Campione, Campo, Formato, Lettore, VoceCromatogramma, VoceScansione};
use mzlab_core::mzml::{Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};

pub struct Finiti {
    inner: Box<dyn Lettore>,
    /// The samples whose index has been checked (an index is read once and asked for many times).
    visti: Vec<bool>,
}

impl Finiti {
    pub fn new(inner: Box<dyn Lettore>) -> Self {
        Finiti {
            inner,
            visti: Vec::new(),
        }
    }
}

fn guasto(dove: &str, chi: impl ToString, cosa: &str, v: f64) -> Error {
    let (key, param) = if dove == "scan" {
        ("err.file.nonFinite", "scan")
    } else {
        ("err.file.nonFiniteTrace", "trace")
    };
    Error::new(key)
        .with(param, chi)
        .with("what", cosa)
        .with("value", v)
}

fn spettro(s: Result<Spectrum>, k: usize) -> Result<Spectrum> {
    let s = s?;
    let chi = || {
        if s.id.is_empty() {
            (k + 1).to_string()
        } else {
            s.id.clone()
        }
    };
    if !s.rt.is_finite() {
        return Err(guasto("scan", chi(), "rt", s.rt));
    }
    if let Some(v) = s.mz.iter().find(|x| !x.is_finite()) {
        return Err(guasto("scan", chi(), "mz", *v));
    }
    if let Some(v) = s.intensity.iter().find(|x| !x.is_finite()) {
        return Err(guasto("scan", chi(), "intensity", f64::from(*v)));
    }
    if let Some(v) = s.wide.iter().flatten().find(|x| !x.is_finite()) {
        return Err(guasto("scan", chi(), "intensity", *v));
    }
    Ok(s)
}

impl Lettore for Finiti {
    fn formato(&self) -> Formato {
        self.inner.formato()
    }
    fn campioni(&self) -> &[Campione] {
        self.inner.campioni()
    }
    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        if !self.visti.get(c).copied().unwrap_or(false) {
            for v in self.inner.indice(c)? {
                for (cosa, x) in [("rt", v.rt), ("tic", v.tic)] {
                    if !x.is_finite() {
                        return Err(guasto("scan", &v.id, cosa, x));
                    }
                }
            }
            if self.visti.len() <= c {
                self.visti.resize(c + 1, false);
            }
            self.visti[c] = true;
        }
        self.inner.indice(c)
    }
    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        spettro(self.inner.scansione(c, k), k)
    }
    fn scansione_dati(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        spettro(self.inner.scansione_dati(c, k), k)
    }
    fn scansione_profilo(&mut self, c: usize, k: usize) -> Result<Option<Spectrum>> {
        self.inner
            .scansione_profilo(c, k)?
            .map(|s| spettro(Ok(s), k))
            .transpose()
    }
    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.inner.intestazione(c, k)
    }
    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        self.inner.cromatogrammi(c)
    }
    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        let t = self.inner.cromatogramma(c, id)?;
        if let Some(v) = t.time.iter().find(|x| !x.is_finite()) {
            return Err(guasto("trace", id, "time", *v));
        }
        if let Some(v) = t.intensity.iter().find(|x| !x.is_finite()) {
            return Err(guasto("trace", id, "intensity", *v));
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
    fn dad_dati(&mut self, c: usize) -> Result<Option<mzlab_core::wiff::dad::DadData>> {
        self.inner.dad_dati(c)
    }
    fn metodo(&mut self, c: usize) -> Result<String> {
        self.inner.metodo(c)
    }
    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        self.inner.testata_mzml(c)
    }
    fn testata_tabella(&mut self, c: usize) -> Result<Option<Header>> {
        self.inner.testata_tabella(c)
    }
    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        self.inner.extra_mzml(s)
    }
    fn extra_scansione(&mut self, c: usize, k: usize, s: &Spectrum) -> SpectrumExtra {
        self.inner.extra_scansione(c, k, s)
    }
    fn non_portati(&self) -> &'static [(&'static str, &'static str)] {
        self.inner.non_portati()
    }
    fn mancanti(&mut self, c: usize) -> usize {
        self.inner.mancanti(c)
    }
    #[cfg(feature = "sciex")]
    fn variante(&mut self, c: usize) -> bool {
        self.inner.variante(c)
    }
}
