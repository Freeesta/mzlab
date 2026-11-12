//! The SCIEX `.wiff` + `.wiff.scan` pair as a `Lettore` (`readers.rs`): the container (`.wiff`) is read whole when the pair is opened, bounded by `Limiti`;
//! the `.wiff.scan` never, a scan reads only its own block. One `Campione` per injection; the index lists the scans (the MRM blocks are chromatograms).

use super::reader_extra::extra_of;
use super::window::Window;
use super::{container_within, samples, scan, Record, Wiff, SAMPLES};
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::method::{sha1_hex, Source, SrcFile};
use crate::mzml::{ChromKind, Chromatogram, Spectrum};
use crate::mzmlwrite::Header;
use crate::ole::Ole;
use crate::readers::{
    ChromatogramEntry, Fiducia, Field, FileSet, Format, Limits, Member, Position, Reader,
    ReaderEntry, Sample, ScanEntry, CONTRATTO,
};

const SCAN: &str = ".wiff.scan";
const WIFF: &str = ".wiff";
const OLE_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

fn wiff_member(fs: &FileSet) -> Option<usize> {
    fs.find(WIFF)
}

fn riconosci(fs: &mut FileSet) -> Fiducia {
    let Some(w) = wiff_member(fs) else {
        return Fiducia::No;
    };
    if fs.testa(w, 8) != OLE_MAGIC {
        return Fiducia::No;
    }
    if fs.find(SCAN).is_some() {
        Fiducia::Si
    } else {
        Fiducia::Forse
    }
}

/// One injection: the reader of its blocks, which blocks are scans, and the index built from the heads of the blocks.
pub(super) struct Part {
    pub(super) w: Wiff,
    /// Indexes into `w.records` of the scans with points.
    pub(super) scans: Vec<usize>,
    /// The point count of each record's block as `build` read it (`scan::read_block`; 0 for a damaged block): the chromatograms do not read the blocks again.
    pub(super) counts: Vec<Option<u32>>,
    /// Damaged scan blocks (`scan::Block::Damaged`): left out of the index, counted by `losses`.
    pub(super) damaged: usize,
    indice: Vec<ScanEntry>,
    chroms: Option<Vec<Chromatogram>>,
    /// Base peak intensity of each scan as the mzML writer saw it (NaN: not yet) and how many it has seen: the BPC of the full scan experiments is listed
    /// once all of them were seen, so that listing the chromatograms never decodes a scan.
    pub(super) bpc: Vec<f64>,
    pub(super) bpc_seen: usize,
    /// A scan with points has a block version the engine's reader does not decode (it would list the sample with no scans), or an MRM block is sparse
    /// (the engine reads MRM dense only): the module reads it.
    #[cfg(feature = "sciex")]
    other_version: bool,
    /// `scans` and `indice` are built (`apri_solo` builds only the injection it is asked for; the others on their first use).
    ready: bool,
}

impl Part {
    fn new(w: Wiff) -> Self {
        Part {
            w,
            scans: Vec::new(),
            counts: Vec::new(),
            damaged: 0,
            indice: Vec::new(),
            chroms: None,
            bpc: Vec::new(),
            bpc_seen: 0,
            #[cfg(feature = "sciex")]
            other_version: false,
            ready: false,
        }
    }
}

pub struct WiffReader {
    ole: Ole,
    nome: String,
    scan: Box<dyn ReadAt>,
    pub(super) scan_len: u64,
    scan_member: u16,
    campioni: Vec<Sample>,
    /// Sections of the `.wiff.scan` the `.wiff` index does not list: (recovered as samples, left out).
    unindexed: (usize, usize),
    pub(super) parts: Vec<Part>,
    /// SHA-1 of the `.wiff.scan` for the header, read once (the whole member).
    scan_sha1: Option<Option<String>>,
}

pub const ENTRY: ReaderEntry = ReaderEntry {
    id: "sciex-wiff",
    contratto: CONTRATTO,
    estensioni: &[".wiff", ".wiff.scan"],
    riconosci,
    apri,
};

/// Injection `c` is of a variant: its method (any time) or a block of another version (once its index is built; `apri_solo` leaves the other injections unbuilt,
/// and the probe of the page does not read their blocks: the method says it for a pair of one instrument).
#[cfg(feature = "sciex")]
fn variant_of(parts: &[Part], c: usize) -> bool {
    parts
        .get(c)
        .is_some_and(|p| p.w.variant() || (p.ready && p.other_version))
}

impl WiffReader {
    pub(super) fn part(&self, c: usize) -> Result<&Part> {
        self.parts
            .get(c)
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    /// Injection `c` with its index built (the heads of its blocks read now when `apri_solo` left it out).
    pub(super) fn ready(&mut self, c: usize) -> Result<&Part> {
        let p = self
            .parts
            .get_mut(c)
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))?;
        if !p.ready {
            p.w.readable()?;
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
            .parts
            .get_mut(c)
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))?;
        if p.chroms.is_none() {
            p.w.readable()?;
            let mut win = Window::new(&mut self.scan, self.scan_len);
            let counts = Some(&p.counts[..]).filter(|c| p.ready && c.len() == p.w.records.len());
            p.chroms = Some(p.w.chromatograms(&mut win, counts)?);
        }
        Ok(p.chroms.as_deref().unwrap_or_default())
    }

    /// The optical DAD / PDA 3D matrix and TWC of injection `c`, when the injection has a `DAD` storage.
    pub fn dad(&self, c: usize) -> Result<Option<&super::dad::DadData>> {
        Ok(self.part(c)?.w.dad.as_ref())
    }
}

/// The injection `c` and its record `k` (the `k`-th scan).
fn row(parts: &[Part], c: usize, k: usize) -> Result<(&Part, &Record)> {
    let p = parts
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
fn source(l: &WiffReader, sample: u32) -> Source {
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

#[rustfmt::skip] impl Reader for WiffReader { #[cfg(feature = "sciex")] fn variant(&mut self, c: usize) -> bool { variant_of(&self.parts, c) } fn extra_mzml(&self, s: &Spectrum) -> crate::mzmlwrite::SpectrumExtra { extra_of(self, s) } fn losses(&mut self, c: usize) -> Result<crate::readers::Losses> { self.losses_of(c) } fn scan_extra(&mut self, c: usize, k: usize, s: &Spectrum) -> crate::mzmlwrite::SpectrumExtra { self.watch(c, k, s); extra_of(self, s) } #[cfg(feature = "mappa-dad")] fn dad_propose(&mut self, c: usize, rt0: f64, rt1: f64) -> Result<Option<String>> { Ok(self.part(c)?.w.dad.as_ref().and_then(|d| super::dad_uv::dad_propose_json(d, rt0, rt1))) }
    fn format(&self) -> Format { Format::SciexWiff }
    fn not_carried(&self) -> &'static [(&'static str, &'static str)] {
        use crate::conv::{IDENTITA, NON_DECODIFICATO};
        &[("DDERealTimeData, TripleTOF and ZenoTOF blocks, MSConfigInfo, MassSpecMethod, ExperimentHeader, PeriodStream and CycleDepParam binaries", NON_DECODIFICATO), ("log state at start and end, CFR audit, acquisition time per sample", NON_DECODIFICATO), ("user, computer, site and paths", IDENTITA)]
    }
    fn missing(&mut self, c: usize) -> usize { let n = self.scan_len; self.parts.get(c).map_or(0, |p| p.w.records.iter().filter(|r| r.cut(n)).count()) }
    fn refused(&mut self, c: usize) -> Option<Error> { self.parts.get(c).and_then(|p| p.w.readable().err()) }
    fn samples(&self) -> &[Sample] { &self.campioni }
    fn unindexed(&self) -> (usize, usize) { self.unindexed }
    fn index(&mut self, c: usize) -> Result<&[ScanEntry]> { Ok(&self.ready(c)?.indice) }
    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.ready(c)?;
        let (p, r) = row(&self.parts, c, k)?;
        let scan::Block::Scan(pts) = scan::read_block(&mut self.scan, self.scan_len, r, None)? else {
            return Err(Error::new("err.file.wiff").with("detail", "not a scan"));
        };
        Ok(p.w.spectrum_of(k, r, pts))
    }
    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Field>> {
        self.ready(c)?;
        let (p, r) = row(&self.parts, c, k)?;
        Ok(vec![field("sample", p.w.sample), field("cycle", r.cycle + 1), field("experiment", r.experiment + 1)])
    }
    fn chromatograms(&mut self, c: usize) -> Result<Vec<ChromatogramEntry>> {
        let mut v: Vec<ChromatogramEntry> = self.chromatograms(c)?.iter().map(|h| ChromatogramEntry { id: h.id.clone(), tipo: h.kind, unita: h.unit.clone() }).collect();
        self.list_bpc(c, &mut v)?;
        if self.part(c)?.w.dad.is_some() && !v.iter().any(|x| x.tipo == ChromKind::Pda) {
            v.push(ChromatogramEntry { id: "DAD TWC".into(), tipo: ChromKind::Pda, unita: "mAU".into() });
        }
        Ok(v)
    }
    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        if let Some(ch) = self.chromatograms(c)?.iter().find(|h| h.id == id).cloned() {
            return Ok(ch);
        }
        if let Some(ch) = self.bpc_chroms(c)?.into_iter().find(|h| h.id == id) {
            return Ok(ch);
        }
        dad_chrom(self.part(c)?.w.dad.as_ref(), id).ok_or_else(|| Error::new("error.badIndex").with("id", id))
    }
    #[cfg(any(not(target_family = "wasm"), feature = "sciex"))]
    fn dad_data(&mut self, c: usize) -> Result<Option<super::dad::DadData>> {
        Ok(self.part(c)?.w.dad.clone())
    }
    #[cfg(feature = "mappa-dad")]
    fn dad_map(&mut self, c: usize) -> Result<Option<String>> {
        Ok(self.part(c)?.w.dad.as_ref().map(|d| d.to_map_json()))
    }
    #[cfg(feature = "mappa-dad")]
    fn dad_wavelength(&mut self, c: usize, nm0: f64, nm1: f64) -> Result<Option<String>> {
        Ok(self.part(c)?.w.dad.as_ref().and_then(|d| super::dad_uv::dad_wavelength_json(d, nm0, nm1)))
    }
    #[cfg(feature = "mappa-dad")]
    fn dad_spectrum(&mut self, c: usize, rt0: f64, rt1: f64) -> Result<Option<String>> {
        Ok(self.part(c)?.w.dad.as_ref().and_then(|d| super::dad_uv::dad_spectrum_json(d, rt0, rt1)))
    }
    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> { with_method(self, c).map(Some) }
    fn table_header(&mut self, c: usize) -> Result<Option<Header>> { file_header(self, c).map(Some) }
    fn method(&mut self, c: usize) -> Result<String> {
        let w = &self.part(c)?.w;
        let sample = w.sample;
        Ok(crate::method_wiff::from_wiff(&self.ole, w.inferred_from.unwrap_or(sample), &source(self, sample)))
    }
}

fn field(name: &str, value: impl ToString) -> Field {
    Field {
        nome: name.into(),
        valore: value.to_string(),
        unita: None,
        cv: None,
    }
}

/// The header of the mzML with the `metodo/1` document in the run: the method (pumps, gradient, detectors) travels in the mzML, as for `.raw` and `.D`,
/// so the page reads it back from a converted file.
fn with_method(l: &mut WiffReader, c: usize) -> Result<Header> {
    let mut h = file_header(l, c)?;
    let doc = l.method(c)?;
    if !doc.is_empty() {
        h.run_params
            .push((crate::method::USERPARAM.to_string(), doc));
    }
    Ok(h)
}

/// The header of the mzML of injection `c`, with the SHA-1 of the `.wiff.scan`.
fn file_header(l: &mut WiffReader, c: usize) -> Result<Header> {
    l.ready(c)?;
    if l.scan_sha1.is_none() {
        let win = &mut Window::new(&mut l.scan, l.scan_len);
        l.scan_sha1 = Some(sha1_hex(win, l.scan_len).ok());
    }
    let sha = l.scan_sha1.clone().flatten();
    let w = &l.part(c)?.w;
    #[cfg(feature = "estesi")]
    let mut h = w.header_estesi(&l.nome, sha);
    #[cfg(not(feature = "estesi"))]
    let mut h = w.header(&l.nome, sha, false);
    // a section the index does not list: the run says that its name, method and times were inferred from an indexed sample
    if let Some(d) = w.inferred_from {
        h.run_params.push((
            "mzlab:wiff-unindexed".into(),
            format!("name, method and times inferred from sample {d}"),
        ));
    }
    Ok(h)
}

fn member(fs: &mut FileSet, i: usize) -> Result<&mut Member> {
    fs.membri
        .get_mut(i)
        .ok_or_else(|| Error::new("err.file.wiff").with("detail", "member"))
}

fn apri(fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    open_only(fs, limits, None)
}

/// Opens the pair; with `solo = Some(c)` only injection `c` reads the heads of its blocks in the `.wiff.scan` (the index of one sample in its own worker: the other injections
/// of a container with many would read the whole `.wiff.scan` each), and the others are listed with 0 scans until they are used.
pub fn open_only(mut fs: FileSet, limits: &Limits, only: Option<usize>) -> Result<Box<dyn Reader>> {
    let wi = wiff_member(&fs).ok_or_else(|| Error::new("err.file.notWiff").with("name", "?"))?;
    let name = member(&mut fs, wi)?
        .nome
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("x.wiff")
        .to_string();
    let si = fs
        .find(SCAN)
        .ok_or_else(|| Error::new("err.file.wiffNoScan").with("name", &name))?;
    let m = member(&mut fs, wi)?;
    if m.dim > limits.byte {
        return Err(Error::memory_budget());
    }
    let mut bytes = vec![0u8; usize::try_from(m.dim).map_err(|_| Error::memory_budget())?];
    m.dati.read_at(0, &mut bytes)?;
    let ole = container_within(bytes, &name, limits)?;
    let sm = member(&mut fs, si)?;
    let scan_len = sm.dim;
    // the SHA-1 of the `.wiff.scan` when the fingerprints were already taken (`upload::impronte`, `impronte_note`): the header does not read the whole file again
    let scan_sha1 = sm.sha1.take().filter(|s| !s.is_empty()).map(Some);
    let mut scan = std::mem::replace(&mut sm.dati, Box::new(Vec::<u8>::new()));
    let membro = u16::try_from(si).unwrap_or(0);
    let (mut parti, mut campioni, mut total) = (Vec::new(), Vec::new(), 0u64);
    let mut raw_names = Vec::new();
    let sha1 = ole.sha1_hex();
    for (i, s) in samples(&ole).into_iter().enumerate() {
        let mut w = Wiff::open_or_refuse(&ole, &name, scan_len, s)?;
        w.wiff_sha1 = Some(sha1.clone());
        total += w.records.len() as u64;
        limits.ok_entries(total)?;
        let mut part = Part::new(w);
        // a sample the engine cannot read is left unbuilt: `pronta` refuses it when it is asked for, the others open
        if only.is_none_or(|c| c == i) && part.w.readable().is_ok() {
            build(&mut part, &mut scan, scan_len, membro)?;
        }
        raw_names.push((
            s,
            ole.read(&format!("{SAMPLES}Sample{s}/SampleDABE/DATA"))
                .and_then(|b| crate::method_wiff::counted_strings(&b).into_iter().next()),
        ));
        campioni.push(Sample {
            id: s,
            nome: String::new(),
            scansioni: part.scans.len(),
            size: None,
        });
        parti.push(part);
    }
    let unindexed = add_unindexed(
        &mut parti,
        &mut campioni,
        &mut raw_names,
        (&mut scan, scan_len, membro),
        (&ole, limits, only),
        &mut total,
    )?;
    Wiff::all_refused(parti.iter().map(|p| &p.w))?;
    for (c, n) in campioni
        .iter_mut()
        .zip(super::names::display_names(&name, &raw_names))
    {
        c.nome = n;
    }
    Ok(Box::new(WiffReader {
        ole,
        nome: name,
        scan,
        scan_len,
        scan_member: membro,
        campioni,
        unindexed,
        parts: parti,
        scan_sha1,
    }))
}

/// The sections of the `.wiff.scan` the `.wiff` index does not list (the copy was made while the acquisition ran), rebuilt as samples after the indexed ones
/// with an inferred name and method; `(recovered, left out)`.
fn add_unindexed(
    parti: &mut Vec<Part>,
    campioni: &mut Vec<Sample>,
    raw_names: &mut Vec<(u32, Option<String>)>,
    (scan, scan_len, membro): (&mut Box<dyn ReadAt>, u64, u16),
    (ole, limits, only): (&Ole, &Limits, Option<usize>),
    total: &mut u64,
) -> Result<(usize, usize)> {
    let known: Vec<&Wiff> = parti.iter().map(|p| &p.w).collect();
    let (end, last_id) = super::unindexed::index_end(ole, &known);
    let found = super::unindexed::recover(scan, scan_len, &known, end, last_id);
    let counts = (found.samples.len(), found.skipped);
    for w in found.samples {
        let (i, s) = (parti.len(), w.sample);
        *total += w.records.len() as u64;
        limits.ok_entries(*total)?;
        let mut part = Part::new(w);
        if only.is_none_or(|c| c == i) && part.w.readable().is_ok() {
            build(&mut part, scan, scan_len, membro)?;
        }
        raw_names.push((s, None));
        campioni.push(Sample {
            id: s,
            nome: String::new(),
            scansioni: part.scans.len(),
            size: None,
        });
        parti.push(part);
    }
    Ok(counts)
}

/// The index of an injection: one row per non-empty scan block whose bytes agree with its head (the blocks are read through a window, front to back).
fn build(p: &mut Part, scan: &mut Box<dyn ReadAt>, scan_len: u64, member: u16) -> Result<()> {
    let w = &p.w;
    // each block is read whole, its points checked against its bytes: the window stops where the last block's own length ends (the last block of the file
    // runs to its end, a window there would read 1 MB per injection)
    let stop = w.records.iter().map(|r| r.data_end()).max().unwrap_or(0);
    let mut win = Window::until(scan, scan_len, stop);
    let (mut scans, mut index, mut tally) = (Vec::new(), Vec::new(), scan::Tally::default());
    let mut counts = Vec::with_capacity(w.records.len());
    #[cfg(feature = "sciex")]
    let (mut other, mut mrm_seen) = (false, vec![false; w.experiments.len().max(1)]);
    for (i, r) in w.records.iter().enumerate() {
        // a block cut by the end of the file is counted by `missing`, never read short
        let points = if r.cut(scan_len) {
            None
        } else {
            let b = scan::read_block(&mut win, scan_len, r, Some(&mut tally))?;
            #[cfg(feature = "sciex")]
            {
                other |= matches!(&b, scan::Block::Scan(p) if p.version != 2 && !p.mz.is_empty());
            }
            b.count()
        };
        counts.push(points);
        // the first MRM block of each experiment: a sparse (scheduled) one starts with `-(N + 0.01)`, even when the method has no `sMRM` stream to say so
        #[cfg(feature = "sciex")]
        if points.is_none()
            && r.size >= 4
            && !w.scan_experiment(r.experiment)
            && let Some(seen) = mrm_seen.get_mut(r.experiment).filter(|s| !**s)
        {
            *seen = true;
            other |= scan::read_mrm(&mut win, scan_len, r.at, 1)?
                .is_some_and(|v| v.first().is_some_and(|x| *x < 0.0));
        }
        if !points.is_some_and(|c| c > 0) {
            continue;
        }
        let e = w.experiments.get(r.experiment).cloned().unwrap_or_default();
        index.push(ScanEntry {
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
            base_peak: None,
            filtro: String::new(),
            precursore: e.precursor,
            position: Some(Position {
                membro: member,
                offset: r.at,
                lunghezza: u32::try_from(r.end.saturating_sub(r.at)).unwrap_or(u32::MAX),
            }),
        });
        scans.push(i);
    }
    (p.scans, p.indice, p.counts, p.damaged, p.ready) =
        (scans, index, counts, tally.check()?, true);
    #[cfg(feature = "sciex")]
    {
        p.other_version = other;
    }
    Ok(())
}
