//! One source of scans and traces for the routes, whatever the file: `Scansioni` (the peaks and the header fields of a row of the index) and
//! `Tracce` (the chromatograms and the header of the file). Two implementations each: an mzML read by offset (every `ReadAt`, with the rows of an
//! index of version 1; `TracceMzml`) and one sample of a `Lettore` (`SorgenteLettore`, with the rows of an index of version 2, `docs/agenti/mzidx.md`).

use crate::error::{Error, Result};
use crate::idxread::{ReadAt, Scan};
use crate::lettori::{Campo, Lettore, VoceCromatogramma};
use crate::mzml::{ChromKind, Chromatogram};
use crate::mzmlhead::{Chrom, Mzml};
use crate::mzmlwrite::MzmlWriter;

/// The scans of a file, one row of its index at a time.
pub trait Scansioni {
    /// The peaks of the scan of row `sc`: m/z and intensity, as many as the shorter array.
    fn spettro(&mut self, sc: &Scan) -> Result<(Vec<f64>, Vec<f64>)>;
    /// The header fields of the scan: in an mzML every `cvParam` / `userParam` with a name (with its accession), from a reader `Lettore::intestazione`.
    fn intestazione(&mut self, sc: &Scan) -> Result<Vec<Campo>>;
}

/// An mzML read by offset: the row gives `start..end` of the `<spectrum>`. A reader that stands for a vendor file (`ReadAt::lettore`) gives, for a row
/// of an index of version 2, the peaks of its `Lettore` and the header fields the mzML of `lettori_mzml::scrivi` would have (`testo_scansione`).
impl<M: ReadAt> Scansioni for M {
    fn spettro(&mut self, sc: &Scan) -> Result<(Vec<f64>, Vec<f64>)> {
        if let (Some(_), Some((l, c))) = (sc.k, self.lettore()) {
            return SorgenteLettore { l, c }.spettro(sc);
        }
        let mut buf = vec![0u8; sc.end.saturating_sub(sc.start) as usize];
        self.read_at(sc.start, &mut buf)?;
        let mut found = None;
        crate::mzml::parse(&buf[..], |it| {
            if let crate::mzml::Item::Spectrum(s) = it {
                found.get_or_insert(s);
            }
        })?;
        let s = found.ok_or_else(|| Error::new("err.scan.outside"))?;
        let n = s.mz.len().min(s.intensity.len());
        Ok((
            s.mz[..n].to_vec(),
            (0..n).map(|i| s.intensity_f64(i)).collect(),
        ))
    }

    fn intestazione(&mut self, sc: &Scan) -> Result<Vec<Campo>> {
        if sc.k.is_some() && self.lettore().is_some() {
            return Ok(crate::scaninfo::campi(&testo_scansione(self, sc)?));
        }
        let head = crate::scaninfo::header(self, sc.start, sc.end)?;
        Ok(crate::scaninfo::campi(&head))
    }
}

/// The text of the `<spectrum>` of a row up to its arrays, as the mzML has it: read by offset, or written for a scan of a `Lettore` as
/// `lettori_mzml::scrivi` writes it (the header only, no arrays). For what is searched in the text of the scans (scan window, injection time).
pub fn testo_scansione<M: ReadAt>(m: &mut M, sc: &Scan) -> Result<String> {
    if let (Some(k), Some((l, c))) = (sc.k, m.lettore()) {
        let s = crate::mzidx_lettore::spettro(&mut *l, c, k)?;
        return Ok(crate::mzmlwrite::spectrum_head(k, &s, &l.extra_mzml(&s)));
    }
    let mut b = vec![0u8; sc.end.saturating_sub(sc.start) as usize];
    m.read_at(sc.start, &mut b)?;
    Ok(String::from_utf8_lossy(&b).into_owned())
}

/// The chromatograms of a file and its header.
pub trait Tracce {
    fn cromatogrammi(&mut self) -> Result<Vec<VoceCromatogramma>>;
    /// The first chromatogram with this id.
    fn cromatogramma(&mut self, id: &str) -> Result<Chromatogram>;
    /// The text of the file before its first spectrum: the header up to `<run ` and what follows the tag (instrument, sources, run parameters).
    fn testata(&mut self) -> Result<String>;
    /// The SRM chromatograms with Q1 and Q3, in the order of the file (`srm::leggi`).
    fn srm(&mut self) -> Result<Vec<crate::srm::Traccia>> {
        crate::srm::da_voci(self)
    }
}

/// The traces of an mzML; `scans_end` is where its last spectrum ends (0 without spectra), as `Mzml::chromatograms` wants it.
pub struct TracceMzml<'a, M: ReadAt> {
    pub m: &'a mut Mzml<M>,
    pub scans_end: u64,
    lista: Option<Vec<Chrom>>,
}

impl<'a, M: ReadAt> TracceMzml<'a, M> {
    pub fn new(m: &'a mut Mzml<M>, scans_end: u64) -> Self {
        TracceMzml {
            m,
            scans_end,
            lista: None,
        }
    }

    /// The headers of the chromatograms, read once.
    fn lista(&mut self) -> Result<&[Chrom]> {
        if self.lista.is_none() {
            self.lista = Some(self.m.chromatograms(self.scans_end)?);
        }
        Ok(self.lista.as_deref().unwrap_or_default())
    }
}

/// The kind `mzml::read` gives a chromatogram, from the kind `Mzml::chromatograms` gives it.
fn kind(c: &Chrom) -> ChromKind {
    match c.kind.as_str() {
        "tic" => ChromKind::Tic,
        "bpc" => ChromKind::Bpc,
        "pda" => ChromKind::Pda,
        "pressure" => ChromKind::Pressure,
        "srm" => ChromKind::Srm,
        "analog" => ChromKind::Analog,
        _ if c.id.starts_with("SIC ") => ChromKind::Sic,
        _ => ChromKind::Other,
    }
}

impl<M: ReadAt> Tracce for TracceMzml<'_, M> {
    fn cromatogrammi(&mut self) -> Result<Vec<VoceCromatogramma>> {
        let list = self.lista()?.to_vec();
        list.iter()
            .map(|c| {
                Ok(VoceCromatogramma {
                    id: c.id.clone(),
                    tipo: kind(c),
                    unita: self.m.chromatogram_unit(c)?,
                })
            })
            .collect()
    }

    fn cromatogramma(&mut self, id: &str) -> Result<Chromatogram> {
        let c = self
            .lista()?
            .iter()
            .find(|c| c.id == id)
            .cloned()
            .ok_or_else(|| Error::new("error.badIndex").with("id", id))?;
        let (time, intensity) = self.m.chromatogram(&c)?;
        Ok(Chromatogram {
            index: c.index,
            id: c.id.clone(),
            kind: kind(&c),
            q1: c.q1,
            q3: c.q3,
            unit: self.m.chromatogram_unit(&c)?,
            time,
            intensity,
        })
    }

    fn testata(&mut self) -> Result<String> {
        self.m.info_head()
    }

    /// Straight from the headers: the units of the other chromatograms are not read.
    fn srm(&mut self) -> Result<Vec<crate::srm::Traccia>> {
        let list = self.lista()?.to_vec();
        let mut out = Vec::new();
        for c in list.iter().filter(|c| c.kind == "srm") {
            let (Some(q1), Some(q3)) = (c.q1, c.q3) else {
                continue;
            };
            let (t, y) = self.m.chromatogram(c)?;
            out.push(crate::srm::Traccia {
                q1,
                q3,
                ce: c.ce,
                name: c.name.clone(),
                t,
                y,
            });
        }
        Ok(out)
    }
}

/// Sample `c` of a reader: the rows of its index (version 2) carry `k`, the number of the scan in `Lettore::indice(c)`.
pub struct SorgenteLettore<'a> {
    pub l: &'a mut dyn Lettore,
    pub c: usize,
}

impl SorgenteLettore<'_> {
    fn k(&self, sc: &Scan) -> Result<usize> {
        sc.k.ok_or_else(|| Error::new("err.scan.outside"))
    }
}

impl Scansioni for SorgenteLettore<'_> {
    fn spettro(&mut self, sc: &Scan) -> Result<(Vec<f64>, Vec<f64>)> {
        let k = self.k(sc)?;
        let s = crate::mzidx_lettore::spettro(&mut *self.l, self.c, k)?;
        let y = (0..s.mz.len()).map(|i| s.intensity_f64(i)).collect();
        Ok((s.mz, y))
    }

    fn intestazione(&mut self, sc: &Scan) -> Result<Vec<Campo>> {
        let k = self.k(sc)?;
        self.l.intestazione(self.c, k)
    }
}

impl Tracce for SorgenteLettore<'_> {
    fn cromatogrammi(&mut self) -> Result<Vec<VoceCromatogramma>> {
        self.l.cromatogrammi(self.c)
    }

    fn cromatogramma(&mut self, id: &str) -> Result<Chromatogram> {
        self.l.cromatogramma(self.c, id)
    }

    /// The header `lettori_mzml::scrivi` writes for the sample.
    fn testata(&mut self) -> Result<String> {
        let h = crate::lettori_mzml::testata(&mut *self.l, self.c)?;
        let mut doc = MzmlWriter::new(Vec::new(), &h)?.finish()?;
        let tail = b"</run>\n</mzML>\n".len();
        doc.truncate(doc.len().saturating_sub(tail));
        Ok(String::from_utf8_lossy(&doc).into_owned())
    }
}
