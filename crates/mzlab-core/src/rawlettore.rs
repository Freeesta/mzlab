//! The Thermo `.raw` as a `Lettore` (`lettori.rs`): the scans by position, one at a time. The container is opened through a `Read + Seek` view of the
//! member (indexes only, no scan is read); a scan reads only its own packet. The index comes from the scan index and the scan events (`scan_metadata`),
//! so listing a run touches no peak data. Peaks are what the file holds (the centroid list of a scan, its profile when it has no other), as `raw::read`.

use opentfraw::extra::scan_extras_with;
use opentfraw::mzml::{extract_arrays, extract_spectrum, scan_metadata, ScanMetadata};
use opentfraw::{ExtraFields, RawFileReader, ScanParams};

use crate::error::{Error, Result};
use crate::lettori::{
    ChromatogramEntry, Fiducia, Field, FileSet, Format, Limits, Position, Reader, ReaderEntry,
    Sample, ScanEntry, CONTRATTO,
};
use crate::mzml::{ChromKind, Chromatogram, Spectrum};
use crate::mzmlwrite::{Header, SpectrumExtra};
use flusso::Stream;
#[cfg(test)]
use flusso::{SKIP_WINDOW, WINDOW};

mod extra;
mod flusso;
mod testata;

const MAGIC: [u8; 18] = [
    0x01, 0xA1, b'F', 0, b'i', 0, b'n', 0, b'n', 0, b'i', 0, b'g', 0, b'a', 0, b'n', 0,
];

fn riconosci(fs: &mut FileSet) -> Fiducia {
    let Some(i) = fs.find(".raw") else {
        return Fiducia::No;
    };
    if fs.testa(i, MAGIC.len()) == MAGIC {
        Fiducia::Si
    } else {
        Fiducia::No
    }
}

pub const ENTRY: ReaderEntry = ReaderEntry {
    id: "thermo-raw",
    contratto: CONTRATTO,
    estensioni: &[".raw"],
    riconosci,
    apri,
};

pub struct RawReader {
    raw: RawFileReader,
    src: Stream,
    name: String,
    campioni: Vec<Sample>,
    indice: Vec<ScanEntry>,
    /// SHA-1 of the file: the one the caller gave (`Membro::sha1`) or read the first time the mzML header is asked for (`Some(None)`: it could not be read).
    sha1: Option<Option<String>>,
    /// The traces of the analog controllers (pressure, temperatures, UV channels), read the first time they are asked for.
    analogiche: Option<Vec<(String, ChromKind, opentfraw::AnalogTrace)>>,
}

fn raw_error(e: impl std::fmt::Display) -> Error {
    Error::new("err.file.raw").with("detail", e)
}

fn apri(mut fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    let i = fs
        .find(".raw")
        .ok_or_else(|| Error::new("err.file.raw").with("detail", "no .raw member"))?;
    let m = fs
        .membri
        .get_mut(i)
        .ok_or_else(|| Error::new("err.file.raw").with("detail", "member"))?;
    let nome = m
        .nome
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("x.raw")
        .to_string();
    let mut src = Stream::new(
        std::mem::replace(&mut m.dati, Box::new(Vec::<u8>::new())),
        m.dim,
    );
    let sha1 = m.sha1.take().map(Some);
    let raw = RawFileReader::open(&mut src).map_err(|e| crate::raw::thermo_error(&e))?;
    limits.ok_entries(u64::from(raw.num_scans))?;
    let index: Vec<ScanEntry> = (0..raw.num_scans)
        .filter_map(|k| scan_metadata(&raw, &mut src, k))
        .map(|m| entry(&raw, &m))
        .collect();
    let stem = nome
        .rsplit_once('.')
        .map_or(&nome[..], |(s, _)| s)
        .to_string();
    let samples = vec![Sample {
        id: 1,
        nome: stem,
        scansioni: index.len(),
        size: None,
    }];
    Ok(Box::new(RawReader {
        raw,
        src,
        name: nome,
        campioni: samples,
        indice: index,
        sha1,
        analogiche: None,
    }))
}

/// The row of the index of a scan, from its metadata alone.
fn entry(raw: &RawFileReader, m: &ScanMetadata) -> ScanEntry {
    let at = raw
        .scan_index
        .get(m.index)
        .map(|e| (raw.data_addr.saturating_add(e.offset), e.data_size));
    ScanEntry {
        id: format!("controllerType=0 controllerNumber=1 scan={}", m.scan_number),
        rt: m.retention_time_min,
        livello: u8::try_from(m.ms_level).unwrap_or(u8::MAX),
        polarita: match m.polarity {
            Some(opentfraw::Polarity::Positive) => 1,
            Some(opentfraw::Polarity::Negative) => -1,
            None => 0,
        },
        tic: m.total_ion_current,
        base_peak: Some(m.base_peak_mz).filter(|x| *x > 0.0),
        filtro: m.filter.clone().unwrap_or_default(),
        precursore: m
            .precursor
            .as_ref()
            .and_then(|p| p.selected_mz.or(p.target_mz)),
        position: at.map(|(offset, length)| Position {
            membro: 0,
            offset,
            lunghezza: length,
        }),
    }
}

/// The kind of an analog channel: the UV channels of a Dionex detector (`UV_VIS_1`) are PDA traces, a channel with a pressure unit or a pump pressure is a pressure,
/// the rest (temperatures) are analog. A spectrum of the UV field is never a trace of this list: it is not decoded (`metodo/1.avvisi`).
fn tipo_analogico(t: &opentfraw::AnalogTrace) -> ChromKind {
    let n = t.name.to_ascii_lowercase();
    let u = t.unit.to_ascii_lowercase();
    if n.starts_with("uv_vis") || n.starts_with("pda") {
        ChromKind::Pda
    } else if n.contains("pressure")
        || matches!(u.as_str(), "psi" | "bar" | "mbar" | "pa" | "kpa" | "mpa")
    {
        ChromKind::Pressure
    } else {
        ChromKind::Analog
    }
}

/// Id, kind and samples of every analog channel; two channels with the same name get the number of their controller after it.
fn analogiche(l: &mut RawReader) -> &[(String, ChromKind, opentfraw::AnalogTrace)] {
    if l.analogiche.is_none() {
        let traces = l.raw.analog_traces(&mut l.src).unwrap_or_default();
        let mut out: Vec<(String, ChromKind, opentfraw::AnalogTrace)> = Vec::new();
        for t in traces {
            let double = out.iter().any(|(id, ..)| *id == t.name);
            let id = if double {
                format!("{} #{}", t.name, t.controller)
            } else {
                t.name.clone()
            };
            out.push((id, tipo_analogico(&t), t));
        }
        l.analogiche = Some(out);
    }
    l.analogiche.as_deref().unwrap_or_default()
}

/// The TIC, then the analog channels.
fn chromatogram_list(l: &mut RawReader) -> Vec<ChromatogramEntry> {
    let mut v = vec![ChromatogramEntry {
        id: "TIC".into(),
        tipo: ChromKind::Tic,
        unita: "number of detector counts".into(),
    }];
    v.extend(analogiche(l).iter().map(|(id, tipo, t)| ChromatogramEntry {
        id: id.clone(),
        tipo: *tipo,
        unita: t.unit.clone(),
    }));
    v
}

/// A series of the instrument log (status log or trailer, `metodo_raw/serie.rs`): not in the list above, so neither the mzML nor the `.mzidx` carry it; computed here when asked for.
fn log_of(l: &mut RawReader, id: &str) -> Option<Chromatogram> {
    let s = crate::metodo_raw::serie::series(&l.raw, &mut l.src, id)?;
    Some(Chromatogram {
        index: 0,
        id: id.into(),
        kind: ChromKind::Analog,
        q1: None,
        q3: None,
        unit: s.unita,
        time: s.t,
        intensity: s.y,
    })
}

fn cromatogramma_di(l: &mut RawReader, id: &str) -> Result<Chromatogram> {
    if let Some(c) = log_of(l, id) {
        return Ok(c);
    }
    if id != "TIC" {
        let (index, (id, kind, t)) = analogiche(l)
            .iter()
            .enumerate()
            .find(|(_, (n, ..))| n == id)
            .ok_or_else(|| Error::new("error.badIndex").with("id", id))?;
        return Ok(Chromatogram {
            index: index + 1,
            id: id.clone(),
            kind: *kind,
            q1: None,
            q3: None,
            unit: t.unit.clone(),
            time: t.time_min.clone(),
            intensity: t.values.clone(),
        });
    }
    let (time, intensity): (Vec<f64>, Vec<f64>) = l.raw.tic_chromatogram().into_iter().unzip();
    Ok(Chromatogram {
        index: 0,
        id: "TIC".into(),
        kind: ChromKind::Tic,
        q1: None,
        q3: None,
        unit: "number of detector counts".into(),
        time,
        intensity,
    })
}

impl RawReader {
    fn one(&self, c: usize) -> Result<()> {
        (c == 0)
            .then_some(())
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    /// The scan-index position of row `k` (the index lists every scan of the file in order).
    fn row(&self, k: usize) -> Result<u32> {
        self.indice
            .get(k)
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        u32::try_from(k).map_err(|_| raw_error("scan number"))
    }

    /// Scan `k` of sample `c` decoded, with the profile points when `profilo` and otherwise the centroids the file holds.
    fn decodificata(&mut self, c: usize, k: usize, profilo: bool) -> Result<Spectrum> {
        self.one(c)?;
        let idx = self.row(k)?;
        let rec = extract_spectrum(&self.raw, &mut self.src, idx, profilo)
            .ok_or_else(|| raw_error("scan cannot be decoded"))?;
        Ok(crate::raw::spectrum(rec))
    }
}

fn field(name: &str, value: impl ToString, unita: Option<&str>, cv: Option<&str>) -> Field {
    Field {
        nome: name.into(),
        valore: value.to_string(),
        unita: unita.map(str::to_string),
        cv: cv.map(str::to_string),
    }
}

impl Reader for RawReader {
    fn format(&self) -> Format {
        Format::ThermoRaw
    }

    fn not_carried(&self) -> &'static [(&'static str, &'static str)] {
        crate::conv::THERMO
    }

    fn samples(&self) -> &[Sample] {
        &self.campioni
    }

    fn index(&mut self, c: usize) -> Result<&[ScanEntry]> {
        self.one(c)?;
        Ok(&self.indice)
    }

    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.decodificata(c, k, false)
    }

    fn scan_profile(&mut self, c: usize, k: usize) -> Result<Option<Spectrum>> {
        self.decodificata(c, k, true).map(Some)
    }

    /// The points of the scan and the rest from the row of the index: no trailer, no filter, no precursors read again.
    fn scan_data(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.one(c)?;
        let idx = self.row(k)?;
        let (mz, intensity, mode) = extract_arrays(&self.raw, &mut self.src, idx)
            .ok_or_else(|| raw_error("scan cannot be decoded"))?;
        let v = self
            .indice
            .get(k)
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        Ok(Spectrum {
            index: k,
            id: String::new(),
            level: v.livello,
            rt: v.rt,
            polarity: v.polarita,
            profile: mode == Some(opentfraw::ScanMode::Profile),
            filter: String::new(),
            precursor: v.precursore,
            mz,
            intensity,
            wide: None,
        })
    }

    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Field>> {
        extra::scan_header(self, c, k)
    }

    fn chromatograms(&mut self, c: usize) -> Result<Vec<ChromatogramEntry>> {
        self.one(c)?;
        Ok(chromatogram_list(self))
    }

    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.one(c)?;
        cromatogramma_di(self, id)
    }

    fn missing(&mut self, _c: usize) -> usize {
        crate::raw::cut_scans(&self.raw)
    }
    fn method(&mut self, c: usize) -> Result<String> {
        self.one(c)?;
        Ok(crate::raw::metodo_of(&self.raw, &mut self.src, &self.name))
    }

    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> {
        testata::complete(self, c)
    }
    fn table_header(&mut self, c: usize) -> Result<Option<Header>> {
        testata::leggera(self, c)
    }

    fn scan_extra(&mut self, c: usize, k: usize, s: &Spectrum) -> SpectrumExtra {
        extra::di_scansione(self, c, k, s)
    }
}

/// The header fields of a scan: what the isolation and the activation say, the injection time, and every `opentfraw.*` value of the scan (the trailer).
fn header_fields(
    raw: &RawFileReader,
    params: Option<ScanParams<'_>>,
    m: &ScanMetadata,
) -> Vec<Field> {
    let mut out = Vec::new();
    if let Some(p) = &m.precursor {
        let ce = p.collision_energy.map(|ce| {
            if p.ce_is_nce {
                field("normalized collision energy", ce, None, Some("MS:1000138"))
            } else {
                field(
                    "collision energy",
                    ce,
                    Some("electronvolt"),
                    Some("MS:1000045"),
                )
            }
        });
        out.extend(ce);
        out.extend(p.target_mz.map(|v| {
            field(
                "isolation window target m/z",
                v,
                Some("m/z"),
                Some("MS:1000827"),
            )
        }));
        out.extend(
            p.isolation_width
                .map(|v| field("isolation window width", v, Some("m/z"), None)),
        );
        out.extend(
            p.charge
                .map(|v| field("charge state", v, None, Some("MS:1000041"))),
        );
    }
    out.extend(
        m.precursor
            .iter()
            .flat_map(crate::rawlettore_reazioni::reaction_fields),
    );
    out.extend(m.ion_injection_time_ms.map(|v| {
        field(
            "ion injection time",
            v,
            Some("millisecond"),
            Some("MS:1000927"),
        )
    }));
    out.extend(m.faims_cv.map(|v| {
        field(
            "FAIMS compensation voltage",
            v,
            Some("volt"),
            Some("MS:1001581"),
        )
    }));
    out.extend(
        scan_extras_with(raw, params, m, &ExtraFields::All)
            .into_iter()
            .map(|(k, v)| field(&k, v, None, None)),
    );
    out
}

#[cfg(test)]
#[path = "rawlettore_stream_tests.rs"]
mod stream_tests;
