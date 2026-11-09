//! An mzML whose arrays are MS-Numpress (msconvert `--numpressLinear/--numpressPic/--numpressSlof/--numpressAll`), as a `Lettore`. The engine of the page refuses it
//! (`err.file.numpress`); the module reads it and `readers_mzml::scrivi` writes it again as a plain mzML (zlib) in OPFS (rule R, docs/agenti/lettori-modulo.md).
//! The index comes from one pass over the offsets (`mzlab_core::scantable`, the headers only); `scansione(c, k)` reads the bytes of spectrum `k` and gives them to `mzml::parse`,
//! whose numpress decoding is on in this build (`mzlab_core::numpress`, feature `estesi`). Reference of the codecs: the MS-Numpress documentation; the code is written from it.

use std::cell::RefCell;

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::method::{self, sha1_hex};
use mzlab_core::mzml::{self, ChromKind, Chromatogram, Item, Spectrum};
use mzlab_core::mzmlhead::{Chrom, Mzml};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};
use mzlab_core::readers::{
    ChromatogramEntry, Fiducia, Field, FileSet, Format, Limits, Position, Reader, ReaderEntry,
    Sample, ScanEntry, CONTRATTO,
};
use mzlab_core::scantable::{self, Row};

use super::mzml_header::{extra_di, header_from, param_list};
use super::np_rewrite;

/// The accessions that make an array numpress (the three codecs, alone and followed by zlib).
const ACCESSION: [&str; 6] = [
    "MS:1002312",
    "MS:1002313",
    "MS:1002314",
    "MS:1002746",
    "MS:1002747",
    "MS:1002748",
];
/// The bytes of the start of the file that `riconosci` looks at first; it reads on (doubling) up to the end of the first spectrum, never past `TESTA_MAX`.
const HEAD: usize = 64 << 10;
const MAX_HEAD: usize = 16 << 20;
pub const ENTRY: ReaderEntry = ReaderEntry {
    id: "mzml-numpress",
    contratto: CONTRATTO,
    estensioni: &[".mzml"],
    riconosci: recognize,
    apri,
};

/// A borrowed `ReadAt` (`Mzml` owns its reader).
struct Loan<'a>(&'a mut dyn ReadAt);

impl ReadAt for Loan<'_> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.0.read_at(off, buf)
    }
}

pub struct NumpressReader {
    data: Box<dyn ReadAt>,
    dim: u64,
    name: String,
    limits: Limits,
    campioni: Vec<Sample>,
    rows: Vec<Row>,
    /// Spectra the run header declares (0: not declared), to count those a truncated file lacks.
    declared: usize,
    /// The file does not end with its closing tag: a TIC lost with the cut is built from the whole scans.
    cut: bool,
    indice: Vec<ScanEntry>,
    traces: Option<Vec<Chrom>>,
    /// SHA-1 of the file, read the first time the header of the mzML is asked for (`Some(None)`: it could not be read).
    sha1: Option<Option<String>>,
    /// The extra of the spectrum `scansione` read last (the writer asks for it right after).
    last: RefCell<Option<(usize, SpectrumExtra)>>,
}

impl NumpressReader {
    fn one(&self, c: usize) -> Result<()> {
        (c == 0)
            .then_some(())
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    /// The bytes of spectrum `k`.
    fn byte(&mut self, k: usize) -> Result<Vec<u8>> {
        let r = self
            .rows
            .get(k)
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        let n = r.end - r.start;
        if n > self.limits.byte {
            return Err(Error::memory_budget());
        }
        let mut b = Vec::new();
        b.try_reserve_exact(n as usize)
            .map_err(|_| Error::memory_budget())?;
        b.resize(n as usize, 0);
        self.data.read_at(r.start, &mut b)?;
        Ok(b)
    }

    fn chromatograms_of_the_file(&mut self) -> Result<&[Chrom]> {
        if self.traces.is_none() {
            let fine = self.rows.last().map_or(0, |r| r.end);
            let mut m = Mzml::new(Loan(&mut *self.data), self.dim);
            self.traces = Some(m.chromatograms(fine)?);
        }
        Ok(self.traces.as_deref().unwrap_or_default())
    }

    fn head_xml(&mut self) -> Result<String> {
        Mzml::new(Loan(&mut *self.data), self.dim).run_head(1 << 20)
    }
}

fn field(name: &str, value: impl ToString, units: Option<&str>, cv: Option<&str>) -> Field {
    Field {
        nome: name.into(),
        valore: value.to_string(),
        unita: units.map(str::to_string),
        cv: cv.map(str::to_string),
    }
}

impl NumpressReader {
    /// The chromatograms of the file with their kind and unit.
    fn trace_entries(&mut self) -> Result<Vec<ChromatogramEntry>> {
        let traces = self.chromatograms_of_the_file()?.to_vec();
        let mut out = Vec::new();
        if self.tic_from_the_index(&traces) {
            out.push(ChromatogramEntry {
                id: "TIC".into(),
                tipo: ChromKind::Tic,
                unita: UNITS_TIC.into(),
            });
        }
        let mut m = Mzml::new(Loan(&mut *self.data), self.dim);
        for t in traces {
            out.push(ChromatogramEntry {
                tipo: tipo(&t.kind),
                unita: m.chromatogram_unit(&t)?,
                id: t.id,
            });
        }
        Ok(out)
    }

    /// The chromatogram `id`, read from its bytes with the numpress arrays plain.
    fn trace(&mut self, id: &str) -> Result<Chromatogram> {
        let traces = self.chromatograms_of_the_file()?.to_vec();
        if id == "TIC" && self.tic_from_the_index(&traces) {
            return Ok(self.index_tic());
        }
        let t = traces
            .into_iter()
            .find(|t| t.id == id)
            .ok_or_else(|| Error::new("error.badIndex").with("id", id))?;
        // the bytes of the chromatogram with its numpress arrays plain, read as a file of its own
        if t.end <= t.start || t.end - t.start > self.limits.byte {
            return Err(Error::memory_budget());
        }
        let mut piece =
            vec![0u8; usize::try_from(t.end - t.start).map_err(|_| Error::memory_budget())?];
        self.data.read_at(t.start, &mut piece)?;
        let simple = np_rewrite::rewrite(&piece, &self.limits)?.into_owned();
        let length = simple.len() as u64;
        let local = Chrom {
            start: 0,
            end: length,
            ..t.clone()
        };
        let mut m = Mzml::new(simple, length);
        let unit = m.chromatogram_unit(&local)?;
        let (time, intensity) = m.chromatogram(&local)?;
        Ok(Chromatogram {
            index: t.index,
            id: t.id.clone(),
            kind: tipo(&t.kind),
            q1: t.q1,
            q3: t.q3,
            unit,
            time,
            intensity,
        })
    }
}

/// The unit of a TIC built from the `totIonCurrent` of the scans (as the mzXML reader gives it).
const UNITS_TIC: &str = "number of detector counts";

impl NumpressReader {
    /// Yes when a cut took the TIC of the file away and scans are left to build it from.
    fn tic_from_the_index(&self, traces: &[Chrom]) -> bool {
        self.cut
            && !self.indice.is_empty()
            && !traces.iter().any(|t| t.kind == "tic" || t.id == "TIC")
    }

    /// The TIC of the whole scans: the `totIonCurrent` of each, in the order of the file (an mzML TIC covers every scan of the run).
    fn index_tic(&self) -> Chromatogram {
        let (time, intensity) = self.indice.iter().map(|v| (v.rt, v.tic)).unzip();
        Chromatogram {
            index: 0,
            id: "TIC".into(),
            kind: ChromKind::Tic,
            q1: None,
            q3: None,
            unit: UNITS_TIC.into(),
            time,
            intensity,
        }
    }
}

/// Yes when the last bytes of the file are its closing tag (`</indexedmzML>` or `</mzML>`).
fn closed(data: &mut dyn ReadAt, dim: u64) -> bool {
    let n = dim.min(64);
    let mut tail = vec![0u8; n as usize];
    if data.read_at(dim - n, &mut tail).is_err() {
        return false;
    }
    let tail = tail.trim_ascii_end();
    tail.ends_with(b"</indexedmzML>") || tail.ends_with(b"</mzML>")
}

fn tipo(k: &str) -> ChromKind {
    match k {
        "tic" => ChromKind::Tic,
        "bpc" => ChromKind::Bpc,
        "pda" => ChromKind::Pda,
        "pressure" => ChromKind::Pressure,
        "srm" => ChromKind::Srm,
        "analog" => ChromKind::Analog,
        _ => ChromKind::Other,
    }
}

impl NumpressReader {
    /// Spectrum `k`: its bytes with the numpress arrays plain, read by the engine's parser; its extra is kept for the writer.
    fn spectrum(&mut self, k: usize) -> Result<Spectrum> {
        let byte = self.byte(k)?;
        let simple = np_rewrite::rewrite(&byte, &self.limits)?;
        let mut spettro = None;
        mzml::parse(&simple[..], |it| {
            if let Item::Spectrum(s) = it {
                spettro = Some(s);
            }
        })?;
        let mut s = spettro
            .ok_or_else(|| Error::new("err.file.xml").with("detail", "no spectrum in its bytes"))?;
        // a spectrum without an id keeps the one of the index (`index={k}`), or `extra_mzml` would not find its extra
        s.index = k;
        if s.id.is_empty() {
            s.id = self.indice[k].id.clone();
        }
        *self.last.borrow_mut() = Some((k, extra_di(&byte)));
        Ok(s)
    }

    /// The header of the mzML we write (`header_from`), with the SHA-1 of the file when the original names no source.
    fn file_header(&mut self) -> Result<Header> {
        let head = self.head_xml()?;
        let mut h = header_from(&head, &self.name, &self.limits)?;
        if h.sources.is_empty() {
            // the whole-file SHA-1 is only worth reading when the original names no source
            if self.sha1.is_none() {
                let dim = self.dim;
                self.sha1 = Some(sha1_hex(&mut self.data, dim).ok());
            }
            h.sources = vec![self.name.clone()];
            h.source_sha1 = self.sha1.clone().flatten().into_iter().collect();
        }
        if let Some(doc) = method::from_userparam(&head) {
            h.run_params.retain(|(k, _)| k != method::USERPARAM);
            h.run_params.push((method::USERPARAM.to_string(), doc));
        }
        Ok(h)
    }
}

/// Yes when a numpress accession shows up before the end of the first spectrum (the m/z array may be plain and hundreds of KB while only the intensities are numpress).
fn recognize(fs: &mut FileSet) -> Fiducia {
    let Some(i) = fs.find(".mzml") else {
        return Fiducia::No;
    };
    let dim = fs.membri.get(i).map_or(0, |m| m.dim);
    let mut n = HEAD;
    loop {
        let head = fs.testa(i, n);
        let text = String::from_utf8_lossy(&head);
        if ha_numpress(&text) {
            return if text.contains("<mzML") {
                Fiducia::Si
            } else {
                Fiducia::No
            };
        }
        if !text.contains("<mzML")
            || text.contains("</spectrum>")
            || (n as u64) >= dim
            || n >= MAX_HEAD
        {
            return Fiducia::No;
        }
        n = (n * 2).min(MAX_HEAD);
    }
}

/// Whether a cvParam of the text names one of the numpress codecs.
fn ha_numpress(text: &str) -> bool {
    ACCESSION
        .iter()
        .any(|a| text.contains(&format!("accession=\"{a}\"")))
}

impl Reader for NumpressReader {
    fn format(&self) -> Format {
        Format::Altro("mzml-numpress")
    }

    fn not_carried(&self) -> &'static [(&'static str, &'static str)] {
        crate::not_carried::NUMPRESS
    }

    fn samples(&self) -> &[Sample] {
        &self.campioni
    }

    fn missing(&mut self, c: usize) -> usize {
        if c != 0 {
            return 0;
        }
        self.declared.saturating_sub(self.rows.len())
    }

    fn index(&mut self, c: usize) -> Result<&[ScanEntry]> {
        self.one(c)?;
        Ok(&self.indice)
    }

    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.one(c)?;
        self.spectrum(k)
    }

    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Field>> {
        self.one(c)?;
        let byte = self.byte(k)?;
        Ok(param_list(&byte)
            .into_iter()
            .map(|p| field(&p.name, p.value, p.units.as_deref(), Some(&p.acc)))
            .collect())
    }

    fn chromatograms(&mut self, c: usize) -> Result<Vec<ChromatogramEntry>> {
        self.one(c)?;
        self.trace_entries()
    }

    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.one(c)?;
        self.trace(id)
    }

    /// The `metodo/1` document the mzML carries (written by mzLab); an mzML of another program has none (`{}`: no section).
    fn method(&mut self, c: usize) -> Result<String> {
        self.one(c)?;
        let head = self.head_xml()?;
        Ok(method::from_userparam(&head).unwrap_or_else(|| "{}".to_string()))
    }

    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> {
        self.one(c)?;
        self.file_header().map(Some)
    }

    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        match self.last.borrow().as_ref() {
            Some((k, e)) if self.indice.get(*k).is_some_and(|v| v.id == s.id) => e.clone(),
            _ => SpectrumExtra::default(),
        }
    }
}

pub(super) fn apri(mut fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    let i = fs
        .find(".mzml")
        .ok_or_else(|| Error::new("err.file.notAccepted"))?;
    let m = fs
        .membri
        .get_mut(i)
        .ok_or_else(|| Error::new("err.file.notAccepted"))?;
    let nome = m
        .nome
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("x.mzML")
        .to_string();
    let dim = m.dim;
    let mut dati: Box<dyn ReadAt> = std::mem::replace(&mut m.dati, Box::new(Vec::<u8>::new()));
    let table = scantable::read(&mut dati, dim)?;
    limits.ok_entries(table.rows.len() as u64)?;
    let mut index = Vec::with_capacity(table.rows.len());
    for (k, r) in table.rows.iter().enumerate() {
        if r.end <= r.start || r.end > dim {
            return Err(Error::new("err.file.xml").with("detail", "spectrum out of the file"));
        }
        let length = u32::try_from(r.end - r.start).map_err(|_| Error::memory_budget())?;
        index.push(ScanEntry {
            id: if r.native.is_empty() {
                format!("index={k}")
            } else {
                r.native.to_string()
            },
            rt: r.rt,
            livello: r.level,
            polarita: r.polarity,
            tic: r.tic,
            base_peak: None,
            filtro: r.filter.to_string(),
            precursore: r.precursor(),
            position: Some(Position {
                membro: 0,
                offset: r.start,
                lunghezza: length,
            }),
        });
    }
    let stem = nome
        .rsplit_once('.')
        .map_or(nome.as_str(), |(s, _)| s)
        .to_string();
    let cut = !closed(&mut *dati, dim);
    Ok(Box::new(NumpressReader {
        data: dati,
        dim,
        campioni: vec![Sample {
            id: 1,
            nome: stem,
            scansioni: index.len(),
            size: None,
        }],
        name: nome,
        limits: *limits,
        declared: table.declared,
        cut,
        rows: table.rows,
        indice: index,
        traces: None,
        sha1: None,
        last: RefCell::new(None),
    }))
}
