//! One source of scans and traces for the routes, whatever the file: `Scansioni` (the peaks and the header fields of a row of the index) and
//! `Tracce` (the chromatograms and the header of the file). Two implementations each: an mzML read by offset (every `ReadAt`, with the rows of an
//! index of version 1; `MzmlTraces`) and one sample of a `Lettore` (`ReaderSource`, with the rows of an index of version 2, `docs/agenti/mzidx.md`).

use crate::error::{Error, Result};
use crate::idxread::{ReadAt, Scan};
use crate::mzml::{ChromKind, Chromatogram};
use crate::mzmlhead::{Chrom, Mzml};
use crate::mzmlwrite::MzmlWriter;
use crate::readers::{ChromatogramEntry, Field, Reader};

/// The scans of a file, one row of its index at a time.
pub trait Scansioni {
    /// The peaks of the scan of row `sc`: m/z and intensity, as many as the shorter array.
    fn spettro(&mut self, sc: &Scan) -> Result<(Vec<f64>, Vec<f64>)>;
    /// The header fields of the scan: in an mzML every `cvParam` / `userParam` with a name (with its accession), from a reader `Lettore::scan_header`.
    fn scan_header(&mut self, sc: &Scan) -> Result<Vec<Field>>;
    /// The ion mobility of the scan (a mobility array or a scan attribute of an mzML), `None` for a scan that has none and for a reader of a vendor file.
    fn mobilita(&mut self, _sc: &Scan) -> Result<Option<crate::mzml::Mobility>> {
        Ok(None)
    }
    /// `spettro` and `mobilita` of the same scan; a source that reads bytes overrides it to read and decode them once.
    fn spettro_mobilita(&mut self, sc: &Scan) -> Result<(Peaks, Option<crate::mzml::Mobility>)> {
        Ok((self.spettro(sc)?, self.mobilita(sc)?))
    }
}

/// An mzML read by offset: the row gives `start..end` of the `<spectrum>`. A reader that stands for a vendor file (`ReadAt::lettore`) gives, for a row
/// of an index of version 2, the peaks of its `Lettore` and the header fields the mzML of `readers_mzml::scrivi` would have (`scan_text`).
impl<M: ReadAt> Scansioni for M {
    fn spettro(&mut self, sc: &Scan) -> Result<(Vec<f64>, Vec<f64>)> {
        Ok(self.spettro_mobilita(sc)?.0)
    }

    fn mobilita(&mut self, sc: &Scan) -> Result<Option<crate::mzml::Mobility>> {
        if sc.k.is_some() && self.reader().is_some() {
            return Ok(None);
        }
        let buf = read_line(self, sc)?;
        let mut found = None;
        crate::mzml::parse(&buf[..], |it| {
            if let crate::mzml::Item::Mobility(m) = it {
                found.get_or_insert(m);
            }
        })?;
        Ok(found)
    }

    fn spettro_mobilita(&mut self, sc: &Scan) -> Result<(Peaks, Option<crate::mzml::Mobility>)> {
        if let (Some(_), Some((l, c))) = (sc.k, self.reader()) {
            return Ok((ReaderSource { l, c }.spettro(sc)?, None));
        }
        spettro_e_mobilita(&read_line(self, sc)?)
    }

    fn scan_header(&mut self, sc: &Scan) -> Result<Vec<Field>> {
        if sc.k.is_some() && self.reader().is_some() {
            return Ok(crate::scaninfo::campi(&scan_text(self, sc)?));
        }
        let head = crate::scaninfo::header(self, sc.start, sc.end)?;
        Ok(crate::scaninfo::campi(&head))
    }
}

type Peaks = (Vec<f64>, Vec<f64>);

/// The bytes `start..end` of the `<spectrum>` of a row.
fn read_line<M: ReadAt + ?Sized>(m: &mut M, sc: &Scan) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; sc.end.saturating_sub(sc.start) as usize];
    m.read_at(sc.start, &mut buf)?;
    Ok(buf)
}

/// The peaks and the ion mobility of the first spectrum in `buf`, from one parse.
fn spettro_e_mobilita(buf: &[u8]) -> Result<(Peaks, Option<crate::mzml::Mobility>)> {
    let (mut found, mut mob) = (None, None);
    crate::mzml::parse(buf, |it| match it {
        crate::mzml::Item::Spectrum(s) => {
            found.get_or_insert(s);
        }
        crate::mzml::Item::Mobility(m) => {
            mob.get_or_insert(m);
        }
        _ => {}
    })?;
    let s = found.ok_or_else(|| Error::new("err.scan.outside"))?;
    let n = s.mz.len().min(s.intensity.len());
    let y = (0..n).map(|i| s.intensity_f64(i)).collect();
    Ok(((s.mz[..n].to_vec(), y), mob))
}

/// The text of the `<spectrum>` of a row up to its arrays, as the mzML has it: read by offset, or written for a scan of a `Lettore` as
/// `readers_mzml::scrivi` writes it (the header only, no arrays). For what is searched in the text of the scans (scan window, injection time).
pub fn scan_text<M: ReadAt>(m: &mut M, sc: &Scan) -> Result<String> {
    if let (Some(k), Some((l, c))) = (sc.k, m.reader()) {
        let s = crate::mzidx_reader::spettro(&mut *l, c, k)?;
        let extra = l.scan_extra(c, k, &s);
        return Ok(crate::mzmlwrite::spectrum_head(k, &s, &extra));
    }
    crate::scaninfo::header(m, sc.start, sc.end)
}

/// The chromatograms of a file and its header.
pub trait Traces {
    fn chromatograms(&mut self) -> Result<Vec<ChromatogramEntry>>;
    /// The first chromatogram with this id.
    fn chromatogram(&mut self, id: &str) -> Result<Chromatogram>;
    /// The text of the file before its first spectrum: the header up to `<run ` and what follows the tag (instrument, sources, run parameters).
    fn file_header(&mut self) -> Result<String>;
    /// The SRM chromatograms with Q1 and Q3, in the order of the file (`srm::leggi`).
    fn srm(&mut self) -> Result<Vec<crate::srm::Trace>> {
        crate::srm::from_entries(self)
    }
}

/// The traces of an mzML; `scans_end` is where its last spectrum ends (0 without spectra), as `Mzml::chromatograms` wants it.
pub struct MzmlTraces<'a, M: ReadAt> {
    pub m: &'a mut Mzml<M>,
    pub scans_end: u64,
    list: Option<Vec<Chrom>>,
}

impl<'a, M: ReadAt> MzmlTraces<'a, M> {
    pub fn new(m: &'a mut Mzml<M>, scans_end: u64) -> Self {
        MzmlTraces {
            m,
            scans_end,
            list: None,
        }
    }

    /// The headers of the chromatograms, read once.
    fn lista(&mut self) -> Result<&[Chrom]> {
        if self.list.is_none() {
            self.list = Some(self.m.chromatograms(self.scans_end)?);
        }
        Ok(self.list.as_deref().unwrap_or_default())
    }
}

/// The kind `mzml::read` gives a chromatogram, from the kind `Mzml::chromatograms` gives it: a SIM ion (`SIM SIC …`, no Q3) is listed with the SRM
/// transitions there (`mzmlhead::chrom_of`), but it is a SIC, as `mzml::read` and the readers say. Shared by every reader of an mzML's chromatogram list.
pub fn kind(c: &Chrom) -> ChromKind {
    if c.id.starts_with("SIM SIC ") && c.q3.is_none() {
        return ChromKind::Sic;
    }
    match c.kind.as_str() {
        "tic" => ChromKind::Tic,
        "bpc" => ChromKind::Bpc,
        "pda" => ChromKind::Pda,
        "pressure" => ChromKind::Pressure,
        "srm" => ChromKind::Srm,
        "analog" => ChromKind::Analog,
        _ if c.id.starts_with("SIC ") || c.id.starts_with("SIM SIC ") => ChromKind::Sic,
        _ => ChromKind::Other,
    }
}

impl<M: ReadAt> Traces for MzmlTraces<'_, M> {
    fn chromatograms(&mut self) -> Result<Vec<ChromatogramEntry>> {
        let list = self.lista()?.to_vec();
        list.iter()
            .map(|c| {
                Ok(ChromatogramEntry {
                    id: c.id.clone(),
                    tipo: kind(c),
                    unita: self.m.chromatogram_unit(c)?,
                })
            })
            .collect()
    }

    fn chromatogram(&mut self, id: &str) -> Result<Chromatogram> {
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

    fn file_header(&mut self) -> Result<String> {
        self.m.info_head()
    }

    /// Straight from the headers: the units of the other chromatograms are not read.
    fn srm(&mut self) -> Result<Vec<crate::srm::Trace>> {
        let list = self.lista()?.to_vec();
        let mut out = Vec::new();
        let srm: Vec<_> = list
            .iter()
            .filter(|c| c.kind == "srm" && c.q1.is_some() && c.q3.is_some())
            .collect();
        self.m.chromatograms_each(&srm, |c, (t, y)| {
            out.push(crate::srm::Trace {
                q1: c.q1.unwrap_or_default(),
                q3: c.q3.unwrap_or_default(),
                ce: c.ce,
                name: c.name.clone(),
                t,
                y,
            });
        })?;
        Ok(out)
    }
}

/// Sample `c` of a reader: the rows of its index (version 2) carry `k`, the number of the scan in `Lettore::index(c)`.
pub struct ReaderSource<'a> {
    pub l: &'a mut dyn Reader,
    pub c: usize,
}

impl ReaderSource<'_> {
    fn k(&self, sc: &Scan) -> Result<usize> {
        sc.k.ok_or_else(|| Error::new("err.scan.outside"))
    }
}

impl Scansioni for ReaderSource<'_> {
    fn spettro(&mut self, sc: &Scan) -> Result<(Vec<f64>, Vec<f64>)> {
        let k = self.k(sc)?;
        let s = crate::mzidx_reader::spettro(&mut *self.l, self.c, k)?;
        let y = (0..s.mz.len()).map(|i| s.intensity_f64(i)).collect();
        Ok((s.mz, y))
    }

    fn scan_header(&mut self, sc: &Scan) -> Result<Vec<Field>> {
        let k = self.k(sc)?;
        self.l.scan_header(self.c, k)
    }
}

impl Traces for ReaderSource<'_> {
    fn chromatograms(&mut self) -> Result<Vec<ChromatogramEntry>> {
        self.l.chromatograms(self.c)
    }

    fn chromatogram(&mut self, id: &str) -> Result<Chromatogram> {
        self.l.chromatogram(self.c, id)
    }

    /// The header `readers_mzml::scrivi` writes for the sample.
    fn file_header(&mut self) -> Result<String> {
        let h = crate::readers_mzml::file_header(&mut *self.l, self.c)?;
        let mut doc = MzmlWriter::new(Vec::new(), &h)?.finish()?;
        let tail = b"</run>\n</mzML>\n".len();
        doc.truncate(doc.len().saturating_sub(tail));
        Ok(String::from_utf8_lossy(&doc).into_owned())
    }
}
