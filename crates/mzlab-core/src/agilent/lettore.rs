//! The Agilent `.D` folder as a `Lettore` (`lettori.rs`): the scans by position, one at a time. The index (`MSScan.bin`) and the small side files are read
//! whole when the folder is opened, bounded by `Limiti`; `MSPeak.bin` is never read whole, a scan reads only its own bytes. The members of the `FileSet` are
//! named as in the folder (`AcqData/MSScan.bin`, `acqmeth.txt`, …); a folder with only `data.ms` is read from that.

use std::collections::BTreeMap;

use super::scan::ScanRec;
use super::{AgilentD, Files, SCAN_SIM};
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::lettori::{
    Campione, Campo, Fiducia, FileSet, Formato, Lettore, Limiti, Posizione, VoceCromatogramma,
    VoceLettore, VoceScansione, CONTRATTO,
};
use crate::metodo::{Source, SrcFile};
use crate::mzml::{ChromKind, Chromatogram, Spectrum};

const SCAN: &str = "AcqData/MSScan.bin";
const XSD: &str = "AcqData/MSScan.xsd";
const PEAK: &str = "AcqData/MSPeak.bin";
const TS: &str = "AcqData/MSTS.xml";
const DEVICES: &str = "AcqData/Devices.xml";
const CONTENTS: &str = "AcqData/Contents.xml";
const TREE: &str = "AcqData/MSTree2.bin";
const ACQMETH: &str = "acqmeth.txt";
const DATA_MS: &str = "data.ms";
/// The other small files of the folder that the metadata keep (`AgilentD::metadata`).
const OTHERS: [&str; 8] = [
    "AcqData/sample_info.xml",
    "AcqData/sequence.xml",
    "AcqData-CumulativeAuditTrail.xml",
    "checksum.xml",
    "GC.ini",
    "PRE_POST.INI",
    "runstart.txt",
    "IndexedDataConverter.log",
];

impl ReadAt for Box<dyn ReadAt> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        (**self).read_at(off, buf)
    }
}

/// The index of the member named `name` (a path inside the folder, any case, with or without a folder in front).
fn find(fs: &FileSet, name: &str) -> Option<usize> {
    let low = name.to_lowercase();
    fs.membri.iter().position(|m| {
        let n = m.nome.to_lowercase().replace('\\', "/");
        n == low || n.ends_with(&format!("/{low}"))
    })
}

/// The name of the `.D` folder as the members say it (`run.D/AcqData/…`), else a neutral one.
fn folder_name(fs: &FileSet) -> String {
    fs.membri
        .iter()
        .flat_map(|m| m.nome.split(['/', '\\']))
        .find(|p| p.to_lowercase().ends_with(".d") && p.len() > 2)
        .unwrap_or("run.D")
        .to_string()
}

fn riconosci(fs: &mut FileSet) -> Fiducia {
    if let Some(i) = find(fs, SCAN) {
        let head = fs.testa(i, super::scan::HEADER_LEN);
        let start = head
            .get(88..92)
            .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as i64);
        let dim = fs.membri[i].dim as i64;
        let fits =
            head.len() == super::scan::HEADER_LEN && start.is_some_and(|s| (92..=dim).contains(&s));
        return match (fits, find(fs, PEAK).is_some()) {
            (true, true) => Fiducia::Si,
            (true, false) => Fiducia::Forse,
            _ => Fiducia::No,
        };
    }
    match find(fs, DATA_MS) {
        Some(i) if fs.testa(i, 2) == [0x01, 0x32] => Fiducia::Si,
        _ => Fiducia::No,
    }
}

/// The bytes of member `i`, counted against what is left of `Limiti::byte`.
fn read_all(fs: &mut FileSet, i: usize, left: &mut u64) -> Result<Vec<u8>> {
    let m = &mut fs.membri[i];
    if m.dim > *left {
        return Err(Error::memory_budget());
    }
    *left -= m.dim;
    let mut b = vec![0u8; m.dim as usize];
    m.dati.read_at(0, &mut b)?;
    Ok(b)
}

pub struct LettoreAgilent {
    d: AgilentD,
    peak: Option<Box<dyn ReadAt>>,
    peak_len: u64,
    /// Indexes into `d.records` of the scans that have points (the ones the index lists).
    scans: Vec<usize>,
    campioni: Vec<Campione>,
    indice: Vec<VoceScansione>,
}

fn apri(mut fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    let name = folder_name(&fs);
    limiti.voci_ok(fs.membri.len() as u64)?;
    let mut left = limiti.byte;
    let mut small: Vec<(&str, Vec<u8>)> = Vec::new();
    for n in [SCAN, XSD, TS, DEVICES, CONTENTS, TREE, ACQMETH, DATA_MS]
        .into_iter()
        .chain(OTHERS)
    {
        if let Some(i) = find(&fs, n) {
            small.push((n, read_all(&mut fs, i, &mut left)?));
        }
    }
    let get = |n: &str| {
        small
            .iter()
            .find(|(k, _)| *k == n)
            .map(|(_, b)| b.as_slice())
    };
    let peak_at = find(&fs, PEAK);
    let peak_len = peak_at.map_or(0, |i| fs.membri[i].dim);
    let others: Vec<(&str, &[u8])> = small
        .iter()
        .filter(|(k, _)| OTHERS.contains(k))
        .map(|(k, b)| (*k, b.as_slice()))
        .collect();
    let files = Files {
        ms_scan: get(SCAN).unwrap_or(&[]),
        data_ms: get(DATA_MS),
        ms_scan_xsd: get(XSD),
        ms_peak_len: peak_len,
        ms_ts: get(TS),
        devices: get(DEVICES),
        contents: get(CONTENTS),
        ms_tree: get(TREE),
        acqmeth: get(ACQMETH),
        others: &others,
    };
    limiti.voci_ok((files.ms_scan.len() / 8) as u64)?;
    let d = AgilentD::open(&name, &files)?;
    limiti.voci_ok(d.records.len() as u64)?;
    let peak =
        peak_at.map(|i| std::mem::replace(&mut fs.membri[i].dati, Box::new(Vec::<u8>::new())));
    Ok(Box::new(LettoreAgilent::new(
        d,
        peak,
        peak_at.unwrap_or(0) as u16,
        peak_len,
    )))
}

pub const VOCE: VoceLettore = VoceLettore {
    id: "agilent-d",
    contratto: CONTRATTO,
    estensioni: &[".d"],
    riconosci,
    apri,
};

impl LettoreAgilent {
    fn new(d: AgilentD, peak: Option<Box<dyn ReadAt>>, peak_member: u16, peak_len: u64) -> Self {
        let scans: Vec<usize> = (0..d.records.len())
            .filter(|&i| d.records[i].points > 0)
            .collect();
        let indice = scans
            .iter()
            .map(|&i| {
                let r = &d.records[i];
                VoceScansione {
                    id: format!("scan={}", r.id),
                    rt: r.time_min,
                    livello: r.level.clamp(1, 255) as u8,
                    polarita: d.polarity_of(r),
                    tic: r.tic,
                    picco_base: Some(r.base_mz).filter(|m| *m > 0.0),
                    // a SIM scan says its ions only in its spectrum
                    filtro: d.filter_of(r.scan_type, &[]).trim_end().to_string(),
                    precursore: (r.level >= 2 && r.mz_of_interest > 0.0)
                        .then_some(r.mz_of_interest),
                    posizione: d.inline.is_none().then_some(Posizione {
                        membro: peak_member,
                        offset: r.offset,
                        lunghezza: r.bytes,
                    }),
                }
            })
            .collect();
        let campioni = vec![Campione {
            id: 1,
            nome: d
                .sample_name()
                .map_or_else(|| d.name.clone(), str::to_string),
            scansioni: scans.len(),
        }];
        LettoreAgilent {
            d,
            peak,
            peak_len,
            scans,
            campioni,
            indice,
        }
    }

    /// The same reader asked for an anonymous `metodo/1` (no people, computer, serial number or paths).
    pub fn anonimo(mut self, on: bool) -> Self {
        self.d.anonymous = on;
        self
    }

    fn one(&self, c: usize) -> Result<()> {
        if c == 0 {
            Ok(())
        } else {
            Err(Error::new("error.badIndex").with("c", c))
        }
    }

    fn record(&self, k: usize) -> Result<(usize, &ScanRec)> {
        let i = *self
            .scans
            .get(k)
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        Ok((i, &self.d.records[i]))
    }

    fn points(&mut self, r: &ScanRec) -> Result<(Vec<f64>, Vec<f64>)> {
        match self.peak.as_mut() {
            Some(p) => self.d.points_of(p, r),
            None => self.d.points_of(&mut Vec::<u8>::new(), r),
        }
    }

    /// The ions of the SIM groups of the method, one per 0.01 m/z, ascending.
    fn sim_ions(&self) -> Vec<i64> {
        let mut keys: Vec<i64> = self
            .d
            .method
            .iter()
            .filter(|m| m.mode == "SIM")
            .flat_map(|m| m.groups.iter().flat_map(|g| g.ions.iter()))
            .map(|(mz, _)| (mz * 100.0).round() as i64)
            .collect();
        keys.sort_unstable();
        keys.dedup();
        keys
    }
}

fn field(name: &str, value: impl ToString, unit: Option<&str>, cv: Option<&str>) -> Campo {
    Campo {
        nome: name.into(),
        valore: value.to_string(),
        unita: unit.map(str::to_string),
        cv: cv.map(str::to_string),
    }
}

impl Lettore for LettoreAgilent {
    fn formato(&self) -> Formato {
        Formato::AgilentD
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
        let (i, _) = self.record(k)?;
        let r = self.d.records[i].clone();
        let (mz, it) = self.points(&r)?;
        Ok(self.d.spectrum_of(k, &r, mz, it))
    }

    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.one(c)?;
        let (_, r) = self.record(k)?;
        let mut v = vec![
            field("ScanID", r.id, None, None),
            field("ScanMethodID", r.method_id, None, None),
            field("TimeSegmentID", r.segment, None, None),
            field("MSLevel", r.level, None, Some("MS:1000511")),
            field("ScanType", r.scan_type, None, None),
            field("TIC", r.tic, None, Some("MS:1000285")),
            field("BasePeakMZ", r.base_mz, None, Some("MS:1000504")),
            field("BasePeakValue", r.base_value, None, Some("MS:1000505")),
            field("CycleNumber", r.cycle, None, None),
            field("IonMode", r.ion_mode, None, None),
            field("IonPolarity", r.polarity, None, None),
            field("Fragmentor", r.fragmentor, None, None),
            field(
                "CollisionEnergy",
                r.collision_energy,
                Some("electron volt"),
                Some("MS:1000045"),
            ),
            field("MzOfInterest", r.mz_of_interest, Some("m/z"), None),
            field("SpectrumFormatID", r.format, None, None),
            field("SpectrumOffset", r.offset, None, None),
            field("ByteCount", r.bytes, None, None),
            field("PointCount", r.points, None, None),
            field("MaxY", r.max_y, None, None),
        ];
        if let Some(u) = r.uncompressed {
            v.push(field("UncompressedByteCount", u, None, None));
        }
        v.extend(
            self.d
                .extra_names
                .iter()
                .zip(&r.extra)
                .map(|(n, x)| field(n, x, None, None)),
        );
        Ok(v)
    }

    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        self.one(c)?;
        let mut v: Vec<VoceCromatogramma> = self
            .d
            .survey_chromatograms()
            .into_iter()
            .map(|h| VoceCromatogramma {
                id: h.id,
                tipo: h.kind,
                unita: h.unit,
            })
            .collect();
        v.extend(self.sim_ions().into_iter().map(|key| VoceCromatogramma {
            id: format!("SIC {:.2}", key as f64 / 100.0),
            tipo: ChromKind::Sic,
            unita: "number of detector counts".into(),
        }));
        Ok(v)
    }

    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.one(c)?;
        let bad = || Error::new("error.badIndex").with("id", id);
        if let Some(h) = self
            .d
            .survey_chromatograms()
            .into_iter()
            .find(|h| h.id == id)
        {
            return Ok(h);
        }
        let mz: f64 = id
            .strip_prefix("SIC ")
            .and_then(|t| t.parse().ok())
            .ok_or_else(bad)?;
        let key = (mz * 100.0).round() as i64;
        let index = 2 + self
            .sim_ions()
            .iter()
            .position(|&k| k == key)
            .ok_or_else(bad)?;
        let mut sic: BTreeMap<i64, (Vec<f64>, Vec<f64>)> = BTreeMap::new();
        for i in self.scans.clone() {
            let r = self.d.records[i].clone();
            if r.scan_type == SCAN_SIM {
                let (m, y) = self.points(&r)?;
                super::run::add_sic(&mut sic, r.time_min, &m, &y);
            }
        }
        let (time, intensity) = sic.remove(&key).ok_or_else(bad)?;
        Ok(super::run::sic_chromatogram(index, key, time, intensity))
    }

    fn metodo(&mut self, c: usize) -> Result<String> {
        self.one(c)?;
        let mut files: Vec<SrcFile> = self
            .d
            .file_sha1
            .iter()
            .map(|(name, sha1)| SrcFile {
                name: name.clone(),
                sha1: Some(sha1.clone()),
                bytes: 0,
            })
            .collect();
        if self.d.inline.is_none() {
            // never hashed here: that would read the whole of MSPeak.bin
            files.push(SrcFile {
                name: PEAK.into(),
                sha1: None,
                bytes: self.peak_len,
            });
        }
        Ok(self.d.metodo(&Source {
            format: "agilent-d",
            files,
            sample: None,
            anonymous: self.d.anonymous,
        }))
    }
}
