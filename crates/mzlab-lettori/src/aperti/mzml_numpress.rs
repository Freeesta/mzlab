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
/// The most cvParams of a spectrum kept as header fields.
const CAMPI_MAX: usize = 64;

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

fn field(nome: &str, valore: impl ToString, unita: Option<&str>, cv: Option<&str>) -> Campo {
    Campo {
        nome: nome.into(),
        valore: valore.to_string(),
        unita: unita.map(str::to_string),
        cv: cv.map(str::to_string),
    }
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

    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.uno(c)?;
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

    /// The `metodo/1` document the mzML carries (written by mzLab); an mzML of another program has none (`{}`: no section).
    fn metodo(&mut self, c: usize) -> Result<String> {
        self.uno(c)?;
        let head = self.testa_xml()?;
        Ok(metodo::from_userparam(&head).unwrap_or_else(|| "{}".to_string()))
    }

    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        self.uno(c)?;
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
        Ok(Some(h))
    }

    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        match self.ultima.borrow().as_ref() {
            Some((k, e)) if self.indice.get(*k).is_some_and(|v| v.id == s.id) => e.clone(),
            _ => SpectrumExtra::default(),
        }
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

// ---- a tiny scanner of tags: the header of an mzML is cut before `<run` (and so not a document), and the module may not take a new XML crate ----

struct Tag {
    nome: String,
    attr: Vec<(String, String)>,
    chiusura: bool,
    vuoto: bool,
}

impl Tag {
    fn get(&self, n: &str) -> &str {
        self.attr
            .iter()
            .find(|(k, _)| k == n)
            .map_or("", |(_, v)| v.as_str())
    }
}

fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&#9;", "\t")
        .replace("&#13;", "\r")
        .replace("&#10;", "\n")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// The tags of `s` in order (declarations, comments and text are skipped). A tag ends at the first `>` outside quotes.
fn tag_di(s: &str) -> Vec<Tag> {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while let Some(p) = s.get(i..).and_then(|r| r.find('<')) {
        let a = i + p + 1;
        let mut q = a;
        let mut citato = 0u8;
        while let Some(&c) = b.get(q) {
            match (citato, c) {
                (0, b'"') | (0, b'\'') => citato = c,
                (x, y) if x != 0 && x == y => citato = 0,
                (0, b'>') => break,
                _ => {}
            }
            q += 1;
        }
        let Some(corpo) = s.get(a..q) else { break };
        i = q + 1;
        if corpo.starts_with(['?', '!']) {
            continue;
        }
        let (chiusura, corpo) = match corpo.strip_prefix('/') {
            Some(r) => (true, r),
            None => (false, corpo),
        };
        let (vuoto, corpo) = match corpo.strip_suffix('/') {
            Some(r) => (true, r),
            None => (false, corpo),
        };
        let mut parti = corpo.splitn(2, |c: char| c.is_whitespace());
        let nome = parti.next().unwrap_or("").to_string();
        let mut attr = Vec::new();
        let mut resto = parti.next().unwrap_or("");
        while let Some(e) = resto.find('=') {
            let chiave = resto[..e].trim().to_string();
            let v = resto[e + 1..].trim_start();
            let Some(virgolette) = v.chars().next().filter(|c| *c == '"' || *c == '\'') else {
                break;
            };
            let Some(fine) = v[1..].find(virgolette) else {
                break;
            };
            attr.push((chiave, unescape(&v[1..1 + fine])));
            resto = &v[fine + 2..];
        }
        out.push(Tag {
            nome,
            attr,
            chiusura,
            vuoto,
        });
    }
    out
}

/// The cvParams of a spectrum header (everything before `<binaryDataArrayList`) and what the writer cannot say with its own fields.
struct Parametro {
    acc: String,
    nome: String,
    valore: String,
    unita: Option<String>,
}

/// The cvParams that `mzmlwrite` writes itself, from the fields of the `Spectrum`.
const SCRITTI: [&str; 20] = [
    "MS:1000511",
    "MS:1000579",
    "MS:1000580",
    "MS:1000130",
    "MS:1000129",
    "MS:1000128",
    "MS:1000127",
    "MS:1000504",
    "MS:1000505",
    "MS:1000285",
    "MS:1000582",
    "MS:1000528",
    "MS:1000527",
    "MS:1000512",
    "MS:1000016",
    "MS:1000795",
    "MS:1000501",
    "MS:1000500",
    "MS:1000827",
    "MS:1000744",
];

fn parametri(spettro: &[u8]) -> Vec<Parametro> {
    let t = String::from_utf8_lossy(spettro);
    let fine = t.find("<binaryDataArrayList").unwrap_or(t.len());
    tag_di(&t[..fine])
        .into_iter()
        .filter(|g| !g.chiusura && g.nome == "cvParam")
        .take(CAMPI_MAX)
        .map(|g| Parametro {
            acc: g.get("accession").to_string(),
            nome: g.get("name").to_string(),
            valore: g.get("value").to_string(),
            unita: Some(g.get("unitName").to_string()).filter(|u| !u.is_empty()),
        })
        .collect()
}

/// What goes into the converted spectrum besides the fields of `Spectrum`: the scan window and, as userParams, every other cvParam with a value (collision energy, isolation width, injection time…).
fn extra_di(spettro: &[u8]) -> SpectrumExtra {
    let mut e = SpectrumExtra::default();
    let (mut lo, mut hi) = (None, None);
    for p in parametri(spettro) {
        let numero = p.valore.parse::<f64>().ok();
        match p.acc.as_str() {
            "MS:1000501" => lo = numero,
            "MS:1000500" => hi = numero,
            a if SCRITTI.contains(&a) => {}
            _ if p.valore.is_empty() => {}
            a => e.params.push((format!("{a} {}", p.nome), p.valore)),
        }
    }
    e.window = lo.zip(hi);
    e
}

// ---- the header of the original mzML ----

/// The header of the mzML we write: the instrument, the sources with their SHA-1, the software and the run of the original, and mzLab as the program that wrote this file.
fn testata_da(head: &str, nome: &str, limiti: &Limiti) -> Result<Header> {
    let mut h = Header {
        run: nome.rsplit_once('.').map_or(nome, |(s, _)| s).to_string(),
        software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
        ..Header::default()
    };
    let mut pila: Vec<String> = Vec::new();
    let mut gruppi: Vec<(String, Vec<(String, String, String)>)> = Vec::new();
    let mut ic_visti = 0usize;
    let mut componente_con_cv = false;
    let mut soft: Option<(String, Option<(String, String)>)> = None;
    for g in tag_di(head) {
        if g.chiusura {
            if let Some(n) = pila.pop() {
                if n == "software" {
                    if let Some((v, Some((a, nm)))) = soft.take() {
                        h.extra_software.push((a, nm, v));
                    }
                    soft = None;
                }
            }
            continue;
        }
        let genitore = pila.last().cloned().unwrap_or_default();
        match g.nome.as_str() {
            "referenceableParamGroup" => gruppi.push((g.get("id").to_string(), Vec::new())),
            "instrumentConfiguration" => ic_visti += 1,
            "sourceFile" => h.sources.push(g.get("name").to_string()),
            "sample" if h.sample.is_none() => {
                h.sample = Some((g.get("id").to_string(), g.get("name").to_string()))
            }
            "software" => soft = Some((g.get("version").to_string(), None)),
            "run" => {
                h.run = g.get("id").to_string();
                h.acquired = Some(g.get("startTimeStamp").to_string()).filter(|t| !t.is_empty());
            }
            "source" | "analyzer" | "detector" if ic_visti == 1 => componente_con_cv = false,
            "referenceableParamGroupRef"
                if ic_visti == 1 && genitore == "instrumentConfiguration" =>
            {
                let r = g.get("ref");
                if let Some((_, cvs)) = gruppi.iter().find(|(id, _)| id == r) {
                    for (a, n, v) in cvs {
                        modello(&mut h, a, n, v);
                    }
                }
            }
            "userParam" if genitore == "run" => {
                h.run_params
                    .push((g.get("name").to_string(), g.get("value").to_string()));
            }
            "cvParam" => {
                let (a, n, v) = (g.get("accession"), g.get("name"), g.get("value"));
                match genitore.as_str() {
                    "fileContent" if a != "MS:1000579" => h.content.push((a.into(), n.into())),
                    "sourceFile" if a == "MS:1000569" => h.source_sha1.push(v.into()),
                    "sourceFile" => {
                        if h.sources.len() == 1 && !h.source_cv.iter().any(|(x, _)| x == a) {
                            h.source_cv.push((a.into(), n.into()));
                        }
                    }
                    "referenceableParamGroup" => {
                        if let Some((_, cvs)) = gruppi.last_mut() {
                            cvs.push((a.into(), n.into(), v.into()));
                        }
                    }
                    "instrumentConfiguration" if ic_visti == 1 => modello(&mut h, a, n, v),
                    "source" | "analyzer" | "detector" if ic_visti == 1 && !componente_con_cv => {
                        componente_con_cv = true;
                        let kind = genitore.clone();
                        h.components.push((kind, a.into(), n.into()));
                    }
                    "software" => {
                        if let Some((_, c @ None)) = soft.as_mut() {
                            *c = Some((a.into(), n.into()));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        if !g.vuoto {
            limiti.profondita_ok(pila.len() + 1)?;
            pila.push(g.nome);
        }
    }
    Ok(h)
}

/// A cvParam of the instrument configuration: the serial number, else the first term is the model.
fn modello(h: &mut Header, acc: &str, nome: &str, valore: &str) {
    if acc == "MS:1000529" {
        h.serial = Some(valore.to_string()).filter(|s| !s.is_empty());
    } else if h.instrument.is_none() {
        h.instrument = Some((acc.to_string(), nome.to_string()));
    }
}
