//! A Bruker timsTOF `.d` folder as a `Lettore` (docs/agenti/bruker.md): `analysis.tdf` + `analysis.tdf_bin` (TIMS on) and `analysis.tsf` + `analysis.tsf_bin` (TIMS off, MALDI).
//! The tables come from `sqlite_min`, the peaks of a frame from `blocco` (TDF) or `tsf`, the cut of a frame into spectra from `spettri`.

use super::blocco::{decodifica, leggi_blocco, Picchi};

use super::spettri::{costruisci, somma, Desc};

use super::sqlite_min::Sqlite;

use super::tabelle::Dati;

use super::testata::Testata;

use super::tsf;

use mzlab_core::error::{Error, Result};

use mzlab_core::idxread::ReadAt;

use mzlab_core::lettori::{
    Campione, Campo, Fiducia, FileSet, Formato, Lettore, Limiti, VoceCromatogramma, VoceLettore,
    VoceScansione, CONTRATTO,
};

use mzlab_core::metodo::SrcFile;

use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};

use mzlab_core::mzmlwrite::{Header, SpectrumExtra};

const UNITA: &str = "Arbitrary Intensity Units";

const MAGIA: &[u8; 16] = b"SQLite format 3\0";

pub const VOCE: VoceLettore = VoceLettore {
    id: "bruker-tdf",
    contratto: CONTRATTO,
    estensioni: &[".d"],
    riconosci: riconosci_tdf,
    apri: apri_tdf,
};

pub const VOCE_TSF: VoceLettore = VoceLettore {
    id: "bruker-tsf",
    contratto: CONTRATTO,
    estensioni: &[".d"],
    riconosci: riconosci_tsf,
    apri: apri_tsf,
};

impl LettoreBruker {
    fn apri(mut fs: FileSet, limiti: &Limiti, tsf: bool) -> Result<LettoreBruker> {
        let base = if tsf { "analysis.tsf" } else { "analysis.tdf" };
        let Some(db_m) = prendi(&mut fs, base) else {
            return Err(Error::new("err.file.brukerBaf"));
        };
        let bin_m = prendi(&mut fs, &format!("{base}_bin")).ok_or_else(|| {
            Error::new("err.dam.unreadable").with("message", format!("no {base}_bin"))
        })?;
        let wal = prendi(&mut fs, &format!("{base}-wal")).map(|m| (m.dati, m.dim));
        let db = Sqlite::apri(db_m.dati, db_m.dim, wal, limiti);
        let mut db = db?;
        let dati = Dati::leggi(&mut db, limiti)?;
        let desc = costruisci(&dati);
        limiti.voci_ok(desc.len() as u64)?;
        let testata = Testata::leggi(&dati, tsf);
        let compressione = dati
            .meta
            .get("TimsCompressionType")
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);
        let campioni = vec![Campione {
            id: 0,
            nome: db_m.nome.rsplit_once('/').map_or("", |x| x.0).to_string(),
            scansioni: desc.len(),
        }];
        let mut l = LettoreBruker {
            tsf,
            nome: db_m.nome,
            nome_bin: bin_m.nome,
            db,
            bin: bin_m.dati,
            dim_bin: bin_m.dim,
            dim_db: db_m.dim,
            dati,
            desc,
            limiti: *limiti,
            testata,
            campioni,
            indice: Vec::new(),
            tic_noto: false,
            cache: None,
            sha1: None,
            compressione,
        };
        l.costruisci_indice();
        Ok(l)
    }

    fn costruisci_indice(&mut self) {
        for d in &self.desc {
            let f = &self.dati.frames[d.frame];
            self.indice.push(VoceScansione {
                id: d.id.clone(),
                rt: f.tempo / 60.0,
                livello: d.livello,
                polarita: f.polarita,
                tic: if d.livello == 1 {
                    f.somma_grezza()
                } else {
                    0.0
                },
                picco_base: None,
                filtro: d.filtro.clone(),
                precursore: d.mz_prec,
                posizione: None,
            });
        }
        self.tic_noto = self.desc.iter().all(|d| d.livello == 1);
    }

    fn uno(&self, c: usize) -> Result<()> {
        (c == 0)
            .then_some(())
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    /// The peaks of frame `fi` (TDF), kept for the next call: the spectra of a frame come one after the other.
    fn picchi(&mut self, fi: usize) -> Result<&Picchi> {
        if self.cache.as_ref().is_none_or(|c| c.0 != fi) {
            let f = &self.dati.frames[fi];
            let b = leggi_blocco(
                &mut *self.bin,
                self.dim_bin,
                u64::try_from(f.pos).unwrap_or(u64::MAX),
                &self.limiti,
            )?;
            let p = decodifica(&b, self.compressione, f.scansioni, f.picchi, &self.limiti)?;
            self.cache = Some((fi, p));
        }
        Ok(self
            .cache
            .as_ref()
            .map(|c| &c.1)
            .unwrap_or_else(|| unreachable!()))
    }

    /// TOF positions and intensities of spectrum `d`, and whether they are a profile.
    fn punti(&mut self, d: &Desc) -> Result<(Vec<f64>, Vec<f64>, bool)> {
        if self.tsf {
            let f = &self.dati.frames[d.frame];
            let camp = self
                .dati
                .meta
                .get("DigitizerNumSamples")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let l = tsf::leggi_frame(
                &mut *self.bin,
                self.dim_bin,
                u64::try_from(f.pos).unwrap_or(u64::MAX),
                f.picchi,
                camp,
                &self.limiti,
            )?;
            return Ok((l.tof, l.int, l.profilo));
        }
        let scan = d.scan.clone();
        let s = somma(self.picchi(d.frame)?, &scan);
        Ok((
            s.iter().map(|x| f64::from(x.0)).collect(),
            s.iter().map(|x| x.1 as f64).collect(),
            false,
        ))
    }

    fn spettro(&mut self, k: usize) -> Result<Spectrum> {
        let d = self.desc.get(k).cloned().ok_or_else(|| indice_cattivo(k))?;
        let f = self.dati.frames[d.frame].clone();
        let cal = self
            .dati
            .cal
            .get(&f.cal)
            .filter(|c| c.valida())
            .ok_or_else(|| {
                Error::new("err.dam.unreadable").with("message", "no usable MzCalibration")
            })?
            .clone();
        let (tof, y, profilo) = self.punti(&d)?;
        let largo = y.iter().any(|&x| x > 16_777_216.0);
        let fattore = cal.fattore(f.t1, f.t2);
        let mz: Vec<f64> = tof.iter().map(|&t| cal.mz(t) * fattore).collect();
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
            wide: largo.then(|| y.clone()),
        })
    }

    /// The TIC of the spectra cut from a frame, summed from their points (a frame's own sum is in `Frames`).
    fn completa(&mut self) -> Result<()> {
        if self.tic_noto {
            return Ok(());
        }
        for k in 0..self.desc.len() {
            if self.desc[k].livello != 1 {
                // a frame that cannot be read leaves a 0: the error comes when the spectrum itself is asked for
                let d = self.desc[k].clone();
                if let Ok((_, y, _)) = self.punti(&d) {
                    self.indice[k].tic = y.iter().sum();
                }
            }
        }
        self.tic_noto = true;
        Ok(())
    }

    fn traccia(&mut self, id: &str) -> Result<Chromatogram> {
        let (tic, kind, index) = match id {
            "TIC" => (true, ChromKind::Tic, 0),
            "BPC" => (false, ChromKind::Bpc, 1),
            _ => return Err(Error::new("error.badIndex").with("id", id)),
        };
        // the trace follows the survey scans; a run with none (MRM, PRM) is traced over all
        let ms1 = self.desc.iter().any(|d| d.livello == 1);
        let ks: Vec<usize> = (0..self.desc.len())
            .filter(|&k| !ms1 || self.desc[k].livello == 1)
            .collect();
        self.completa()?;
        let mut intensity = Vec::with_capacity(ks.len());
        for &k in &ks {
            intensity.push(if tic {
                self.indice[k].tic
            } else {
                self.spettro(k)?
                    .intensity
                    .iter()
                    .fold(0.0f64, |m, &x| m.max(f64::from(x)))
            });
        }
        Ok(Chromatogram {
            index,
            id: id.into(),
            kind,
            q1: None,
            q3: None,
            unit: UNITA.into(),
            time: ks.iter().map(|&k| self.indice[k].rt).collect(),
            intensity,
        })
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
                    name: self.nome_bin.clone(),
                    sha1: None,
                    bytes: self.dim_bin,
                },
            ],
            vec![(self.nome.clone(), sha), (self.nome_bin.clone(), None)],
        )
    }
}

impl Lettore for LettoreBruker {
    fn formato(&self) -> Formato {
        Formato::Altro(if self.tsf { "bruker-tsf" } else { "bruker-tdf" })
    }

    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }

    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        self.uno(c)?;
        self.completa()?;
        Ok(&self.indice)
    }

    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.uno(c)?;
        self.spettro(k)
    }

    /// Frame, scan range, isolation and collision energy of spectrum `k`.
    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.uno(c)?;
        let d = self.desc.get(k).ok_or_else(|| indice_cattivo(k))?;
        let f = &self.dati.frames[d.frame];
        let campo = |nome: &str, valore: String, unita: Option<&str>, cv: Option<&str>| Campo {
            nome: nome.into(),
            valore,
            unita: unita.map(Into::into),
            cv: cv.map(Into::into),
        };
        let mut o = vec![campo("frame", f.id.to_string(), None, None)];
        if !self.tsf {
            let r: Vec<String> = d.scan.iter().map(|s| format!("{}-{}", s.0, s.1)).collect();
            o.push(campo("mobility scans [begin-end)", r.join(","), None, None));
            o.push(campo(
                "accumulation scans",
                f.scansioni.to_string(),
                None,
                None,
            ));
        }
        if let Some(w) = d.iso {
            o.push(campo(
                "isolation window target m/z",
                w.mz.to_string(),
                Some("m/z"),
                Some("MS:1000827"),
            ));
            o.push(campo(
                "isolation window width",
                w.larghezza.to_string(),
                Some("m/z"),
                None,
            ));
            o.push(campo(
                "collision energy",
                w.ce.to_string(),
                Some("eV"),
                Some("MS:1000045"),
            ));
        }
        if let Some(z) = d.carica {
            o.push(campo(
                "charge state",
                z.to_string(),
                None,
                Some("MS:1000041"),
            ));
        }
        if let Some(p) = d.precursore {
            o.push(campo("precursor", p.to_string(), None, None));
        }
        Ok(o)
    }

    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        self.uno(c)?;
        Ok(["TIC", "BPC"]
            .into_iter()
            .zip([ChromKind::Tic, ChromKind::Bpc])
            .map(|(id, tipo)| VoceCromatogramma {
                id: id.into(),
                tipo,
                unita: UNITA.into(),
            })
            .collect())
    }

    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.uno(c)?;
        self.traccia(id)
    }

    fn metodo(&mut self, c: usize) -> Result<String> {
        self.uno(c)?;
        let (f, _) = self.file();
        Ok(self.testata.documento(f))
    }

    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        self.uno(c)?;
        let (f, nomi) = self.file();
        let doc = self.testata.documento(f);
        let cartella = self
            .nome
            .rsplit_once('/')
            .map_or(self.nome.as_str(), |x| x.0)
            .to_string();
        Ok(Some(self.testata.header(&cartella, &nomi, doc)))
    }

    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        let num = |k: &str| self.dati.meta.get(k).and_then(|v| v.parse::<f64>().ok());
        let mut params = Vec::new();
        if let Some(d) = self.desc.get(s.index) {
            if !self.tsf {
                let r: Vec<String> = d.scan.iter().map(|x| format!("{}-{}", x.0, x.1)).collect();
                params.push(("mzLab mobility scans".to_string(), r.join(",")));
            }
            if let Some(w) = d.iso {
                params.push((
                    "mzLab isolation width (m/z)".to_string(),
                    w.larghezza.to_string(),
                ));
                params.push(("mzLab collision energy (eV)".to_string(), w.ce.to_string()));
            }
        }
        SpectrumExtra {
            sim: false,
            observed: false,
            window: num("MzAcqRangeLower").zip(num("MzAcqRangeUpper")),
            params,
        }
    }
}

fn riconosci_tdf(fs: &mut FileSet) -> Fiducia {
    riconosci(fs, "analysis.tdf")
}

fn riconosci_tsf(fs: &mut FileSet) -> Fiducia {
    riconosci(fs, "analysis.tsf")
}

/// A folder is Bruker's when its database starts with the SQLite magic and the file of the peaks is beside it; a folder with only `analysis.baf` is recognised to say why it does not open.
fn riconosci(fs: &mut FileSet, base: &str) -> Fiducia {
    let Some(i) = fs.trova(base) else {
        return if fs.trova("analysis.baf").is_some()
            && fs.trova("analysis.tdf").is_none()
            && fs.trova("analysis.tsf").is_none()
        {
            Fiducia::Forse
        } else {
            Fiducia::No
        };
    };
    if fs.testa(i, 16) != MAGIA || fs.trova(&format!("{base}_bin")).is_none() {
        return Fiducia::No;
    }
    Fiducia::Si
}

fn apri_tdf(fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    LettoreBruker::apri(fs, limiti, false).map(|l| Box::new(l) as Box<dyn Lettore>)
}

fn apri_tsf(fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    LettoreBruker::apri(fs, limiti, true).map(|l| Box::new(l) as Box<dyn Lettore>)
}

fn prendi(fs: &mut FileSet, suffisso: &str) -> Option<mzlab_core::lettori::Membro> {
    let i = fs.trova(suffisso)?;
    Some(fs.membri.remove(i))
}

fn indice_cattivo(k: impl std::fmt::Display) -> Error {
    Error::new("error.badIndex").with("k", k)
}

pub struct LettoreBruker {
    tsf: bool,
    /// Name of the database file as given (`x.d/analysis.tdf`).
    nome: String,
    nome_bin: String,
    db: Sqlite,
    bin: Box<dyn ReadAt>,
    dim_bin: u64,
    dim_db: u64,
    dati: Dati,
    desc: Vec<Desc>,
    limiti: Limiti,
    testata: Testata,
    campioni: Vec<Campione>,
    indice: Vec<VoceScansione>,
    tic_noto: bool,
    cache: Option<(usize, Picchi)>,
    sha1: Option<Option<String>>,
    compressione: u32,
}
