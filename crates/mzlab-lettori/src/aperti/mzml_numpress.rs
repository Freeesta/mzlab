//! An mzML whose arrays are MS-Numpress (msconvert `--numpressLinear/--numpressPic/--numpressSlof/--numpressAll`), as a `Lettore`. The engine of the page refuses it
//! (`err.file.numpress`); the module reads it and `lettori_mzml::scrivi` writes it again as a plain mzML (zlib) in OPFS (rule R, docs/agenti/lettori-modulo.md).
//! The index comes from one pass over the offsets (`mzlab_core::scantable`, the headers only); `scansione(c, k)` reads the bytes of spectrum `k` and gives them to `mzml::parse`,
//! whose numpress decoding is on in this build (`mzlab_core::numpress`, feature `estesi`). Reference of the codecs: the MS-Numpress documentation; the code is written from it.

use std::cell::RefCell;

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{
    Campione, Campo, Fiducia, FileSet, Formato, Lettore, Limiti, Posizione, VoceCromatogramma,
    VoceLettore, VoceScansione, CONTRATTO,
};
use mzlab_core::metodo::{self, sha1_hex};
use mzlab_core::mzml::{self, ChromKind, Chromatogram, Item, Spectrum};
use mzlab_core::mzmlhead::{Chrom, Mzml};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};
use mzlab_core::scantable::{self, Row};

use super::mzml_testata::{extra_di, parametri, testata_da};
use super::np_riscrivi;

/// The accessions that make an array numpress (the three codecs, alone and followed by zlib).
const ACCESSION: [&str; 6] = [
    "MS:1002312",
    "MS:1002313",
    "MS:1002314",
    "MS:1002746",
    "MS:1002747",
    "MS:1002748",
];
/// The bytes of the start of the file that `riconosci` looks at.
const TESTA: usize = 64 << 10;
pub const VOCE: VoceLettore = VoceLettore {
    id: "mzml-numpress",
    contratto: CONTRATTO,
    estensioni: &[".mzml"],
    riconosci,
    apri,
};

/// A borrowed `ReadAt` (`Mzml` owns its reader).
struct Prestito<'a>(&'a mut dyn ReadAt);

impl ReadAt for Prestito<'_> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.0.read_at(off, buf)
    }
}

pub struct LettoreNumpress {
    dati: Box<dyn ReadAt>,
    dim: u64,
    nome: String,
    limiti: Limiti,
    campioni: Vec<Campione>,
    righe: Vec<Row>,
    indice: Vec<VoceScansione>,
    tracce: Option<Vec<Chrom>>,
    /// SHA-1 of the file, read the first time the header of the mzML is asked for (`Some(None)`: it could not be read).
    sha1: Option<Option<String>>,
    /// The extra of the spectrum `scansione` read last (the writer asks for it right after).
    ultima: RefCell<Option<(usize, SpectrumExtra)>>,
}

impl LettoreNumpress {
    fn uno(&self, c: usize) -> Result<()> {
        (c == 0)
            .then_some(())
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    /// The bytes of spectrum `k`.
    fn byte(&mut self, k: usize) -> Result<Vec<u8>> {
        let r = self
            .righe
            .get(k)
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        let n = r.end - r.start;
        if n > self.limiti.byte {
            return Err(Error::memory_budget());
        }
        let mut b = Vec::new();
        b.try_reserve_exact(n as usize)
            .map_err(|_| Error::memory_budget())?;
        b.resize(n as usize, 0);
        self.dati.read_at(r.start, &mut b)?;
        Ok(b)
    }

    fn cromatogrammi_del_file(&mut self) -> Result<&[Chrom]> {
        if self.tracce.is_none() {
            let fine = self.righe.last().map_or(0, |r| r.end);
            let mut m = Mzml::new(Prestito(&mut *self.dati), self.dim);
            self.tracce = Some(m.chromatograms(fine)?);
        }
        Ok(self.tracce.as_deref().unwrap_or_default())
    }

    fn testa_xml(&mut self) -> Result<String> {
        Mzml::new(Prestito(&mut *self.dati), self.dim).run_head(1 << 20)
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

impl LettoreNumpress {
    /// The chromatograms of the file with their kind and unit.
    fn voci_tracce(&mut self) -> Result<Vec<VoceCromatogramma>> {
        let tracce = self.cromatogrammi_del_file()?.to_vec();
        let mut m = Mzml::new(Prestito(&mut *self.dati), self.dim);
        let mut out = Vec::new();
        for t in tracce {
            out.push(VoceCromatogramma {
                tipo: tipo(&t.kind),
                unita: m.chromatogram_unit(&t)?,
                id: t.id,
            });
        }
        Ok(out)
    }

    /// The chromatogram `id`, read from its bytes with the numpress arrays plain.
    fn traccia(&mut self, id: &str) -> Result<Chromatogram> {
        let t = self
            .cromatogrammi_del_file()?
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| Error::new("error.badIndex").with("id", id))?;
        // the bytes of the chromatogram with its numpress arrays plain, read as a file of its own
        if t.end <= t.start || t.end - t.start > self.limiti.byte {
            return Err(Error::memory_budget());
        }
        let mut pezzo =
            vec![0u8; usize::try_from(t.end - t.start).map_err(|_| Error::memory_budget())?];
        self.dati.read_at(t.start, &mut pezzo)?;
        let semplice = np_riscrivi::riscrivi(&pezzo, &self.limiti)?.into_owned();
        let lunghezza = semplice.len() as u64;
        let locale = Chrom {
            start: 0,
            end: lunghezza,
            ..t.clone()
        };
        let mut m = Mzml::new(semplice, lunghezza);
        let unit = m.chromatogram_unit(&locale)?;
        let (time, intensity) = m.chromatogram(&locale)?;
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

impl LettoreNumpress {
    /// Spectrum `k`: its bytes with the numpress arrays plain, read by the engine's parser; its extra is kept for the writer.
    fn spettro(&mut self, k: usize) -> Result<Spectrum> {
        let byte = self.byte(k)?;
        let semplice = np_riscrivi::riscrivi(&byte, &self.limiti)?;
        let mut spettro = None;
        mzml::parse(&semplice[..], |it| {
            if let Item::Spectrum(s) = it {
                spettro = Some(s);
            }
        })?;
        let s = spettro
            .ok_or_else(|| Error::new("err.file.xml").with("detail", "no spectrum in its bytes"))?;
        *self.ultima.borrow_mut() = Some((k, extra_di(&byte)));
        Ok(s)
    }

    /// The header of the mzML we write (`testata_da`), with the SHA-1 of the file when the original names no source.
    fn testata(&mut self) -> Result<Header> {
        if self.sha1.is_none() {
            let dim = self.dim;
            self.sha1 = Some(sha1_hex(&mut self.dati, dim).ok());
        }
        let sha1 = self.sha1.clone().flatten();
        let head = self.testa_xml()?;
        let mut h = testata_da(&head, &self.nome, &self.limiti)?;
        if h.sources.is_empty() {
            h.sources = vec![self.nome.clone()];
            h.source_sha1 = sha1.into_iter().collect();
        }
        if let Some(doc) = metodo::from_userparam(&head) {
            h.run_params.retain(|(k, _)| k != metodo::USERPARAM);
            h.run_params.push((metodo::USERPARAM.to_string(), doc));
        }
        Ok(h)
    }
}

fn riconosci(fs: &mut FileSet) -> Fiducia {
    let Some(i) = fs.trova(".mzml") else {
        return Fiducia::No;
    };
    let head = fs.testa(i, TESTA);
    let text = String::from_utf8_lossy(&head);
    let numpress = ACCESSION
        .iter()
        .any(|a| text.contains(&format!("accession=\"{a}\"")));
    if text.contains("<mzML") && numpress {
        Fiducia::Si
    } else {
        Fiducia::No
    }
}

impl Lettore for LettoreNumpress {
    fn formato(&self) -> Formato {
        Formato::Altro("mzml-numpress")
    }

    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }

    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        self.uno(c)?;
        Ok(&self.indice)
    }

    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.uno(c)?;
        self.spettro(k)
    }

    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.uno(c)?;
        let byte = self.byte(k)?;
        Ok(parametri(&byte)
            .into_iter()
            .map(|p| field(&p.nome, p.valore, p.unita.as_deref(), Some(&p.acc)))
            .collect())
    }

    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        self.uno(c)?;
        self.voci_tracce()
    }

    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.uno(c)?;
        self.traccia(id)
    }

    /// The `metodo/1` document the mzML carries (written by mzLab); an mzML of another program has none (`{}`: no section).
    fn metodo(&mut self, c: usize) -> Result<String> {
        self.uno(c)?;
        let head = self.testa_xml()?;
        Ok(metodo::from_userparam(&head).unwrap_or_else(|| "{}".to_string()))
    }

    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        self.uno(c)?;
        self.testata().map(Some)
    }

    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        match self.ultima.borrow().as_ref() {
            Some((k, e)) if self.indice.get(*k).is_some_and(|v| v.id == s.id) => e.clone(),
            _ => SpectrumExtra::default(),
        }
    }
}

fn apri(mut fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    let i = fs
        .trova(".mzml")
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
    let tabella = scantable::read(&mut dati, dim)?;
    limiti.voci_ok(tabella.rows.len() as u64)?;
    let mut indice = Vec::with_capacity(tabella.rows.len());
    for (k, r) in tabella.rows.iter().enumerate() {
        if r.end <= r.start || r.end > dim {
            return Err(Error::new("err.file.xml").with("detail", "spectrum out of the file"));
        }
        let lunghezza = u32::try_from(r.end - r.start).map_err(|_| Error::memory_budget())?;
        indice.push(VoceScansione {
            id: if r.native.is_empty() {
                format!("index={k}")
            } else {
                r.native.clone()
            },
            rt: r.rt,
            livello: r.level,
            polarita: r.polarity,
            tic: r.tic,
            picco_base: None,
            filtro: r.filter.clone(),
            precursore: r.precursor,
            posizione: Some(Posizione {
                membro: 0,
                offset: r.start,
                lunghezza,
            }),
        });
    }
    let stem = nome
        .rsplit_once('.')
        .map_or(nome.as_str(), |(s, _)| s)
        .to_string();
    Ok(Box::new(LettoreNumpress {
        dati,
        dim,
        campioni: vec![Campione {
            id: 1,
            nome: stem,
            scansioni: indice.len(),
        }],
        nome,
        limiti: *limiti,
        righe: tabella.rows,
        indice,
        tracce: None,
        sha1: None,
        ultima: RefCell::new(None),
    }))
}
