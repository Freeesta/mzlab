//! The Thermo `.raw` as a `Lettore` (`lettori.rs`): the scans by position, one at a time. The container is opened through a `Read + Seek` view of the
//! member (indexes only, no scan is read); a scan reads only its own packet. The index comes from the scan index and the scan events (`scan_metadata`),
//! so listing a run touches no peak data. Peaks are what the file holds (the centroid list of a scan, its profile when it has no other), as `raw::read`.

use std::io::{Read, Seek, SeekFrom};

use opentfraw::extra::scan_extras;
use opentfraw::mzml::{extract_spectrum, scan_metadata, ScanMetadata};
use opentfraw::{ExtraFields, RawFileReader};

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::lettori::{
    Campione, Campo, Fiducia, FileSet, Formato, Lettore, Limiti, Posizione, VoceCromatogramma,
    VoceLettore, VoceScansione, CONTRATTO,
};
use crate::metodo::{self, sha1_hex};
use crate::mzml::{ChromKind, Chromatogram, Spectrum};
use crate::mzmlwrite::Header;

const MAGIC: [u8; 18] = [
    0x01, 0xA1, b'F', 0, b'i', 0, b'n', 0, b'n', 0, b'i', 0, b'g', 0, b'a', 0, b'n', 0,
];

fn riconosci(fs: &mut FileSet) -> Fiducia {
    let Some(i) = fs.trova(".raw") else {
        return Fiducia::No;
    };
    if fs.testa(i, MAGIC.len()) == MAGIC {
        Fiducia::Si
    } else {
        Fiducia::No
    }
}

pub const VOCE: VoceLettore = VoceLettore {
    id: "thermo-raw",
    contratto: CONTRATTO,
    estensioni: &[".raw"],
    riconosci,
    apri,
};

/// A `ReadAt` seen as `Read + Seek` (what `opentfraw` reads from).
struct Stream {
    inner: Box<dyn ReadAt>,
    pos: u64,
    len: u64,
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = usize::try_from(self.len.saturating_sub(self.pos))
            .unwrap_or(usize::MAX)
            .min(buf.len());
        let Some(out) = buf.get_mut(..n) else {
            return Ok(0);
        };
        self.inner
            .read_at(self.pos, out)
            .map_err(|e| std::io::Error::other(e.key))?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for Stream {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let p = match to {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::End(d) => self.len.checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        self.pos = p.ok_or_else(|| std::io::Error::other("seek before the start"))?;
        Ok(self.pos)
    }
}

pub struct LettoreRaw {
    raw: RawFileReader,
    src: Stream,
    nome: String,
    campioni: Vec<Campione>,
    indice: Vec<VoceScansione>,
    /// SHA-1 of the file, read the first time the mzML header is asked for (`Some(None)`: it could not be read).
    sha1: Option<Option<String>>,
    /// The traces of the analog controllers (pressure, temperatures, UV channels), read the first time they are asked for.
    analogiche: Option<Vec<(String, ChromKind, opentfraw::AnalogTrace)>>,
}

fn raw_error(e: impl std::fmt::Display) -> Error {
    Error::new("err.file.raw").with("detail", e)
}

fn apri(mut fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    let i = fs
        .trova(".raw")
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
    let mut src = Stream {
        inner: std::mem::replace(&mut m.dati, Box::new(Vec::<u8>::new())),
        pos: 0,
        len: m.dim,
    };
    let raw = RawFileReader::open(&mut src).map_err(raw_error)?;
    limiti.voci_ok(u64::from(raw.num_scans))?;
    let indice: Vec<VoceScansione> = (0..raw.num_scans)
        .filter_map(|k| scan_metadata(&raw, k))
        .map(|m| entry(&raw, &m))
        .collect();
    let stem = nome
        .rsplit_once('.')
        .map_or(nome.as_str(), |(s, _)| s)
        .to_string();
    let campioni = vec![Campione {
        id: 1,
        nome: stem,
        scansioni: indice.len(),
    }];
    Ok(Box::new(LettoreRaw {
        raw,
        src,
        nome,
        campioni,
        indice,
        sha1: None,
        analogiche: None,
    }))
}

/// The row of the index of a scan, from its metadata alone.
fn entry(raw: &RawFileReader, m: &ScanMetadata) -> VoceScansione {
    let at = raw
        .scan_index
        .get(m.index)
        .map(|e| (raw.data_addr.saturating_add(e.offset), e.data_size));
    VoceScansione {
        id: format!("controllerType=0 controllerNumber=1 scan={}", m.scan_number),
        rt: m.retention_time_min,
        livello: u8::try_from(m.ms_level).unwrap_or(u8::MAX),
        polarita: match m.polarity {
            Some(opentfraw::Polarity::Positive) => 1,
            Some(opentfraw::Polarity::Negative) => -1,
            None => 0,
        },
        tic: m.total_ion_current,
        picco_base: Some(m.base_peak_mz).filter(|x| *x > 0.0),
        filtro: m.filter.clone().unwrap_or_default(),
        precursore: m
            .precursor
            .as_ref()
            .and_then(|p| p.selected_mz.or(p.target_mz)),
        posizione: at.map(|(offset, lunghezza)| Posizione {
            membro: 0,
            offset,
            lunghezza,
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
fn analogiche(l: &mut LettoreRaw) -> &[(String, ChromKind, opentfraw::AnalogTrace)] {
    if l.analogiche.is_none() {
        let tracce = l.raw.analog_traces(&mut l.src).unwrap_or_default();
        let mut out: Vec<(String, ChromKind, opentfraw::AnalogTrace)> = Vec::new();
        for t in tracce {
            let doppio = out.iter().any(|(id, ..)| *id == t.name);
            let id = if doppio {
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
fn elenco_cromatogrammi(l: &mut LettoreRaw) -> Vec<VoceCromatogramma> {
    let mut v = vec![VoceCromatogramma {
        id: "TIC".into(),
        tipo: ChromKind::Tic,
        unita: "number of detector counts".into(),
    }];
    v.extend(analogiche(l).iter().map(|(id, tipo, t)| VoceCromatogramma {
        id: id.clone(),
        tipo: *tipo,
        unita: t.unit.clone(),
    }));
    v
}

/// A series of the instrument log (status log or trailer, `metodo_raw/serie.rs`): not in the list above, so neither the mzML nor the `.mzidx` carry it; computed here when asked for.
fn registro_di(l: &LettoreRaw, id: &str) -> Option<Chromatogram> {
    let s = crate::metodo_raw::serie::serie(&l.raw, id)?;
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

fn cromatogramma_di(l: &mut LettoreRaw, id: &str) -> Result<Chromatogram> {
    if let Some(c) = registro_di(l, id) {
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

impl LettoreRaw {
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
}

fn field(nome: &str, valore: impl ToString, unita: Option<&str>, cv: Option<&str>) -> Campo {
    Campo {
        nome: nome.into(),
        valore: valore.to_string(),
        unita: unita.map(str::to_string),
        cv: cv.map(str::to_string),
    }
}

impl Lettore for LettoreRaw {
    fn formato(&self) -> Formato {
        Formato::ThermoRaw
    }

    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }

    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        self.one(c)?;
        Ok(&self.indice)
    }

    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.one(c)?;
        let idx = self.row(k)?;
        let rec = extract_spectrum(&self.raw, &mut self.src, idx, false)
            .ok_or_else(|| raw_error("scan cannot be decoded"))?;
        Ok(crate::raw::spectrum(rec))
    }

    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.one(c)?;
        let m = scan_metadata(&self.raw, self.row(k)?).ok_or_else(|| raw_error("scan metadata"))?;
        let mut out = header_fields(&self.raw, &m);
        out.extend(noise_fields(&self.raw, &mut self.src, k as u32));
        Ok(out)
    }

    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        self.one(c)?;
        Ok(elenco_cromatogrammi(self))
    }

    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.one(c)?;
        cromatogramma_di(self, id)
    }

    fn metodo(&mut self, c: usize) -> Result<String> {
        self.one(c)?;
        Ok(crate::raw::metodo_of(&self.raw, &mut self.src, &self.nome))
    }

    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        self.one(c)?;
        Ok(Some(testata(self)))
    }
}

fn median(mut v: Vec<f32>) -> f32 {
    // hot path: per-point/per-peak sort, std is ~2x faster than sortc (docs/studi/OTTIMIZZAZIONE_CORE.md §4.1)
    #[allow(clippy::disallowed_methods)]
    v.sort_by(f32::total_cmp);
    v[v.len() / 2]
}

/// The noise and the baseline an FT scan carries (a piecewise-linear function of m/z, a few dozen nodes): `thermo:noise_function` holds every node as
/// `m/z,noise,baseline` joined by `;` (nothing is lost: the value at a peak is the interpolation), `thermo:noise` and `thermo:baseline` the median of the nodes
/// for a quick read. A scan without them (ion trap, SRM) gives nothing. The peaks are not touched.
fn noise_fields(raw: &RawFileReader, src: &mut Stream, idx: u32) -> Vec<Campo> {
    let scan = raw.run_header.sample_info.first_scan_number + idx;
    let Ok(p) = raw.read_scan_labels(src, scan) else {
        return Vec::new();
    };
    let nodes = &p.noise_nodes;
    if nodes.is_empty() {
        return Vec::new();
    }
    let function = nodes
        .iter()
        .map(|n| format!("{},{},{}", n.mz, n.noise, n.baseline))
        .collect::<Vec<_>>()
        .join(";");
    vec![
        field(
            "thermo:noise",
            median(nodes.iter().map(|n| n.noise).collect()),
            None,
            None,
        ),
        field(
            "thermo:baseline",
            median(nodes.iter().map(|n| n.baseline).collect()),
            None,
            None,
        ),
        field("thermo:noise_function", function, None, None),
    ]
}

/// The header fields of a scan: what the isolation and the activation say, the injection time, and every `opentfraw.*` value of the scan (the trailer).
fn header_fields(raw: &RawFileReader, m: &ScanMetadata) -> Vec<Campo> {
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
        scan_extras(raw, m, &ExtraFields::All)
            .into_iter()
            .map(|(k, v)| field(&k, v, None, None)),
    );
    out
}

/// The header of the mzML: the instrument, the run terms the Thermo converter writes (`opentfraw.*`), the start of the acquisition, the SHA-1 of the
/// file and the `metodo/1` document, so that the download of a `.raw` loses none of them.
fn testata(l: &mut LettoreRaw) -> Header {
    if l.sha1.is_none() {
        let len = l.src.len;
        l.sha1 = Some(sha1_hex(&mut l.src.inner, len).ok());
    }
    let sha1 = l.sha1.clone().flatten();
    let t = opentfraw::mzml::run_terms(&l.raw);
    let run = l
        .campioni
        .first()
        .map(|c| c.nome.clone())
        .unwrap_or_default();
    let document = crate::raw::metodo_sha1(&l.raw, &mut l.src, &l.nome, sha1.clone());
    let acquired = t
        .start_timestamp
        .or_else(|| crate::metodo_raw::campi::clock(l.raw.header.audit_start.time));
    let mut run_params: Vec<(String, String)> = t.extra;
    run_params.push((metodo::USERPARAM.to_string(), document));
    Header {
        run,
        sources: vec![l.nome.clone()],
        source_sha1: sha1.into_iter().collect(),
        source_cv: vec![
            ("MS:1000563".into(), "Thermo RAW format".into()),
            ("MS:1000768".into(), "Thermo nativeID format".into()),
        ],
        instrument: Some(t.instrument),
        components: t
            .analyzers
            .into_iter()
            .map(|(a, n)| ("analyzer".to_string(), a.to_string(), n.to_string()))
            .collect(),
        acquired,
        run_params,
        software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
        ..Header::default()
    }
}
