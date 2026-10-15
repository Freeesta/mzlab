//! The Agilent `.D` folder as a `Lettore` (`lettori.rs`): the scans by position, one at a time. The index (`MSScan.bin`) and the small side files are read
//! whole when the folder is opened, bounded by `Limiti`; `MSPeak.bin` is never read whole, a scan reads only its own bytes. The members of the `FileSet` are
//! named as in the folder (`AcqData/MSScan.bin`, `acqmeth.txt`, …); a folder with only `data.ms` is read from that.

use std::collections::BTreeMap;

use super::run::sha1_of;
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
use crate::mzmlwrite::{Header, SpectrumExtra};
use crate::window::Windowed;

const SCAN: &str = "AcqData/MSScan.bin";
const XSD: &str = "AcqData/MSScan.xsd";
const PEAK: &str = "AcqData/MSPeak.bin";
const TS: &str = "AcqData/MSTS.xml";
const DEVICES: &str = "AcqData/Devices.xml";
const CONTENTS: &str = "AcqData/Contents.xml";
const TREE: &str = "AcqData/MSTree2.bin";
const ACQMETH: &str = "acqmeth.txt";
const DATA_MS: &str = "data.ms";
/// The profile spectra of a Q-TOF run, where `MSPeak.bin` is empty or absent (feature `estesi`).
const PROFILE: &str = "AcqData/MSProfile.bin";
const MASS_CAL: &str = "AcqData/MSMassCal.bin";
const DEFAULT_CAL: &str = "AcqData/DefaultMassCal.xml";
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

/// The method report of an LC/MS run (feature `estesi`).
const ACQ_METHOD: &str = "AcqData/AcqMethod.xml";
/// The binary members of `AcqData` that are not decoded and are kept as size and SHA-1 when they are small (module traces, actuals, spectra of the DAD…).
const UNDECODED_MAX: u64 = 4 << 20;

/// The members under `AcqData/` (not in a subfolder) that an LC/MS run keeps besides the data: (index, path in the folder). `MSPeak.bin`, `MSProfile.bin` and the
/// files `apri` reads for the data are not among them.
fn lc_extras(fs: &FileSet) -> Vec<(usize, String)> {
    const KNOWN: [&str; 13] = [
        SCAN,
        XSD,
        TS,
        DEVICES,
        CONTENTS,
        TREE,
        PEAK,
        PROFILE,
        MASS_CAL,
        DEFAULT_CAL,
        ACQ_METHOD,
        "AcqData/sample_info.xml",
        "AcqData/sequence.xml",
    ];
    fs.membri
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let path = m.nome.replace('\\', "/");
            let parts: Vec<&str> = path.split('/').collect();
            let at = parts
                .iter()
                .rposition(|p| p.eq_ignore_ascii_case("AcqData"))?;
            let rest = &parts[at + 1..];
            let file = *rest.first().filter(|_| rest.len() == 1)?;
            let rel = format!("AcqData/{file}");
            let ext = file.rsplit_once('.')?.1.to_lowercase();
            let wanted = matches!(
                ext.as_str(),
                "xml" | "ini" | "cd" | "cg" | "sd" | "sp" | "ch" | "uv" | "pdf" | "bin"
            );
            (wanted && !KNOWN.iter().any(|k| k.eq_ignore_ascii_case(&rel))).then_some((i, rel))
        })
        .collect()
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

/// Records looked at by `variante`.
const VARIANT_SCANS: usize = 64;

/// True when the folder is a variant that the engine's reader (the one without `estesi`) opens or refuses but cannot read the points of: an xsd with nested types,
/// or a record whose spectrum is not 16 bytes per point of the GC/MS layout (other formats, profile, compressed). Looks at the head of `MSScan.bin` and at the xsd
/// only. A folder that is not an Agilent `.D` is not a variant.
pub fn variante(fs: &mut FileSet) -> bool {
    let Some(i) = find(fs, SCAN) else {
        return false;
    };
    let layout = match find(fs, XSD) {
        Some(x) if fs.membri[x].dim <= (1 << 20) => {
            let xsd = fs.testa(x, fs.membri[x].dim as usize);
            match super::scan::Layout::from_xsd(&String::from_utf8_lossy(&xsd)) {
                Ok(l) => l,
                Err(_) => return true,
            }
        }
        _ => super::scan::Layout::builtin(),
    };
    let head = fs.testa(i, super::scan::HEADER_LEN);
    let Some(start) = head
        .get(88..92)
        .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .filter(|s| *s >= 92)
    else {
        return false;
    };
    let n = layout.record_len;
    let bytes = fs.testa(i, start as usize + VARIANT_SCANS * n);
    bytes.get(start as usize..).is_some_and(|rest| {
        rest.chunks_exact(n)
            .map(|r| super::scan::record(&layout, r))
            .filter(|r| r.points > 0)
            .any(|r| {
                r.uncompressed.is_some_and(|u| u != r.bytes)
                    || r.format != 1
                    || r.bytes as usize != r.points as usize * 16
            })
    })
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

fn field(name: &str, value: impl ToString, unit: Option<&str>, cv: Option<&str>) -> Campo {
    Campo {
        nome: name.into(),
        valore: value.to_string(),
        unita: unit.map(str::to_string),
        cv: cv.map(str::to_string),
    }
}

/// Every field of the record of a scan, the extra ones of the xsd included.
fn header_fields(d: &AgilentD, r: &ScanRec) -> Vec<Campo> {
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
    let extra = d.extra_names.iter().zip(&r.extra);
    v.extend(extra.map(|(n, x)| field(n, x, None, None)));
    v
}

/// The index of the scans that have points: what is known of each before its peaks are read.
fn build_index(d: &AgilentD, scans: &[usize], peak_member: u16) -> Vec<VoceScansione> {
    scans
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
                precursore: (r.level >= 2 && r.mz_of_interest > 0.0).then_some(r.mz_of_interest),
                posizione: d.inline.is_none().then_some(Posizione {
                    membro: peak_member,
                    offset: r.offset,
                    lunghezza: r.bytes,
                }),
            }
        })
        .collect()
}

/// The `metodo/1` document; `MSPeak.bin` is listed without a SHA-1 (hashing it would read the whole file).
fn document(l: &LettoreAgilent) -> String {
    let mut files: Vec<SrcFile> =
        l.d.file_sha1
            .iter()
            .map(|(name, sha1)| SrcFile {
                name: name.clone(),
                sha1: Some(sha1.clone()),
                bytes: 0,
            })
            .collect();
    if l.d.inline.is_none() {
        let member = l.d.data_file;
        files.push(SrcFile {
            name: member.into(),
            sha1: None,
            bytes: l.peak_len,
        });
    }
    l.d.metodo(&Source {
        format: "agilent-d",
        files,
        sample: None,
    })
}

/// The chromatogram `id`: the TIC and the BPC from the records, a SIC by reading the SIM scans.
fn trace(l: &mut LettoreAgilent, id: &str) -> Result<Chromatogram> {
    let bad = || Error::new("error.badIndex").with("id", id);
    if let Some(h) = l.d.survey_chromatograms().into_iter().find(|h| h.id == id) {
        return Ok(h);
    }
    if id.starts_with("SRM SIC ") {
        return l
            .srm()?
            .iter()
            .find(|c| c.id == id)
            .cloned()
            .ok_or_else(bad);
    }
    let mz: f64 = id
        .strip_prefix("SIC ")
        .and_then(|t| t.parse().ok())
        .ok_or_else(bad)?;
    let key = (mz * 100.0).round() as i64;
    let index = 2 + l
        .sim_ions()
        .iter()
        .position(|&k| k == key)
        .ok_or_else(bad)?;
    let mut sic: BTreeMap<i64, (Vec<f64>, Vec<f64>)> = BTreeMap::new();
    for i in l.scans.clone() {
        let r = l.d.records[i].clone();
        if r.scan_type == SCAN_SIM {
            let (m, y) = l.points(&r)?;
            super::run::add_sic(&mut sic, r.time_min, &m, &y);
        }
    }
    let (time, intensity) = sic.remove(&key).ok_or_else(bad)?;
    Ok(super::run::sic_chromatogram(index, key, time, intensity))
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
    let peak_at = peak_at
        .filter(|&i| fs.membri[i].dim > 0)
        .or_else(|| find(&fs, PROFILE))
        .or(peak_at);
    let cal = (
        find(&fs, MASS_CAL)
            .map(|i| read_all(&mut fs, i, &mut left))
            .transpose()?,
        find(&fs, DEFAULT_CAL)
            .map(|i| read_all(&mut fs, i, &mut left))
            .transpose()?,
    );
    let (method_xml, extra_files, extra_sizes) = {
        let method = find(&fs, ACQ_METHOD)
            .map(|i| read_all(&mut fs, i, &mut left))
            .transpose()?;
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut sizes: Vec<(String, u64)> = Vec::new();
        for (i, rel) in lc_extras(&fs) {
            let dim = fs.membri[i].dim;
            // a binary too large to hash is named by its size; the budget of the folder is spent only on what is read
            if dim > UNDECODED_MAX || dim > left {
                sizes.push((rel, dim));
            } else {
                files.push((rel, read_all(&mut fs, i, &mut left)?));
            }
        }
        (method, files, sizes)
    };
    let data_file = match peak_at {
        Some(i) if find(&fs, PROFILE) == Some(i) => PROFILE,
        _ => PEAK,
    };
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
    let d = {
        let d = d.with_calibration(cal.0.as_deref(), cal.1.as_deref());
        let refs: Vec<(&str, &[u8])> = extra_files
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        let sizes: Vec<(&str, u64)> = extra_sizes.iter().map(|(n, s)| (n.as_str(), *s)).collect();
        d.with_lc_files(
            method_xml.as_deref(),
            get(DEVICES),
            &refs,
            &sizes,
            data_file,
        )
    };
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

pub struct LettoreAgilent {
    d: AgilentD,
    peak: Option<Box<dyn ReadAt>>,
    peak_len: u64,
    /// Indexes into `d.records` of the scans that have points (the ones the index lists).
    scans: Vec<usize>,
    campioni: Vec<Campione>,
    indice: Vec<VoceScansione>,
    /// The SRM chromatograms of an MRM run, read on first use.
    srm: Option<Vec<Chromatogram>>,
}

impl LettoreAgilent {
    fn new(d: AgilentD, peak: Option<Box<dyn ReadAt>>, peak_member: u16, peak_len: u64) -> Self {
        let scans: Vec<usize> = (0..d.records.len())
            .filter(|&i| d.listed(&d.records[i]))
            .collect();
        let indice = build_index(&d, &scans, peak_member);
        let nome = d
            .sample_name()
            .map_or_else(|| d.name.clone(), str::to_string);
        let campioni = vec![Campione {
            id: 1,
            nome,
            scansioni: scans.len(),
        }];
        LettoreAgilent {
            d,
            peak,
            peak_len,
            scans,
            campioni,
            indice,
            srm: None,
        }
    }

    /// The SRM chromatograms of the run, numbered after the TIC, the BPC and the SIC ones.
    fn srm(&mut self) -> Result<&[Chromatogram]> {
        if self.srm.is_none() {
            let first = 2 + self.sim_ions().len();
            let found = match self.peak.as_mut() {
                Some(p) => self.d.srm_chromatograms(p, first)?,
                None => self.d.srm_chromatograms(&mut Vec::<u8>::new(), first)?,
            };
            self.srm = Some(found);
        }
        Ok(self.srm.as_deref().unwrap_or(&[]))
    }

    fn one(&self, c: usize) -> Result<()> {
        (c == 0)
            .then_some(())
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
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
        crate::sortc::slice_ord(&mut keys);
        keys.dedup();
        keys
    }
}

/// The chromatograms of the run without their points: the TIC, the BPC and one SIC per ion of the SIM groups.
fn listed(l: &mut LettoreAgilent) -> Result<Vec<VoceCromatogramma>> {
    let survey = l.d.survey_chromatograms().into_iter();
    let mut v: Vec<VoceCromatogramma> = survey
        .map(|h| VoceCromatogramma {
            id: h.id,
            tipo: h.kind,
            unita: h.unit,
        })
        .collect();
    v.extend(l.sim_ions().into_iter().map(|key| VoceCromatogramma {
        id: format!("SIC {:.2}", key as f64 / 100.0),
        tipo: ChromKind::Sic,
        unita: "number of detector counts".into(),
    }));
    v.extend(l.srm()?.iter().map(|h| VoceCromatogramma {
        id: h.id.clone(),
        tipo: h.kind,
        unita: h.unit.clone(),
    }));
    Ok(v)
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
        let r = self.record(k)?.1.clone();
        let (mz, it) = self.points(&r)?;
        Ok(self.d.spectrum_of(k, &r, mz, it))
    }

    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.one(c)?;
        Ok(header_fields(&self.d, self.record(k)?.1))
    }

    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        self.one(c)?;
        listed(self)
    }

    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.one(c)?;
        trace(self, id)
    }

    fn metodo(&mut self, c: usize) -> Result<String> {
        self.one(c)?;
        Ok(document(self))
    }

    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        self.one(c)?;
        let sha = match (&mut self.peak, self.d.inline.is_some()) {
            (Some(p), false) => sha1_of(&mut Windowed::new(p, self.peak_len), self.peak_len)?,
            _ => String::new(),
        };
        Ok(Some(self.d.header(&sha)))
    }

    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        self.d.extra_of(s)
    }
}
