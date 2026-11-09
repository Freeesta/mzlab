//! A Bruker timsTOF `.d` folder as a `Lettore` (docs/agenti/bruker.md): `analysis.tdf` + `analysis.tdf_bin` (TIMS on) and `analysis.tsf` + `analysis.tsf_bin` (TIMS off, MALDI).
//! The tables come from `sqlite_min`, the peaks of a frame from `blocco` (TDF) or `tsf`, the cut of a frame into spectra from `spettri`.
use super::block::{decode, read_block, Peaks};
use super::header::BrukerHeader;
use super::spectra::{build, summed, Desc};
use super::sqlite_min::Sqlite;
use super::tables::Data;
use super::tdf_helpers::{absent, bad_index, cartella, cut_ones, recognize, sum_scan, take};
use super::tsf;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::method::SrcFile;
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use mzlab_core::readers::{
    ChromatogramEntry, Fiducia, Field, FileSet, Format, Limits, Reader, ReaderEntry, Sample,
    ScanEntry, CONTRATTO,
};

use mzlab_core::mzmlwrite::{Header, Precursore, SpectrumExtra};
use traces::maximum;

#[path = "traces.rs"]
mod traces;

const UNITS: &str = "Arbitrary Intensity Units";

pub const ENTRY: ReaderEntry = ReaderEntry {
    id: "bruker-tdf",
    contratto: CONTRATTO,
    estensioni: &[".d"],
    riconosci: recognize_tdf,
    apri: open_tdf,
};

pub const ENTRY_TSF: ReaderEntry = ReaderEntry {
    id: "bruker-tsf",
    contratto: CONTRATTO,
    estensioni: &[".d"],
    riconosci: recognize_tsf,
    apri: open_tsf,
};

impl BrukerReader {
    fn apri(mut fs: FileSet, limits: &Limits, tsf: bool) -> Result<BrukerReader> {
        let base = if tsf { "analysis.tsf" } else { "analysis.tdf" };
        let bin = format!("{base}_bin");
        let first_one = fs
            .membri
            .first()
            .map(|m| m.nome.clone())
            .unwrap_or_default();
        let Some(db_m) = take(&mut fs, base) else {
            return Err(match fs.find(&bin) {
                Some(_) => absent(&first_one, base),
                None => Error::new("err.file.brukerBaf"),
            });
        };
        let bin_m = take(&mut fs, &bin).ok_or_else(|| absent(&db_m.nome, &bin))?;
        let wal = take(&mut fs, &format!("{base}-wal")).map(|m| (m.dati, m.dim));
        let db = Sqlite::open(db_m.dati, db_m.dim, wal, limits);
        let mut db = db?;
        let data = Data::read(&mut db, limits)?;
        let mut desc = build(&data);
        limits.ok_entries(desc.len() as u64)?;
        // a copy of `*_bin` cut short: the spectra of the frames past its end leave the index and are counted, the rest reads
        let mut bin = bin_m.dati;
        let pos: Vec<i64> = data.frames.iter().map(|f| f.pos).collect();
        let cut = cut_ones(&pos, &mut *bin, bin_m.dim, tsf);
        let all = desc.len();
        desc.retain(|d| !cut[d.frame]);
        let missing_ones = all - desc.len();
        let header = BrukerHeader::read(&data, tsf);
        let compression = data
            .meta
            .get("TimsCompressionType")
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);
        let samples = vec![Sample {
            id: 0,
            nome: cartella(&db_m.nome).unwrap_or("").to_string(),
            scansioni: desc.len(),
            size: None,
        }];
        let mut l = BrukerReader {
            tsf,
            nome: db_m.nome,
            bin_name: bin_m.nome,
            db,
            bin,
            dim_bin: bin_m.dim,
            dim_db: db_m.dim,
            dati: data,
            desc,
            limits: *limits,
            testata: header,
            campioni: samples,
            indice: Vec::new(),
            known_tic: false,
            maxima: Vec::new(),
            cache: None,
            sha1: None,
            compressione: compression,
            missing_ones,
        };
        l.build_index();
        Ok(l)
    }

    fn build_index(&mut self) {
        for d in &self.desc {
            let f = &self.dati.frames[d.frame];
            self.indice.push(ScanEntry {
                id: d.id.clone(),
                rt: f.moment / 60.0,
                livello: d.livello,
                polarita: f.polarity,
                tic: if d.livello == 1 { f.raw_sum() } else { 0.0 },
                base_peak: None,
                filtro: d.filter.clone(),
                precursore: d.mz_prec,
                position: None,
            });
        }
        self.known_tic = self.desc.iter().all(|d| d.livello == 1);
        self.maxima = vec![None; self.desc.len()];
    }

    fn one(&self, c: usize) -> Result<()> {
        (c == 0)
            .then_some(())
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    /// The peaks of frame `fi` (TDF), kept for the next call: the spectra of a frame come one after the other.
    fn peaks(&mut self, fi: usize) -> Result<&Peaks> {
        if self.cache.as_ref().is_none_or(|c| c.0 != fi) {
            let f = &self.dati.frames[fi];
            let b = read_block(
                &mut *self.bin,
                self.dim_bin,
                u64::try_from(f.pos).unwrap_or(u64::MAX),
                &self.limits,
            )?;
            let p = decode(&b, self.compressione, f.scans, f.peaks, &self.limits)?;
            self.cache = Some((fi, p));
        }
        Ok(self
            .cache
            .as_ref()
            .map(|c| &c.1)
            .unwrap_or_else(|| unreachable!()))
    }

    /// TOF positions and intensities of spectrum `d`, and whether they are a profile.
    fn points(&mut self, d: &Desc) -> Result<(Vec<f64>, Vec<f64>, bool)> {
        if self.tsf {
            let f = &self.dati.frames[d.frame];
            let camp = self
                .dati
                .meta
                .get("DigitizerNumSamples")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let l = tsf::read_frame(
                &mut *self.bin,
                self.dim_bin,
                u64::try_from(f.pos).unwrap_or(u64::MAX),
                f.peaks,
                camp,
                &self.limits,
            )?;
            return Ok((l.tof, l.int, l.profile));
        }
        let scan = d.scan.clone();
        let s = summed(self.peaks(d.frame)?, &scan);
        Ok((
            s.iter().map(|x| f64::from(x.0)).collect(),
            s.iter().map(|x| x.1 as f64).collect(),
            false,
        ))
    }

    fn spectrum(&mut self, k: usize) -> Result<Spectrum> {
        let d = self.desc.get(k).cloned().ok_or_else(|| bad_index(k))?;
        let f = self.dati.frames[d.frame].clone();
        let cal = self
            .dati
            .cal
            .get(&f.cal)
            .filter(|c| c.validate())
            .ok_or_else(|| {
                Error::new("err.file.brukerDamaged").with("detail", "no usable MzCalibration")
            })?
            .clone();
        let (tof, y, profilo) = self.points(&d)?;
        self.maxima[k] = Some(maximum(&y));
        let broad = y.iter().any(|&x| x > 16_777_216.0);
        let factor = cal.factor(f.t1, f.t2);
        let mz: Vec<f64> = tof.iter().map(|&t| cal.mz(t) * factor).collect();
        let v = &self.indice[k];
        Ok(Spectrum {
            index: k,
            id: v.id.clone(),
            level: d.livello,
            rt: v.rt,
            polarity: v.polarita,
            profile: profilo,
            filter: v.filtro.clone(),
            precursor: d.mz_prec,
            mz,
            intensity: y.iter().map(|&x| x as f32).collect(),
            wide: broad.then_some(y),
        })
    }

    /// The TIC of the spectra cut from a frame, summed from their points (a frame's own sum is in `Frames`).
    fn complete(&mut self) -> Result<()> {
        if self.known_tic {
            return Ok(());
        }
        for k in 0..self.desc.len() {
            if self.desc[k].livello != 1 {
                // a frame that cannot be read leaves a 0: the error comes when the spectrum itself is asked for
                let d = self.desc[k].clone();
                if let Some(t) = self.sum_points(&d) {
                    self.indice[k].tic = t;
                }
            }
        }
        self.known_tic = true;
        Ok(())
    }

    /// The sum of the intensities of spectrum `d`; for a TDF frame straight from the points of its scan ranges (the counts are whole numbers, so the order of the sum does not matter).
    fn sum_points(&mut self, d: &Desc) -> Option<f64> {
        if self.tsf {
            return self.points(d).ok().map(|(_, y, _)| y.iter().sum());
        }
        let p = self.peaks(d.frame).ok()?;
        Some(sum_scan(p, &d.scan))
    }

    fn file(&mut self) -> (Vec<SrcFile>, Vec<(String, Option<String>)>) {
        if self.sha1.is_none() {
            self.sha1 = Some(self.db.sha1());
        }
        let sha = self.sha1.clone().flatten();
        (
            vec![
                SrcFile {
                    name: self.nome.clone(),
                    sha1: sha.clone(),
                    bytes: self.dim_db,
                },
                SrcFile {
                    name: self.bin_name.clone(),
                    sha1: None,
                    bytes: self.dim_bin,
                },
            ],
            vec![(self.nome.clone(), sha), (self.bin_name.clone(), None)],
        )
    }
}

impl Reader for BrukerReader {
    fn format(&self) -> Format {
        Format::Altro(if self.tsf { "bruker-tsf" } else { "bruker-tdf" })
    }

    fn not_carried(&self) -> &'static [(&'static str, &'static str)] {
        crate::not_carried::BRUKER
    }

    fn samples(&self) -> &[Sample] {
        &self.campioni
    }

    fn missing(&mut self, _c: usize) -> usize {
        self.missing_ones
    }

    fn index(&mut self, c: usize) -> Result<&[ScanEntry]> {
        self.one(c)?;
        self.complete()?;
        Ok(&self.indice)
    }

    // the decoding of every MS2 frame for its TIC is left to the callers that read it
    fn index_without_tic(&mut self, c: usize) -> Result<&[ScanEntry]> {
        self.one(c)?;
        Ok(&self.indice)
    }

    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.one(c)?;
        self.spectrum(k)
    }

    /// Frame, scan range, isolation and collision energy of spectrum `k`.
    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Field>> {
        self.one(c)?;
        let d = self.desc.get(k).ok_or_else(|| bad_index(k))?;
        let f = &self.dati.frames[d.frame];
        let field = |name: &str, value: String, units: Option<&str>, cv: Option<&str>| Field {
            nome: name.into(),
            valore: value,
            unita: units.map(Into::into),
            cv: cv.map(Into::into),
        };
        let mut o = vec![field("frame", f.id.to_string(), None, None)];
        if !self.tsf {
            let r: Vec<String> = d.scan.iter().map(|s| format!("{}-{}", s.0, s.1)).collect();
            o.push(field("mobility scans [begin-end)", r.join(","), None, None));
            o.push(field("accumulation scans", f.scans.to_string(), None, None));
        }
        if let Some(w) = d.iso {
            o.push(field(
                "isolation window target m/z",
                w.mz.to_string(),
                Some("m/z"),
                Some("MS:1000827"),
            ));
            o.push(field(
                "isolation window width",
                w.width.to_string(),
                Some("m/z"),
                None,
            ));
            o.push(field(
                "collision energy",
                w.ce.to_string(),
                Some("eV"),
                Some("MS:1000045"),
            ));
        }
        if let Some(z) = d.load {
            o.push(field(
                "charge state",
                z.to_string(),
                None,
                Some("MS:1000041"),
            ));
        }
        if let Some(p) = d.precursor {
            o.push(field("precursor", p.to_string(), None, None));
        }
        Ok(o)
    }

    fn chromatograms(&mut self, c: usize) -> Result<Vec<ChromatogramEntry>> {
        self.one(c)?;
        Ok(["TIC", "BPC"]
            .into_iter()
            .zip([ChromKind::Tic, ChromKind::Bpc])
            .map(|(id, kind)| ChromatogramEntry {
                id: id.into(),
                tipo: kind,
                unita: UNITS.into(),
            })
            .collect())
    }

    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.one(c)?;
        self.trace(id)
    }

    fn method(&mut self, c: usize) -> Result<String> {
        self.one(c)?;
        let (f, _) = self.file();
        Ok(self.testata.document(f))
    }

    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> {
        self.one(c)?;
        let (f, names) = self.file();
        let doc = self.testata.document(f);
        let folder = cartella(&self.nome).unwrap_or(&self.nome).to_string();
        Ok(Some(self.testata.header(&folder, &names, doc)))
    }

    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        let num = |k: &str| self.dati.meta.get(k).and_then(|v| v.parse::<f64>().ok());
        let mut params = Vec::new();
        let mut precursors = Vec::new();
        if let Some(d) = self.desc.get(s.index) {
            if !self.tsf {
                let r: Vec<String> = d.scan.iter().map(|x| format!("{}-{}", x.0, x.1)).collect();
                params.push(("mzLab mobility scans".to_string(), r.join(",")));
            }
            if let Some(w) = d.iso {
                params.push((
                    "mzLab isolation width (m/z)".to_string(),
                    w.width.to_string(),
                ));
                params.push(("mzLab collision energy (eV)".to_string(), w.ce.to_string()));
                // the window the instrument isolated and the energy it applied; the dissociation method is not said in the file
                let meta = w.width / 2.0;
                precursors.extend(s.precursor.map(|mz| Precursore {
                    selected_mz: Some(mz),
                    iso_target: Some(mz),
                    iso_lower: Some(meta),
                    iso_upper: Some(meta),
                    energy: (w.ce > 0.0).then(|| ("MS:1000045".to_string(), w.ce)),
                    ..Precursore::default()
                }));
            }
        }
        SpectrumExtra {
            sim: false,
            observed: false,
            window: num("MzAcqRangeLower").zip(num("MzAcqRangeUpper")),
            params,
            precursors,
            ..SpectrumExtra::default()
        }
    }
}

fn recognize_tdf(fs: &mut FileSet) -> Fiducia {
    recognize(fs, "analysis.tdf")
}

fn recognize_tsf(fs: &mut FileSet) -> Fiducia {
    recognize(fs, "analysis.tsf")
}

fn open_tdf(fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    BrukerReader::apri(fs, limits, false).map(|l| Box::new(l) as Box<dyn Reader>)
}

fn open_tsf(fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    BrukerReader::apri(fs, limits, true).map(|l| Box::new(l) as Box<dyn Reader>)
}

pub struct BrukerReader {
    tsf: bool,
    /// Name of the database file as given (`x.d/analysis.tdf`).
    nome: String,
    bin_name: String,
    db: Sqlite,
    bin: Box<dyn ReadAt>,
    dim_bin: u64,
    dim_db: u64,
    dati: Data,
    desc: Vec<Desc>,
    limits: Limits,
    testata: BrukerHeader,
    campioni: Vec<Sample>,
    indice: Vec<ScanEntry>,
    known_tic: bool,
    /// The base peak intensity of every spectrum already decoded: the BPC after a full pass (index, mzML) decodes no frame again.
    maxima: Vec<Option<f64>>,
    cache: Option<(usize, Peaks)>,
    sha1: Option<Option<String>>,
    compressione: u32,
    /// Spectra of frames whose block lies past the end of `*_bin`: not in the index.
    missing_ones: usize,
}
