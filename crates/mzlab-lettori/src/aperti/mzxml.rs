//! An mzXML file (ISB, versions 2.x and 3.x) as a `Lettore`: spectra with level, precursor, polarity and retention time, TIC and BPC chromatograms computed from the scans, the header of the file
//! as `metodo/1`. The index comes from `<index name="scan">` or from one pass over the bytes (`mzxml_indice.rs`); `scansione(c, k)` reads only the bytes of scan `k` (`mzxml_scan.rs`).
//! SRM/MRM scans become chromatograms through `mzlab_lettori::srm::SrmDaScansioni`, which wraps every reader of this crate. Written from the mzXML schema (ISB); not from any program's code.

use std::cell::RefCell;

use mzlab_core::error::{Error, Result};

use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{
    Campione, Campo, Fiducia, FileSet, Formato, Lettore, Limiti, VoceCromatogramma, VoceLettore,
    VoceScansione, CONTRATTO,
};

use mzlab_core::metodo::sha1_hex;
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};

use mzlab_core::mzmlwrite::{Header, SpectrumExtra};

use super::mzxml_finestra::{peak_sums, Window};
use super::mzxml_indice::{self, Row};
use super::mzxml_precursori::precursori_di;
use super::mzxml_voce::entry;

use super::mzxml_scan::{self as scan, attr, each, Tag, Testa};

use super::mzxml_testata::{self as testata, Cabina};

/// The bytes of the start of the file that `riconosci` looks at.
const TESTA: usize = 4096;
/// The most bytes of the start of a scan read to learn its attributes (the peaks come after them), and of the header of the file.
const TESTA_SCANSIONE: u64 = 1 << 20;
const UNITA: &str = "number of detector counts";
/// The most `nameValue`/`precursorMz` elements kept for a scan.
pub(super) const CAP: usize = 256;

pub const ENTRY: VoceLettore = VoceLettore {
    id: "mzxml",
    contratto: CONTRATTO,
    estensioni: &[".mzxml"],
    riconosci,
    apri,
};

impl MzxmlReader {
    fn one(&self, c: usize) -> Result<()> {
        (c == 0)
            .then_some(())
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    fn read(&mut self, da: u64, n: u64) -> Result<Vec<u8>> {
        let mut b = Vec::new();
        b.try_reserve_exact(usize::try_from(n).map_err(|_| Error::memory_budget())?)
            .map_err(|_| Error::memory_budget())?;
        b.resize(n as usize, 0);
        self.data.read_at(da, &mut b)?;
        Ok(b)
    }

    /// The bytes of scan `k`; an index entry that does not point at a `<scan` is an error with a key.
    fn byte(&mut self, k: usize) -> Result<Vec<u8>> {
        let r = *self
            .rows
            .get(k)
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        if r.end - r.start > self.limits.byte {
            return Err(Error::memory_budget());
        }
        let b = self.read(r.start, r.end - r.start)?;
        if !b.starts_with(b"<scan") {
            return Err(Error::new("err.file.xml").with("detail", "index offset is not a scan"));
        }
        Ok(b)
    }

    /// Scan `k`: its attributes and its peaks, and the extra of its mzML spectrum.
    fn spettro(&mut self, k: usize) -> Result<(Spectrum, SpectrumExtra)> {
        let byte = self.byte(k)?;
        let text = String::from_utf8_lossy(&byte);
        let t = scan::testa(&text, CAP)?;
        let conteggio = number(t.scan, "peaksCount").map(|n| n.max(0.0) as usize);
        let p = match t.peaks {
            Some((g, corpo)) => scan::peaks(g, corpo, conteggio, &self.limits)?,
            None => scan::Peaks {
                mz: Vec::new(),
                intensities: Vec::new(),
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
            intensity: p.intensities.iter().map(|&x| x as f32).collect(),
            wide: p.a_64_bit.then_some(p.intensities),
            mz: p.mz,
        };
        Ok((s, extra_di(&t, &self.indice)))
    }
}

impl MzxmlReader {
    /// Fills in the TIC (`tic`) or the base peak intensity of the scans whose attributes do not say it, from their peaks; a scan parsed for one
    /// also gets the other, so the second pass finds nothing left to parse.
    fn complete(&mut self, tic: bool) -> Result<()> {
        for k in 0..self.indice.len() {
            let (missing_tic, missing_bpi) = (!self.known_tic[k], self.bpi[k].is_none());
            if !(if tic { missing_tic } else { missing_bpi }) {
                continue;
            }
            let (tic_k, bpi_k) = self.sums(k)?;
            if missing_tic {
                self.indice[k].tic = tic_k;
                self.known_tic[k] = true;
            }
            if missing_bpi {
                self.bpi[k] = Some(bpi_k);
            }
        }
        Ok(())
    }

    /// TIC and base peak intensity of scan `k` from its peaks, without building the spectrum.
    fn sums(&mut self, k: usize) -> Result<(f64, f64)> {
        let byte = self.byte(k)?;
        peak_sums(&byte, &self.limits)
    }

    /// The scans a chromatogram sums: the MS1 ones, or all of them when the file has no MS1.
    fn choices(&self) -> Vec<usize> {
        let one: Vec<usize> = (0..self.indice.len())
            .filter(|&k| self.indice[k].livello == 1)
            .collect();
        if one.is_empty() {
            (0..self.indice.len()).collect()
        } else {
            one
        }
    }

    fn trace(&mut self, id: &str) -> Result<Chromatogram> {
        let (tic, kind, index) = match id {
            "TIC" => (true, ChromKind::Tic, 0),
            "BPC" => (false, ChromKind::Bpc, 1),
            _ => return Err(Error::new("error.badIndex").with("id", id)),
        };
        self.complete(tic)?;
        let k = self.choices();
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
            self.sha1 = Some(sha1_hex(&mut self.data, dim).ok());
        }
        self.sha1.clone().flatten()
    }
}

impl Lettore for MzxmlReader {
    fn format(&self) -> Formato {
        Formato::Altro("mzxml")
    }

    fn not_carried(&self) -> &'static [(&'static str, &'static str)] {
        use mzlab_core::conv::FUORI_MODELLO;
        &[(
            "text of the header elements, scanOrigin, maldi, separation, spotting and processingOperation",
            FUORI_MODELLO,
        )]
    }

    fn samples(&self) -> &[Campione] {
        &self.samples
    }

    fn missing(&mut self, c: usize) -> usize {
        if c != 0 {
            return 0;
        }
        self.dichiarati
            .saturating_sub(self.indice.len())
            .max(self.discarded)
    }

    fn index(&mut self, c: usize) -> Result<&[VoceScansione]> {
        self.one(c)?;
        self.complete(true)?;
        Ok(&self.indice)
    }

    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.one(c)?;
        let (s, e) = self.spettro(k)?;
        *self.last.borrow_mut() = Some((k, e));
        Ok(s)
    }

    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.one(c)?;
        let byte = self.byte(k)?;
        let text = String::from_utf8_lossy(&byte);
        let t = scan::testa(&text, CAP)?;
        let mut out = Vec::new();
        let add = |out: &mut Vec<Campo>, pre: &str, g: Tag| {
            each(g, &mut |n, v| out.push(field(&format!("{pre}{n}"), v)))
        };
        add(&mut out, "", t.scan);
        for (g, corpo) in &t.precursori {
            add(&mut out, "precursorMz.", g);
            out.push(field("precursorMz", corpo));
        }
        if let Some((g, _)) = t.peaks {
            add(&mut out, "peaks.", g);
        }
        for g in &t.values {
            out.push(field(attr(g, "name"), attr(g, "value")));
        }
        Ok(out)
    }

    fn chromatograms(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        self.one(c)?;
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

    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.one(c)?;
        self.trace(id)
    }

    fn method(&mut self, c: usize) -> Result<String> {
        self.one(c)?;
        let sha1 = self.sha1();
        Ok(testata::document(&self.cabina, &self.name, self.dim, sha1))
    }

    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> {
        self.one(c)?;
        let sha1 = self.sha1();
        let doc = testata::document(&self.cabina, &self.name, self.dim, sha1.clone());
        Ok(Some(testata::header(&self.cabina, &self.name, sha1, doc)))
    }

    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        match self.last.borrow().as_ref() {
            Some((k, e)) if *k == s.index => e.clone(),
            _ => SpectrumExtra::default(),
        }
    }
}

impl MzxmlReader {
    /// The header of the file (before the first scan).
    fn read_head(&mut self) -> Result<()> {
        let fine = self.rows[0].start.min(TESTA_SCANSIONE);
        let b = self.read(0, fine)?;
        let text = String::from_utf8_lossy(&b);
        self.cabina = testata::read(&text, &self.limits)?;
        scan::tags(&text, &mut |g, _| {
            if scan::name(g) == "msRun" {
                let n = number(g, "scanCount").map_or(0, |n| n.max(0.0) as usize);
                self.dichiarati = self.dichiarati.max(n);
            }
            true
        });
        Ok(())
    }

    /// The index entries, from the attributes at the start of each scan.
    fn build_index(&mut self) -> Result<()> {
        let mut fin = Window::default();
        for k in 0..self.rows.len() {
            let r = self.rows[k];
            let mut n = 4096u64;
            let (known, bpi, v) = loop {
                let b = fin.read(
                    self.data.as_mut(),
                    self.dim,
                    r.start,
                    (r.end - r.start).min(n),
                )?;
                if b.windows(6).any(|w| w == b"<peaks")
                    || n >= r.end - r.start
                    || n >= TESTA_SCANSIONE
                {
                    let text = String::from_utf8_lossy(b);
                    let t = scan::testa(&text, CAP)?;
                    break (
                        number(t.scan, "totIonCurrent").is_some(),
                        number(t.scan, "basePeakIntensity"),
                        entry(k, r, &t)?,
                    );
                }
                n *= 4;
            };
            self.known_tic.push(known);
            self.bpi.push(bpi);
            self.indice.push(v);
        }
        Ok(())
    }
}

pub struct MzxmlReader {
    data: Box<dyn ReadAt>,
    dim: u64,
    name: String,
    limits: Limiti,
    samples: Vec<Campione>,
    rows: Vec<Row>,
    indice: Vec<VoceScansione>,
    /// Whether the file says the TIC / the base peak intensity of each scan (`false`: computed from the peaks when asked).
    known_tic: Vec<bool>,
    bpi: Vec<Option<f64>>,
    /// The scans the file says it has (`scanCount`, the `<offset>`s of its index) and those the pass left out as damaged, to count those a damaged copy lacks.
    dichiarati: usize,
    discarded: usize,
    cabina: Cabina,
    sha1: Option<Option<String>>,
    /// The extra of the spectrum `scansione` read last (the writer asks for it right after).
    last: RefCell<Option<(usize, SpectrumExtra)>>,
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

pub(super) fn number(t: Tag, n: &str) -> Option<f64> {
    attr(t, n)
        .trim()
        .parse()
        .ok()
        .filter(|x: &f64| x.is_finite())
}

/// The `userParam`s, the window and the precursors of a scan for the mzML we write.
fn extra_di(t: &Testa, index: &[VoceScansione]) -> SpectrumExtra {
    let mut e = SpectrumExtra {
        sim: attr(t.scan, "scanType").eq_ignore_ascii_case("SIM"),
        observed: number(t.scan, "lowMz").is_some() && number(t.scan, "highMz").is_some(),
        window: number(t.scan, "startMz").zip(number(t.scan, "endMz")),
        precursors: precursori_di(t, index),
        ..SpectrumExtra::default()
    };
    let mut add = |g: Tag, names: &[&str]| {
        for n in names {
            if !attr(g, n).is_empty() {
                e.params
                    .push((format!("mzXML {n}"), attr(g, n).to_string()));
            }
        }
    };
    add(t.scan, &["collisionEnergy", "scanType", "filterLine"]);
    for p in &t.precursori {
        add(
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

fn field(name: &str, value: &str) -> Campo {
    Campo {
        nome: name.into(),
        valore: value.into(),
        unita: None,
        cv: None,
    }
}

fn apri(mut fs: FileSet, limits: &Limiti) -> Result<Box<dyn Lettore>> {
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
    let mzxml_indice::Passata {
        rows: righe,
        discarded,
        indicizzate,
    } = mzxml_indice::rows(&mut dati, dim, limits)?;
    if righe.is_empty() {
        return Err(Error::new("err.file.xml").with("detail", "no scans"));
    }
    let mut l = MzxmlReader {
        data: dati,
        dim,
        name: nome,
        limits: *limits,
        samples: Vec::new(),
        rows: righe,
        indice: Vec::new(),
        known_tic: Vec::new(),
        bpi: Vec::new(),
        dichiarati: indicizzate,
        discarded,
        cabina: Cabina::default(),
        sha1: None,
        last: RefCell::new(None),
    };
    l.read_head()?;
    l.build_index()?;
    let stem = l
        .name
        .rsplit_once('.')
        .map_or(l.name.as_str(), |(s, _)| s)
        .to_string();
    l.samples = vec![Campione {
        id: 1,
        nome: stem,
        scansioni: l.indice.len(),
        dimensione: None,
    }];
    Ok(Box::new(l))
}
