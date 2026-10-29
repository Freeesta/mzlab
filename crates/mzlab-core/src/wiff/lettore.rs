//! The SCIEX `.wiff` + `.wiff.scan` pair as a `Lettore` (`lettori.rs`): the container (`.wiff`) is read whole when the pair is opened, bounded by `Limiti`;
//! the `.wiff.scan` never, a scan reads only its own block. One `Campione` per injection; the index lists the scans (the MRM blocks are chromatograms).

use super::window::Window;
use super::{container, samples, scan, Record, Wiff, SAMPLES};
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::lettori::{
    Campione, Campo, Fiducia, FileSet, Formato, Lettore, Limiti, Membro, Posizione,
    VoceCromatogramma, VoceLettore, VoceScansione, CONTRATTO,
};
use crate::metodo::{sha1_hex, Source, SrcFile};
use crate::mzml::{ChromKind, Chromatogram, Spectrum};
use crate::mzmlwrite::Header;
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
pub(super) struct Parte {
    pub(super) w: Wiff,
    /// Indexes into `w.records` of the scans with points.
    pub(super) scans: Vec<usize>,
    indice: Vec<VoceScansione>,
    chroms: Option<Vec<Chromatogram>>,
    /// Base peak intensity of each scan as the mzML writer saw it (NaN: not yet) and how many it has seen: the BPC of the full scan experiments is listed
    /// once all of them were seen, so that listing the chromatograms never decodes a scan.
    pub(super) bpc: Vec<f64>,
    pub(super) bpc_seen: usize,
    /// A scan with points has a block version the engine's reader does not decode (it would list the sample with no scans): the module reads it.
    #[cfg(feature = "sciex")]
    other_version: bool,
    /// `scans` and `indice` are built (`apri_solo` builds only the injection it is asked for; the others on their first use).
    pronta: bool,
}

pub struct LettoreWiff {
    ole: Ole,
    nome: String,
    scan: Box<dyn ReadAt>,
    scan_len: u64,
    scan_member: u16,
    campioni: Vec<Campione>,
    pub(super) parti: Vec<Parte>,
    /// SHA-1 of the `.wiff.scan` for the header, read once (the whole member).
    scan_sha1: Option<Option<String>>,
}

pub const VOCE: VoceLettore = VoceLettore {
    id: "sciex-wiff",
    contratto: CONTRATTO,
    estensioni: &[".wiff", ".wiff.scan"],
    riconosci,
    apri,
};

impl LettoreWiff {
    pub(super) fn parte(&self, c: usize) -> Result<&Parte> {
        self.parti
            .get(c)
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    /// Injection `c` with its index built (the heads of its blocks read now when `apri_solo` left it out).
    pub(super) fn pronta(&mut self, c: usize) -> Result<&Parte> {
        let p = self
            .parti
            .get_mut(c)
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))?;
        if !p.pronta {
            build(p, &mut self.scan, self.scan_len, self.scan_member)?;
            // the list of samples told 0 scans for an injection `apri_solo` left out
            if let Some(k) = self.campioni.get_mut(c) {
                k.scansioni = p.scans.len();
            }
        }
        Ok(p)
    }

    /// The chromatograms of injection `c`, read once (the MRM blocks only).
    pub(super) fn chromatograms(&mut self, c: usize) -> Result<&[Chromatogram]> {
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

    /// The optical DAD / PDA 3D matrix and TWC of injection `c`, when the injection has a `DAD` storage.
    pub fn dad(&self, c: usize) -> Result<Option<&super::dad::DadData>> {
        Ok(self.parte(c)?.w.dad.as_ref())
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
    }
}

fn dad_chrom(dad: Option<&super::dad::DadData>, id: &str) -> Option<Chromatogram> {
    let dad = dad?;
    if id == "DAD TWC" || id.eq_ignore_ascii_case("pda") {
        return Some(dad.twc_chromatogram(0));
    }
    let nm: f64 = id
        .strip_prefix("DAD ")?
        .strip_suffix(" nm")?
        .trim()
        .parse()
        .ok()?;
    dad.wavelength_chromatogram(nm, None, 0)
}

#[rustfmt::skip] impl Lettore for LettoreWiff { #[cfg(feature = "sciex")] fn variante(&mut self, c: usize) -> bool { self.pronta(c).is_ok_and(|p| p.w.variante() || p.other_version) } fn extra_mzml(&self, s: &Spectrum) -> crate::mzmlwrite::SpectrumExtra { extra_of(self, s) } fn extra_scansione(&mut self, c: usize, k: usize, s: &Spectrum) -> crate::mzmlwrite::SpectrumExtra { #[cfg(feature = "estesi")] self.watch(c, k, s); extra_of(self, s) }
    fn formato(&self) -> Formato { Formato::SciexWiff }
    fn non_portati(&self) -> &'static [(&'static str, &'static str)] {
        use crate::conv::{IDENTITA, NON_DECODIFICATO};
        &[("device traces (Devices/Device_N/DevData), DDERealTimeData, TripleTOF and ZenoTOF blocks, MSConfigInfo, MassSpecMethod, ExperimentHeader, PeriodStream and CycleDepParam binaries", NON_DECODIFICATO), ("log state at start and end, CFR audit, acquisition time per sample", NON_DECODIFICATO), ("user, computer, site and paths", IDENTITA)]
    }
    fn mancanti(&mut self, c: usize) -> usize {
        let n = self.scan_len;
        self.parti.get(c).map_or(0, |p| p.w.records.iter().filter(|r| r.end > n || r.at >= n).count())
    }
    fn campioni(&self) -> &[Campione] { &self.campioni }
    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> { Ok(&self.pronta(c)?.indice) }
    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.pronta(c)?;
        let (p, r) = row(&self.parti, c, k)?;
        let pts = scan::read_scan(&mut self.scan, self.scan_len, r.at, r.end)?
            .ok_or_else(|| Error::new("err.file.wiff").with("detail", "not a scan"))?;
        Ok(p.w.spectrum_of(k, r, pts))
    }
    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.pronta(c)?;
        let (p, r) = row(&self.parti, c, k)?;
        Ok(vec![field("sample", p.w.sample), field("cycle", r.cycle + 1), field("experiment", r.experiment + 1)])
    }
    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        let mut v: Vec<VoceCromatogramma> = self.chromatograms(c)?.iter().map(|h| VoceCromatogramma { id: h.id.clone(), tipo: h.kind, unita: h.unit.clone() }).collect();
        #[cfg(feature = "estesi")]
        self.list_bpc(c, &mut v)?;
        if self.parte(c)?.w.dad.is_some() && !v.iter().any(|x| x.tipo == ChromKind::Pda) {
            v.push(VoceCromatogramma { id: "DAD TWC".into(), tipo: ChromKind::Pda, unita: "mAU".into() });
        }
        Ok(v)
    }
    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        if let Some(ch) = self.chromatograms(c)?.iter().find(|h| h.id == id).cloned() {
            return Ok(ch);
        }
        #[cfg(feature = "estesi")]
        if let Some(ch) = self.bpc_chroms(c)?.into_iter().find(|h| h.id == id) {
            return Ok(ch);
        }
        dad_chrom(self.parte(c)?.w.dad.as_ref(), id).ok_or_else(|| Error::new("error.badIndex").with("id", id))
    }
    #[cfg(not(target_family = "wasm"))]
    fn dad_dati(&mut self, c: usize) -> Result<Option<super::dad::DadData>> {
        Ok(self.parte(c)?.w.dad.clone())
    }
    #[cfg(feature = "mappa-dad")]
    fn dad_map(&mut self, c: usize) -> Result<Option<String>> {
        Ok(self.parte(c)?.w.dad.as_ref().map(|d| d.to_map_json()))
    }
    #[cfg(feature = "mappa-dad")]
    fn dad_wavelength(&mut self, c: usize, nm0: f64, nm1: f64) -> Result<Option<String>> {
        Ok(self.parte(c)?.w.dad.as_ref().and_then(|d| super::dad_uv::dad_wavelength_json(d, nm0, nm1)))
    }
    #[cfg(feature = "mappa-dad")]
    fn dad_spectrum(&mut self, c: usize, rt0: f64, rt1: f64) -> Result<Option<String>> {
        Ok(self.parte(c)?.w.dad.as_ref().and_then(|d| super::dad_uv::dad_spectrum_json(d, rt0, rt1)))
    }
    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> { testata(self, c).map(Some) }
    fn metodo(&mut self, c: usize) -> Result<String> {
        let sample = self.parte(c)?.w.sample;
        Ok(crate::metodo_wiff::from_wiff(&self.ole, sample, &source(self, sample)))
    }
}

fn field(nome: &str, valore: impl ToString) -> Campo {
    Campo {
        nome: nome.into(),
        valore: valore.to_string(),
        unita: None,
        cv: None,
    }
}

/// The header of the mzML of injection `c`, with the SHA-1 of the `.wiff.scan`.
fn testata(l: &mut LettoreWiff, c: usize) -> Result<Header> {
    l.pronta(c)?;
    if l.scan_sha1.is_none() {
        let win = &mut Window::new(&mut l.scan, l.scan_len);
        l.scan_sha1 = Some(sha1_hex(win, l.scan_len).ok());
    }
    let sha = l.scan_sha1.clone().flatten();
    #[cfg(feature = "estesi")]
    return Ok(l.parte(c)?.w.header_estesi(&l.nome, sha));
    #[cfg(not(feature = "estesi"))]
    Ok(l.parte(c)?.w.header(&l.nome, sha, false))
}

fn member(fs: &mut FileSet, i: usize) -> Result<&mut Membro> {
    fs.membri
        .get_mut(i)
        .ok_or_else(|| Error::new("err.file.wiff").with("detail", "member"))
}

fn apri(fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    apri_solo(fs, limiti, None)
}

/// Opens the pair; with `solo = Some(c)` only injection `c` reads the heads of its blocks in the `.wiff.scan` (the index of one sample in its own worker: the other injections
/// of a container with many would read the whole `.wiff.scan` each), and the others are listed with 0 scans until they are used.
pub fn apri_solo(
    mut fs: FileSet,
    limiti: &Limiti,
    solo: Option<usize>,
) -> Result<Box<dyn Lettore>> {
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
    let sha1 = ole.sha1_hex();
    for (i, s) in samples(&ole).into_iter().enumerate() {
        let mut w = Wiff::open(&ole, &nome, Some(scan_len), s)?;
        w.wiff_sha1 = Some(sha1.clone());
        total += w.records.len() as u64;
        limiti.voci_ok(total)?;
        let mut parte = Parte {
            w,
            scans: Vec::new(),
            indice: Vec::new(),
            chroms: None,
            bpc: Vec::new(),
            bpc_seen: 0,
            #[cfg(feature = "sciex")]
            other_version: false,
            pronta: false,
        };
        if solo.is_none_or(|c| c == i) {
            build(&mut parte, &mut scan, scan_len, membro)?;
        }
        let nome_campione = ole
            .read(&format!("{SAMPLES}Sample{s}/SampleDABE/DATA"))
            .and_then(|b| {
                crate::metodo_wiff::counted_strings(&b)
                    .into_iter()
                    .next()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| format!("{nome} #{s}"));
        campioni.push(Campione {
            id: s,
            nome: nome_campione,
            scansioni: parte.scans.len(),
            dimensione: None,
        });
        parti.push(parte);
    }
    Ok(Box::new(LettoreWiff {
        ole,
        nome,
        scan,
        scan_len,
        scan_member: membro,
        campioni,
        parti,
        scan_sha1: None,
    }))
}

/// The index of an injection: one row per non-empty scan block (the heads of the blocks are read through a window, front to back).
fn build(p: &mut Parte, scan: &mut Box<dyn ReadAt>, scan_len: u64, member: u16) -> Result<()> {
    let w = &p.w;
    let mut win = Window::new(scan, scan_len);
    let (mut scans, mut indice) = (Vec::new(), Vec::new());
    #[cfg_attr(not(feature = "sciex"), allow(unused_mut, unused_variables))]
    let mut other = false;
    for (i, r) in w.records.iter().enumerate() {
        if !scan::point_count(&mut win, scan_len, r.at, r.end)?.is_some_and(|c| c > 0) {
            continue;
        }
        #[cfg(feature = "sciex")]
        {
            other |= scan::other_version(&mut win, scan_len, r.at, r.end)?;
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
            polarita: w.polarity_of(&e),
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
    (p.scans, p.indice, p.pronta) = (scans, indice, true);
    #[cfg(feature = "sciex")]
    {
        p.other_version = other;
    }
    Ok(())
}

/// The number after `key` in a spectrum id (`sample=2 ... experiment=1`).
fn id_field(id: &str, key: &str) -> Option<usize> {
    id.split_whitespace()
        .find_map(|w| w.strip_prefix(key))
        .and_then(|v| v.parse::<usize>().ok())
}

/// The TIC the index of the file holds for the block of `s`, when the block is where the spectrum number says and the value is usable.
#[cfg(feature = "estesi")]
fn header_tic(l: &LettoreWiff, s: &Spectrum) -> Option<f64> {
    let p = l
        .parti
        .iter()
        .find(|p| Some(p.w.sample as usize) == id_field(&s.id, "sample="))?;
    let r = p.w.records.get(*p.scans.get(s.index)?)?;
    let same = id_field(&s.id, "cycle=") == Some(r.cycle + 1)
        && id_field(&s.id, "experiment=") == Some(r.experiment + 1);
    (same && r.tic.is_finite() && r.tic > 0.0).then_some(r.tic)
}

/// What the mzML says of a spectrum besides its points: the collision energy and isolation window of its experiment, and (readers module) the spectrum
/// type (CV term) of precursor-ion and neutral-loss scans.
fn extra_of(l: &LettoreWiff, s: &Spectrum) -> crate::mzmlwrite::SpectrumExtra {
    let e = id_field(&s.id, "sample=")
        .and_then(|n| l.parti.iter().find(|p| p.w.sample as usize == n))
        .zip(id_field(&s.id, "experiment="))
        .and_then(|(p, e)| p.w.experiments.get(e.wrapping_sub(1)));
    let mut extra = crate::mzmlwrite::SpectrumExtra::default();
    #[cfg(feature = "estesi")]
    {
        extra.tic = header_tic(l, s);
    }
    let Some(e) = e else { return extra };
    (extra.ce, extra.iso_half) = (e.ce, e.iso_half);
    #[cfg(feature = "estesi")]
    if let Some((acc, name)) = super::estesi::spectrum_term(e.code) {
        extra.params.push((format!("cv:{acc}"), name.to_string()));
    }
    extra
}
