//! An mzXML file (ISB, versions 2.x and 3.x) as a `Lettore`: spectra with level, precursor, polarity and retention time, TIC and BPC chromatograms computed from the scans, the header of the file
//! as `metodo/1`. The index comes from `<index name="scan">` or from one pass over the bytes (`mzxml_indice.rs`); `scansione(c, k)` reads only the bytes of scan `k` (`mzxml_scan.rs`).
//! SRM/MRM scans become chromatograms through `mzlab_lettori::srm::SrmDaScansioni`, which wraps every reader of this crate. Written from the mzXML schema (ISB); not from any program's code.

use std::cell::RefCell;

use mzlab_core::error::{Error, Result};

use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{
    Campione, Campo, Fiducia, FileSet, Formato, Lettore, Limiti, Posizione, VoceCromatogramma,
    VoceLettore, VoceScansione, CONTRATTO,
};

use mzlab_core::metodo::sha1_hex;
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};

use mzlab_core::mzmlwrite::{Header, SpectrumExtra};

use super::mzxml_indice::{self, Riga};

use super::mzxml_scan::{self as scan, attr, each, Tag, Testa};

use super::mzxml_testata::{self as testata, Cabina};

/// The bytes of the start of the file that `riconosci` looks at.
const TESTA: usize = 4096;
/// The most bytes of the start of a scan read to learn its attributes (the peaks come after them), and of the header of the file.
const TESTA_SCANSIONE: u64 = 1 << 20;
const UNITA: &str = "number of detector counts";
/// The most `nameValue`/`precursorMz` elements kept for a scan.
const TETTO: usize = 256;

pub const VOCE: VoceLettore = VoceLettore {
    id: "mzxml",
    contratto: CONTRATTO,
    estensioni: &[".mzxml"],
    riconosci,
    apri,
};

impl LettoreMzxml {
    fn uno(&self, c: usize) -> Result<()> {
        (c == 0)
            .then_some(())
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    fn leggi(&mut self, da: u64, n: u64) -> Result<Vec<u8>> {
        let mut b = Vec::new();
        b.try_reserve_exact(usize::try_from(n).map_err(|_| Error::memory_budget())?)
            .map_err(|_| Error::memory_budget())?;
        b.resize(n as usize, 0);
        self.dati.read_at(da, &mut b)?;
        Ok(b)
    }

    /// The bytes of scan `k`; an index entry that does not point at a `<scan` is an error with a key.
    fn byte(&mut self, k: usize) -> Result<Vec<u8>> {
        let r = *self
            .righe
            .get(k)
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        if r.end - r.start > self.limiti.byte {
            return Err(Error::memory_budget());
        }
        let b = self.leggi(r.start, r.end - r.start)?;
        if !b.starts_with(b"<scan") {
            return Err(Error::new("err.file.xml").with("detail", "index offset is not a scan"));
        }
        Ok(b)
    }

    /// Scan `k`: its attributes and its peaks, and the extra of its mzML spectrum.
    fn spettro(&mut self, k: usize) -> Result<(Spectrum, SpectrumExtra)> {
        let byte = self.byte(k)?;
        let testo = String::from_utf8_lossy(&byte);
        let t = scan::testa(&testo, TETTO)?;
        let conteggio = numero(t.scan, "peaksCount").map(|n| n.max(0.0) as usize);
        let p = match t.picchi {
            Some((g, corpo)) => scan::picchi(g, corpo, conteggio, &self.limiti)?,
            None => scan::Picchi {
                mz: Vec::new(),
                intensita: Vec::new(),
                a_64_bit: false,
            },
        };
        let v = self
            .indice
            .get(k)
            .cloned()
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        let profilo = match attr(t.scan, "centroided") {
            "1" => false,
            "0" => true,
            _ => self.cabina.centroidi == Some(false),
        };
        let s = Spectrum {
            index: k,
            id: v.id,
            level: v.livello,
            rt: v.rt,
            polarity: v.polarita,
            profile: profilo,
            filter: v.filtro,
            precursor: v.precursore,
            intensity: p.intensita.iter().map(|&x| x as f32).collect(),
            wide: p.a_64_bit.then(|| p.intensita.clone()),
            mz: p.mz,
        };
        Ok((s, extra_di(&t)))
    }
}

impl LettoreMzxml {
    /// Fills in the TIC (`tic`) or the base peak intensity of the scans whose attributes do not say it, from their peaks.
    fn completa(&mut self, tic: bool) -> Result<()> {
        for k in 0..self.indice.len() {
            let manca = if tic {
                !self.tic_noto[k]
            } else {
                self.bpi[k].is_none()
            };
            if !manca {
                continue;
            }
            let (s, _) = self.spettro(k)?;
            if tic {
                self.indice[k].tic = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
                self.tic_noto[k] = true;
            } else {
                self.bpi[k] = Some(
                    (0..s.mz.len())
                        .map(|i| s.intensity_f64(i))
                        .fold(0.0, f64::max),
                );
            }
        }
        Ok(())
    }

    /// The scans a chromatogram sums: the MS1 ones, or all of them when the file has no MS1.
    fn scelte(&self) -> Vec<usize> {
        let uno: Vec<usize> = (0..self.indice.len())
            .filter(|&k| self.indice[k].livello == 1)
            .collect();
        if uno.is_empty() {
            (0..self.indice.len()).collect()
        } else {
            uno
        }
    }

    fn traccia(&mut self, id: &str) -> Result<Chromatogram> {
        let (tic, kind, index) = match id {
            "TIC" => (true, ChromKind::Tic, 0),
            "BPC" => (false, ChromKind::Bpc, 1),
            _ => return Err(Error::new("error.badIndex").with("id", id)),
        };
        self.completa(tic)?;
        let k = self.scelte();
        Ok(Chromatogram {
            index,
            id: id.to_string(),
            kind,
            q1: None,
            q3: None,
            unit: UNITA.to_string(),
            time: k.iter().map(|&i| self.indice[i].rt).collect(),
            intensity: k
                .iter()
                .map(|&i| {
                    if tic {
                        self.indice[i].tic
                    } else {
                        self.bpi[i].unwrap_or(0.0)
                    }
                })
                .collect(),
        })
    }

    fn sha1(&mut self) -> Option<String> {
        if self.sha1.is_none() {
            let dim = self.dim;
            self.sha1 = Some(sha1_hex(&mut self.dati, dim).ok());
        }
        self.sha1.clone().flatten()
    }
}

impl Lettore for LettoreMzxml {
    fn formato(&self) -> Formato {
        Formato::Altro("mzxml")
    }

    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }

    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        self.uno(c)?;
        self.completa(true)?;
        Ok(&self.indice)
    }

    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.uno(c)?;
        let (s, e) = self.spettro(k)?;
        *self.ultima.borrow_mut() = Some((k, e));
        Ok(s)
    }

    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.uno(c)?;
        let byte = self.byte(k)?;
        let testo = String::from_utf8_lossy(&byte);
        let t = scan::testa(&testo, TETTO)?;
        let mut out = Vec::new();
        let aggiungi = |out: &mut Vec<Campo>, pre: &str, g: Tag| {
            each(g, &mut |n, v| out.push(campo(&format!("{pre}{n}"), v)))
        };
        aggiungi(&mut out, "", t.scan);
        for (g, corpo) in &t.precursori {
            aggiungi(&mut out, "precursorMz.", g);
            out.push(campo("precursorMz", corpo));
        }
        if let Some((g, _)) = t.picchi {
            aggiungi(&mut out, "peaks.", g);
        }
        for g in &t.valori {
            out.push(campo(attr(g, "name"), attr(g, "value")));
        }
        Ok(out)
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
        let sha1 = self.sha1();
        Ok(testata::documento(&self.cabina, &self.nome, self.dim, sha1))
    }

    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        self.uno(c)?;
        let sha1 = self.sha1();
        let doc = testata::documento(&self.cabina, &self.nome, self.dim, sha1.clone());
        Ok(Some(testata::header(&self.cabina, &self.nome, sha1, doc)))
    }

    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        match self.ultima.borrow().as_ref() {
            Some((k, e)) if *k == s.index => e.clone(),
            _ => SpectrumExtra::default(),
        }
    }
}

impl LettoreMzxml {
    /// The header of the file (before the first scan).
    fn leggi_testa(&mut self) -> Result<()> {
        let fine = self.righe[0].start.min(TESTA_SCANSIONE);
        let b = self.leggi(0, fine)?;
        self.cabina = testata::leggi(&String::from_utf8_lossy(&b), &self.limiti)?;
        Ok(())
    }

    /// The index entries, from the attributes at the start of each scan.
    fn costruisci_indice(&mut self) -> Result<()> {
        for k in 0..self.righe.len() {
            let r = self.righe[k];
            let mut n = 4096u64;
            let (noto, bpi, v) = loop {
                let b = self.leggi(r.start, (r.end - r.start).min(n))?;
                if b.windows(6).any(|w| w == b"<peaks")
                    || n >= r.end - r.start
                    || n >= TESTA_SCANSIONE
                {
                    let testo = String::from_utf8_lossy(&b);
                    let t = scan::testa(&testo, TETTO)?;
                    break (
                        numero(t.scan, "totIonCurrent").is_some(),
                        numero(t.scan, "basePeakIntensity"),
                        voce(k, r, &t)?,
                    );
                }
                n *= 4;
            };
            self.tic_noto.push(noto);
            self.bpi.push(bpi);
            self.indice.push(v);
        }
        Ok(())
    }
}

pub struct LettoreMzxml {
    dati: Box<dyn ReadAt>,
    dim: u64,
    nome: String,
    limiti: Limiti,
    campioni: Vec<Campione>,
    righe: Vec<Riga>,
    indice: Vec<VoceScansione>,
    /// Whether the file says the TIC / the base peak intensity of each scan (`false`: computed from the peaks when asked).
    tic_noto: Vec<bool>,
    bpi: Vec<Option<f64>>,
    cabina: Cabina,
    sha1: Option<Option<String>>,
    /// The extra of the spectrum `scansione` read last (the writer asks for it right after).
    ultima: RefCell<Option<(usize, SpectrumExtra)>>,
}

fn riconosci(fs: &mut FileSet) -> Fiducia {
    let Some(i) = fs.trova(".mzxml") else {
        return Fiducia::No;
    };
    let head = fs.testa(i, TESTA);
    if String::from_utf8_lossy(&head).contains("<mzXML") {
        Fiducia::Si
    } else {
        Fiducia::No
    }
}

fn numero(t: Tag, n: &str) -> Option<f64> {
    attr(t, n)
        .trim()
        .parse()
        .ok()
        .filter(|x: &f64| x.is_finite())
}

fn polarita(t: &str) -> i8 {
    match t.trim() {
        "+" | "positive" | "Positive" => 1,
        "-" | "negative" | "Negative" => -1,
        _ => 0,
    }
}

/// The filter of a scan: its `filterLine`, with the scan type in front for SRM/MRM scans whose line does not say it (the SRM traces are found by it).
fn filtro(t: Tag) -> String {
    let linea = attr(t, "filterLine").trim();
    let tipo = attr(t, "scanType").trim();
    let basso = linea.to_ascii_lowercase();
    let srm = ["srm", "mrm", "crm"]
        .iter()
        .any(|x| tipo.eq_ignore_ascii_case(x));
    if srm && !basso.contains("srm") && !basso.contains("mrm") {
        format!("{tipo} {linea}").trim().to_string()
    } else {
        linea.to_string()
    }
}

/// The index entry of scan `k` from its attributes.
fn voce(k: usize, r: Riga, t: &Testa) -> Result<VoceScansione> {
    let lunghezza = u32::try_from(r.end - r.start).map_err(|_| Error::memory_budget())?;
    let num = attr(t.scan, "num").trim();
    Ok(VoceScansione {
        id: if num.is_empty() {
            format!("scan={}", k + 1)
        } else {
            format!("scan={num}")
        },
        rt: scan::minuti(attr(t.scan, "retentionTime")).unwrap_or(0.0),
        livello: numero(t.scan, "msLevel").map_or(1, |l| l.clamp(1.0, 255.0) as u8),
        polarita: polarita(attr(t.scan, "polarity")),
        tic: numero(t.scan, "totIonCurrent").unwrap_or(0.0),
        picco_base: numero(t.scan, "basePeakMz"),
        filtro: filtro(t.scan),
        precursore: t.precursori.iter().rev().find_map(|p| p.1.parse().ok()),
        posizione: Some(Posizione {
            membro: 0,
            offset: r.start,
            lunghezza,
        }),
    })
}

/// The `userParam`s and the window of a scan for the mzML we write.
fn extra_di(t: &Testa) -> SpectrumExtra {
    let mut e = SpectrumExtra {
        sim: attr(t.scan, "scanType").eq_ignore_ascii_case("SIM"),
        observed: numero(t.scan, "lowMz").is_some() && numero(t.scan, "highMz").is_some(),
        window: numero(t.scan, "startMz").zip(numero(t.scan, "endMz")),
        params: Vec::new(),
    };
    let mut aggiungi = |g: Tag, nomi: &[&str]| {
        for n in nomi {
            if !attr(g, n).is_empty() {
                e.params
                    .push((format!("mzXML {n}"), attr(g, n).to_string()));
            }
        }
    };
    aggiungi(t.scan, &["collisionEnergy", "scanType", "filterLine"]);
    for p in &t.precursori {
        aggiungi(
            p.0,
            &[
                "precursorCharge",
                "precursorIntensity",
                "activationMethod",
                "windowWideness",
            ],
        );
    }
    e
}

fn campo(nome: &str, valore: &str) -> Campo {
    Campo {
        nome: nome.into(),
        valore: valore.into(),
        unita: None,
        cv: None,
    }
}

fn apri(mut fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    let i = fs
        .trova(".mzxml")
        .ok_or_else(|| Error::new("err.file.notAccepted"))?;
    let m = fs
        .membri
        .get_mut(i)
        .ok_or_else(|| Error::new("err.file.notAccepted"))?;
    let nome = m
        .nome
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("x.mzXML")
        .to_string();
    let dim = m.dim;
    let mut dati: Box<dyn ReadAt> = std::mem::replace(&mut m.dati, Box::new(Vec::<u8>::new()));
    let righe = mzxml_indice::righe(&mut dati, dim, limiti)?;
    if righe.is_empty() {
        return Err(Error::new("err.file.xml").with("detail", "no scans"));
    }
    let mut l = LettoreMzxml {
        dati,
        dim,
        nome,
        limiti: *limiti,
        campioni: Vec::new(),
        righe,
        indice: Vec::new(),
        tic_noto: Vec::new(),
        bpi: Vec::new(),
        cabina: Cabina::default(),
        sha1: None,
        ultima: RefCell::new(None),
    };
    l.leggi_testa()?;
    l.costruisci_indice()?;
    let stem = l
        .nome
        .rsplit_once('.')
        .map_or(l.nome.as_str(), |(s, _)| s)
        .to_string();
    l.campioni = vec![Campione {
        id: 1,
        nome: stem,
        scansioni: l.indice.len(),
    }];
    Ok(Box::new(l))
}
