//! A reader whose profile scans are reduced to one point per peak (`centroidi`, MS:1000035), as the conversion of a SCIEX `.wiff` with `centroid`
//! does (`wiff::Wiff::to_mzml_progress`): the parameters of each sample come from at most 64 equally spaced scans of its index (`centroids::estimate`).
//! Everything else is the reader's.

use crate::centroids::{self, Buf, Params};
use crate::error::Result;
use crate::mzml::{Chromatogram, Spectrum};
use crate::mzmlwrite::{Header, SpectrumExtra};
use crate::readers::{ChromatogramEntry, Field, Format, Reader, Sample, ScanEntry};

pub struct Centroidato {
    l: Box<dyn Reader>,
    params: Vec<Option<Params>>,
    buf: Buf,
    /// The scan last given by `grezza`, kept for the `scansione` that asks for the same one (the two versions of an index from one decoding).
    last: Option<(usize, usize, Spectrum)>,
}

impl Centroidato {
    pub fn nuovo(l: Box<dyn Reader>) -> Centroidato {
        let n = l.samples().len();
        Centroidato {
            l,
            params: vec![None; n],
            buf: Buf::default(),
            last: None,
        }
    }

    /// The reader underneath: the one of the version with the scans as the file has them (profile stays profile).
    pub fn simple(&mut self) -> &mut dyn Reader {
        &mut *self.l
    }

    /// Scan `k` of sample `c` as the reader gives it, decoded now and kept: the next `scansione(c, k)` centroids this one instead of decoding it again.
    pub fn grezza(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let s = self.l.scan(c, k)?;
        self.last = Some((c, k, s.clone()));
        Ok(s)
    }

    fn params(&mut self, c: usize) -> Result<Params> {
        if let Some(Some(p)) = self.params.get(c) {
            return Ok(p.clone());
        }
        let p = stima(&mut *self.l, c)?.params;
        if let Some(slot) = self.params.get_mut(c) {
            *slot = Some(p.clone());
        }
        Ok(p)
    }
}

/// The estimate of the peak picking of sample `c` on at most 64 equally spaced scans of its index (`peaks` 0: the width is the fallback).
pub(crate) fn stima(l: &mut dyn Reader, c: usize) -> Result<centroids::Estimate> {
    let len = l.index(c)?.len();
    let n = len.min(64);
    let mut scans = Vec::new();
    for i in 0..n {
        // the estimate is made on the points it will reduce: the profile, not the maker's centroids of a reader that keeps both
        let s = match crate::readers_mzml::profile_of(l, c, i * len / n)? {
            Some(s) => s,
            None => l.scan(c, i * len / n)?,
        };
        let y = s
            .wide
            .unwrap_or_else(|| s.intensity.iter().map(|&v| f64::from(v)).collect());
        scans.push((s.mz, y));
    }
    let refs: Vec<(&[f64], &[f64])> = scans
        .iter()
        .map(|(a, b)| (a.as_slice(), b.as_slice()))
        .collect();
    Ok(centroids::estimate(&refs, 0.7))
}

/// `s` as centroids when it is a profile scan, with the parameters `p`; a scan already centroided stays as it is.
pub(crate) fn picchi(s: &mut Spectrum, p: &Params, buf: &mut Buf) {
    if !s.profile {
        return;
    }
    let y = s
        .wide
        .take()
        .unwrap_or_else(|| s.intensity.iter().map(|&v| f64::from(v)).collect());
    let cs = centroids::centroids(&s.mz, &y, p, buf);
    s.mz = cs.iter().map(|c| c.mz).collect();
    let v: Vec<f64> = cs.iter().map(|c| c.value).collect();
    s.intensity = v.iter().map(|&x| narrow(x)).collect();
    s.wide = Some(v);
    s.profile = false;
}

/// The intensity kept in 32 bits, as the mzML of the conversion writes it.
#[allow(clippy::cast_possible_truncation)]
fn narrow(x: f64) -> f32 {
    x as f32
}

impl Reader for Centroidato {
    fn format(&self) -> Format {
        self.l.format()
    }
    fn samples(&self) -> &[Sample] {
        self.l.samples()
    }
    fn index(&mut self, c: usize) -> Result<&[ScanEntry]> {
        self.l.index(c)
    }
    fn index_without_tic(&mut self, c: usize) -> Result<&[ScanEntry]> {
        self.l.index_without_tic(c)
    }
    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let mut s = match self.last.take() {
            Some((cc, kk, s)) if cc == c && kk == k => s,
            _ => self.l.scan(c, k)?,
        };
        if s.profile {
            let p = self.params(c)?;
            picchi(&mut s, &p, &mut self.buf);
        }
        Ok(s)
    }
    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Field>> {
        self.l.scan_header(c, k)
    }
    fn chromatograms(&mut self, c: usize) -> Result<Vec<ChromatogramEntry>> {
        self.l.chromatograms(c)
    }
    fn missing(&mut self, c: usize) -> usize {
        self.l.missing(c)
    }
    fn losses(&mut self, c: usize) -> Result<crate::readers::Losses> {
        self.l.losses(c)
    }
    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.l.chromatogram(c, id)
    }
    fn method(&mut self, c: usize) -> Result<String> {
        self.l.method(c)
    }
    /// The header of the reader, marked as centroided (the step of the processing the mzML names).
    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> {
        let h = self.l.mzml_header(c)?;
        Ok(h.map(|h| Header {
            centroided: true,
            ..h
        }))
    }
    fn table_header(&mut self, c: usize) -> Result<Option<Header>> {
        let h = self.l.table_header(c)?;
        Ok(h.map(|h| Header {
            centroided: true,
            ..h
        }))
    }
    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        self.l.extra_mzml(s)
    }
    fn scan_extra(&mut self, c: usize, k: usize, s: &Spectrum) -> SpectrumExtra {
        self.l.scan_extra(c, k, s)
    }
}

/// `l` with its profile scans as centroids.
pub fn centroidato(l: Box<dyn Reader>) -> Box<dyn Reader> {
    Box::new(Centroidato::nuovo(l))
}

#[cfg(test)]
mod tests;
