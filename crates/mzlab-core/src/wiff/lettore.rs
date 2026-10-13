//! The SCIEX `.wiff` + `.wiff.scan` pair as a `Lettore` (`lettori.rs`): the container (`.wiff`) is read whole when the pair is opened, bounded by `Limiti`;
//! the `.wiff.scan` never, a scan reads only its own block. One `Campione` per injection; the index lists the scans (the MRM blocks are chromatograms).

use super::window::Window;
use super::{container, samples, scan, Record, Wiff};
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::lettori::{
    Campione, Campo, Fiducia, FileSet, Formato, Lettore, Limiti, Membro, Posizione,
    VoceCromatogramma, VoceLettore, VoceScansione, CONTRATTO,
};
use crate::metodo::{Source, SrcFile};
use crate::mzml::{Chromatogram, Spectrum};
use crate::ole::Ole;

const SCAN: &str = ".wiff.scan";
const WIFF: &str = ".wiff";
const OLE_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

fn wiff_member(fs: &FileSet) -> Option<usize> {
    fs.trova(WIFF)
}

fn riconosci(fs: &mut FileSet) -> Fiducia {
    let Some(w) = wiff_member(fs) else {
        return Fiducia::No;
    };
    if fs.testa(w, 8) != OLE_MAGIC {
        return Fiducia::No;
    }
    if fs.trova(SCAN).is_some() {
        Fiducia::Si
    } else {
        Fiducia::Forse
    }
}

/// One injection: the reader of its blocks, which blocks are scans, and the index built from the heads of the blocks.
struct Parte {
    w: Wiff,
    /// Indexes into `w.records` of the scans with points.
    scans: Vec<usize>,
    indice: Vec<VoceScansione>,
    chroms: Option<Vec<Chromatogram>>,
}

pub struct LettoreWiff {
    ole: Ole,
    nome: String,
    scan: Box<dyn ReadAt>,
    scan_len: u64,
    scan_member: u16,
    anonimo: bool,
    campioni: Vec<Campione>,
    parti: Vec<Parte>,
}

pub const VOCE: VoceLettore = VoceLettore {
    id: "sciex-wiff",
    contratto: CONTRATTO,
    estensioni: &[".wiff", ".wiff.scan"],
    riconosci,
    apri,
};

impl LettoreWiff {
    /// The same reader asked for an anonymous `metodo/1` (no people, computer, serial number or paths).
    pub fn anonimo(mut self, on: bool) -> Self {
        self.anonimo = on;
        self
    }

    fn parte(&self, c: usize) -> Result<&Parte> {
        self.parti
            .get(c)
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    /// The chromatograms of injection `c`, read once (the MRM blocks only).
    fn chromatograms(&mut self, c: usize) -> Result<&[Chromatogram]> {
        let p = self
            .parti
            .get_mut(c)
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))?;
        if p.chroms.is_none() {
            let mut win = Window::new(&mut self.scan, self.scan_len);
            p.chroms = Some(p.w.chromatograms(&mut win)?);
        }
        Ok(p.chroms.as_deref().unwrap_or_default())
    }

    /// The member of the `FileSet` that holds the `.wiff.scan` (what `Posizione::membro` says).
    pub fn membro_scan(&self) -> u16 {
        self.scan_member
    }
}

/// The injection `c` and its record `k` (the `k`-th scan).
fn row(parti: &[Parte], c: usize, k: usize) -> Result<(&Parte, &Record)> {
    let p = parti
        .get(c)
        .ok_or_else(|| Error::new("error.badIndex").with("c", c))?;
    let r = p
        .scans
        .get(k)
        .and_then(|&i| p.w.records.get(i))
        .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
    Ok((p, r))
}

/// `origine` of the document of injection `sample`.
fn source(l: &LettoreWiff, sample: u32) -> Source {
    let file = |name: String, sha1: Option<String>, bytes: u64| SrcFile { name, sha1, bytes };
    Source {
        format: "sciex-wiff",
        files: vec![
            file(l.nome.clone(), Some(l.ole.sha1_hex()), 0),
            file(format!("{}.scan", l.nome), None, l.scan_len),
        ],
        sample: Some(sample),
        anonymous: l.anonimo,
        secret: crate::pseudo::current(),
    }
}

impl Lettore for LettoreWiff {
    fn imposta_anonimo(&mut self, on: bool) {
        self.anonimo = on;
    }

    fn formato(&self) -> Formato {
        Formato::SciexWiff
    }

    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }

    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        Ok(&self.parte(c)?.indice)
    }

    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let (p, r) = row(&self.parti, c, k)?;
        let pts = scan::read_scan(&mut self.scan, self.scan_len, r.at, r.end)?
            .ok_or_else(|| Error::new("err.file.wiff").with("detail", "not a scan"))?;
        Ok(p.w.spectrum_of(k, r, pts))
    }

    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        let (p, r) = row(&self.parti, c, k)?;
        Ok(vec![
            field("sample", p.w.sample),
            field("cycle", r.cycle + 1),
            field("experiment", r.experiment + 1),
        ])
    }

    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        let all = self.chromatograms(c)?;
        Ok(all
            .iter()
            .map(|h| VoceCromatogramma {
                id: h.id.clone(),
                tipo: h.kind,
                unita: h.unit.clone(),
            })
            .collect())
    }

    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.chromatograms(c)?
            .iter()
            .find(|h| h.id == id)
            .cloned()
            .ok_or_else(|| Error::new("error.badIndex").with("id", id))
    }

    fn metodo(&mut self, c: usize) -> Result<String> {
        let sample = self.parte(c)?.w.sample;
        Ok(crate::metodo_wiff::from_wiff(
            &self.ole,
            sample,
            &source(self, sample),
        ))
    }
}

fn member(fs: &mut FileSet, i: usize) -> Result<&mut Membro> {
    fs.membri
        .get_mut(i)
        .ok_or_else(|| Error::new("err.file.wiff").with("detail", "member"))
}

fn apri(mut fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    let wi = wiff_member(&fs).ok_or_else(|| Error::new("err.file.notWiff").with("name", "?"))?;
    let nome = member(&mut fs, wi)?
        .nome
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("x.wiff")
        .to_string();
    let si = fs
        .trova(SCAN)
        .ok_or_else(|| Error::new("err.file.wiffNoScan").with("name", &nome))?;
    let m = member(&mut fs, wi)?;
    if m.dim > limiti.byte {
        return Err(Error::memory_budget());
    }
    let mut bytes = vec![0u8; usize::try_from(m.dim).map_err(|_| Error::memory_budget())?];
    m.dati.read_at(0, &mut bytes)?;
    let ole = container(bytes, &nome)?;
    let sm = member(&mut fs, si)?;
    let scan_len = sm.dim;
    let mut scan = std::mem::replace(&mut sm.dati, Box::new(Vec::<u8>::new()));
    let membro = u16::try_from(si).unwrap_or(0);
    let (mut parti, mut campioni, mut total) = (Vec::new(), Vec::new(), 0u64);
    for s in samples(&ole) {
        let w = Wiff::open(&ole, &nome, Some(scan_len), s)?;
        total += w.records.len() as u64;
        limiti.voci_ok(total)?;
        let parte = build(w, &mut scan, scan_len, membro)?;
        campioni.push(Campione {
            id: s,
            nome: format!("{nome} #{s}"),
            scansioni: parte.scans.len(),
        });
        parti.push(parte);
    }
    Ok(Box::new(LettoreWiff {
        ole,
        nome,
        scan,
        scan_len,
        scan_member: membro,
        anonimo: false,
        campioni,
        parti,
    }))
}

/// The index of an injection: one row per non-empty scan block (the heads of the blocks are read through a window, front to back).
fn build(w: Wiff, scan: &mut Box<dyn ReadAt>, scan_len: u64, member: u16) -> Result<Parte> {
    let mut win = Window::new(scan, scan_len);
    let (mut scans, mut indice) = (Vec::new(), Vec::new());
    for (i, r) in w.records.iter().enumerate() {
        if !scan::point_count(&mut win, scan_len, r.at, r.end)?.is_some_and(|c| c > 0) {
            continue;
        }
        let e = w.experiments.get(r.experiment).cloned().unwrap_or_default();
        indice.push(VoceScansione {
            id: format!(
                "sample={} period=1 cycle={} experiment={}",
                w.sample,
                r.cycle + 1,
                r.experiment + 1
            ),
            rt: r.time_min,
            livello: e.level.max(1),
            polarita: w.polarity,
            tic: r.tic,
            picco_base: None,
            filtro: String::new(),
            precursore: e.precursor,
            posizione: Some(Posizione {
                membro: member,
                offset: r.at,
                lunghezza: u32::try_from(r.end.saturating_sub(r.at)).unwrap_or(u32::MAX),
            }),
        });
        scans.push(i);
    }
    Ok(Parte {
        w,
        scans,
        indice,
        chroms: None,
    })
}

fn field(nome: &str, valore: impl ToString) -> Campo {
    Campo {
        nome: nome.into(),
        valore: valore.to_string(),
        unita: None,
        cv: None,
    }
}
