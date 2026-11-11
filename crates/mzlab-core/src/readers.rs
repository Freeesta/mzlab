//! The contract of the readers of vendor files (`docs/agenti/estendere.md`): a `Lettore` serves the scans of a file by position, one at a time, so that the pages
//! read a `.raw`, a `.wiff` or a `.D` as they read an mzML, and the conversion to mzML is one export among the others (`scrivi_mzml`, the one writer).
//! A format is a `ReaderEntry` (how to recognise it, how to open it) in the static registry `LETTORI`; nothing is loaded at run time.
//! `Limiti` is passed to every `apri`: depth, entries and bytes that a damaged or hostile file may ask for are bounded by construction.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::mzml::{ChromKind, Chromatogram, Spectrum};
use crate::mzmlwrite::{Header, SpectrumExtra};

/// Version of this contract; a `ReaderEntry` states the one it was written for.
pub const CONTRATTO: u16 = 1;

/// The formats the readers know (the `id` is the one of the registry and of `metodo/1.origine.formato`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    ThermoRaw,
    SciexWiff,
    SciexDam,
    AgilentD,
    Mzml,
    Altro(&'static str),
}

impl Format {
    pub fn id(&self) -> &'static str {
        match self {
            Format::ThermoRaw => "thermo-raw",
            Format::SciexWiff => "sciex-wiff",
            Format::SciexDam => "sciex-dam",
            Format::AgilentD => "agilent-d",
            Format::Mzml => "mzml",
            Format::Altro(id) => id,
        }
    }
}

/// How sure a reader is that a set of files is its format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fiducia {
    No,
    Forse,
    Si,
}

/// What a damaged or hostile file may ask for. Every reader checks the numbers it reads from a file against these before it allocates or recurses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Nesting of elements, directories, chains.
    pub profondita: usize,
    /// Entries of any table (scans, directory entries, records).
    pub entries: usize,
    /// Bytes held in memory at once.
    pub byte: u64,
}

impl Limits {
    /// The browser: the floor of its budgets, 192 MB (`budget::WASM_BUDGET_MB`); a bigger budget changes the way (memory or slabs), not these limits.
    pub const fn sito() -> Limits {
        Limits {
            profondita: 256,
            entries: 5_000_000,
            byte: (crate::budget::WASM_BUDGET_MB as u64) << 20,
        }
    }
    /// The desktop app.
    pub const fn desktop() -> Limits {
        Limits {
            profondita: 256,
            entries: 50_000_000,
            byte: (crate::budget::NATIVE_BUDGET_MB as u64) << 20,
        }
    }
    /// Bytes the caller already holds in memory (a `.dam`, the container of a `.wiff`): only entries and depth are checked, both as on the site.
    pub const fn held() -> Limits {
        Limits {
            byte: u64::MAX,
            ..Limits::sito()
        }
    }
    /// `n` entries asked by a file: an error with a key when they are more than the limit (and so never an allocation).
    pub fn ok_entries(&self, n: u64) -> Result<usize> {
        usize::try_from(n)
            .ok()
            .filter(|&n| n <= self.entries)
            .ok_or_else(Error::memory_budget)
    }
    /// A nesting depth: an error with a key when it passes the limit.
    pub fn profondita_ok(&self, d: usize) -> Result<()> {
        if d <= self.profondita {
            Ok(())
        } else {
            Err(Error::new("err.dam.unreadable").with("message", "nested too deeply"))
        }
    }
}

impl Default for Limits {
    fn default() -> Limits {
        Limits::sito()
    }
}

/// One file of a set, with the bytes at any offset.
pub struct Member {
    pub nome: String,
    pub dim: u64,
    pub dati: Box<dyn ReadAt>,
    /// SHA-1 of the whole member when the caller already has it (taken during the copy of the file): the reader that writes the header of an mzML uses it instead of reading the file again.
    pub sha1: Option<String>,
}

/// The files that make one dataset: `X.wiff` + `X.wiff.scan`, the members of a `.raw` or `.D` folder, or a single file. The names are logical (relative to the dataset).
#[derive(Default)]
pub struct FileSet {
    pub membri: Vec<Member>,
}

impl FileSet {
    /// Index of the member whose name ends with `suffisso`, ignoring case.
    pub fn find(&self, suffisso: &str) -> Option<usize> {
        let s = suffisso.to_lowercase();
        self.membri
            .iter()
            .position(|m| m.nome.to_lowercase().ends_with(&s))
    }
    /// The first `n` bytes of member `i` (fewer when it is shorter): what `riconosci` looks at, never the extension alone.
    pub fn testa(&mut self, i: usize, n: usize) -> Vec<u8> {
        let Some(m) = self.membri.get_mut(i) else {
            return Vec::new();
        };
        let mut b = vec![0u8; n.min(usize::try_from(m.dim).unwrap_or(usize::MAX))];
        if m.dati.read_at(0, &mut b).is_err() {
            b.clear();
        }
        b
    }
}

/// A sample of a file (a `.wiff` has several; the other formats one).
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub id: u32,
    pub nome: String,
    pub scansioni: usize,
    pub size: Option<u64>,
}

/// Where the bytes of a scan lie in the original: member of the `FileSet`, offset, length (what the `.mzidx` of a vendor file will store).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub membro: u16,
    pub offset: u64,
    pub lunghezza: u32,
}

/// One row of the index of a sample: what is known of a scan before its peaks are read.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanEntry {
    pub id: String,
    /// Minutes.
    pub rt: f64,
    /// MS level, 0 when the file does not say it.
    pub livello: u8,
    /// +1, -1 or 0 (not said).
    pub polarita: i8,
    pub tic: f64,
    pub base_peak: Option<f64>,
    pub filtro: String,
    pub precursore: Option<f64>,
    pub position: Option<Position>,
}

/// A name, a value and, when the file or the CV says so, a unit and an accession (`MS:1000927`): what the header of a scan holds besides its peaks.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Field {
    pub nome: String,
    pub valore: String,
    pub unita: Option<String>,
    pub cv: Option<String>,
}

/// A chromatogram of the file without its points.
#[derive(Debug, Clone, PartialEq)]
pub struct ChromatogramEntry {
    pub id: String,
    pub tipo: ChromKind,
    pub unita: String,
}

pub use outcome::{Losses, ReadOutcome, ReadOutcomes, EVENTS_REFUSED, TRUNCATED};

/// A reader of one dataset. Random access: `indice` is read once, `scansione` reads only the bytes of one scan; streaming is `scansione(c, 0..n)`.
/// An error is a key (`err.file.*`), never a panic; what a file lacks is absent from the answer, not an error.
pub trait Reader {
    fn format(&self) -> Format;
    /// At least one.
    fn samples(&self) -> &[Sample];
    /// The scans of sample `c`, in the order of time.
    fn index(&mut self, c: usize) -> Result<&[ScanEntry]>;
    /// The rows of `indice` for a caller that does not read their `tic`: a reader that sums it from the points of spectra it must decode (Bruker MS2) may leave it 0.
    fn index_without_tic(&mut self, c: usize) -> Result<&[ScanEntry]> {
        self.index(c)
    }
    /// Scan `k` of the index of sample `c`.
    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum>;
    /// Scan `k` for a caller that holds the index already (the module of the readers, `transport.rs`): the points, level, polarity, profile, RT and precursor as `scansione` gives them; `id` and `filter` may be empty (they are the ones of the row of the index).
    fn scan_data(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.scan(c, k)
    }
    /// Scan `k` with the profile points of the file where the reader keeps them next to the vendor's centroids; `None` when it has no profile of its own.
    fn scan_profile(&mut self, _c: usize, _k: usize) -> Result<Option<Spectrum>> {
        Ok(None)
    }
    /// The header fields of scan `k` (collision energy, isolation, trailer): `nome`, `valore`, `unita`, `cv`.
    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Field>>;
    fn chromatograms(&mut self, c: usize) -> Result<Vec<ChromatogramEntry>>;
    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram>;
    /// Optical DAD / PDA 2D map JSON for sample `c`, if present.
    fn dad_map(&mut self, _c: usize) -> Result<Option<String>> {
        Ok(None)
    }
    /// UV spectrum JSON of the DAD / PDA of sample `c` between the times `rt0` and `rt1`, if it has the optical matrix.
    fn dad_spectrum(&mut self, _c: usize, _rt0: f64, _rt1: f64) -> Result<Option<String>> {
        Ok(None)
    }
    /// Trace of the DAD / PDA of sample `c` at the wavelengths `nm0`..`nm1` (JSON `{rt, y, nm0, nm1, n}`), if it has the optical matrix.
    fn dad_wavelength(&mut self, _c: usize, _nm0: f64, _nm1: f64) -> Result<Option<String>> {
        Ok(None)
    }
    /// The optical (DAD / PDA) matrix of sample `c` as the reader holds it, for `convert.mzml` with `dad`; `None` when the sample has none.
    #[cfg(any(not(target_family = "wasm"), feature = "sciex"))]
    fn dad_data(&mut self, _c: usize) -> Result<Option<crate::wiff::dad::DadData>> {
        Ok(None)
    }
    /// The `metodo/1` document of the sample (`docs/agenti/metodo.md`).
    fn method(&mut self, c: usize) -> Result<String>;
    /// The header of the mzML of sample `c` (instrument, sources, hashes); `None`: `readers_mzml::scrivi` makes a plain one from the format.
    fn mzml_header(&mut self, _c: usize) -> Result<Option<Header>> {
        Ok(None)
    }
    /// What the scan table needs of `testata_mzml` (instrument, sources, analyzers); a reader whose header is costly gives a lighter one. Default: `testata_mzml`.
    fn table_header(&mut self, c: usize) -> Result<Option<Header>> {
        self.mzml_header(c)
    }
    /// What the mzML says of a spectrum besides its points (SIM mark, scan window).
    fn extra_mzml(&self, _s: &Spectrum) -> SpectrumExtra {
        SpectrumExtra::default()
    }
    /// `extra_mzml` for a reader that has to read the file again to answer (the trailer of a Thermo scan): scan `k` of sample `c`, whose points are `s`. The mzML writer asks this one; the index builders ask `extra_mzml` (cheap, no read), so what only this one gives is in the mzML and not in the rows of an index.
    fn scan_extra(&mut self, _c: usize, _k: usize, s: &Spectrum) -> SpectrumExtra {
        self.extra_mzml(s)
    }
    /// What the format holds and the mzML of the conversion does not carry, as (field, reason) with the reasons of `conv`: the account `conv/1` gives.
    fn not_carried(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }
    #[rustfmt::skip]    fn unindexed(&self) -> (usize, usize) { (0, 0) } // data sections the index lacks (an incomplete copy): (recovered, left out)
    /// The scans the index of sample `c` lists but that lie beyond the end of the data file (a truncated copy): they are not in the index.
    fn missing(&mut self, _c: usize) -> usize {
        0
    }
    /// The scans of sample `c` the file declares and the ones read (`ReadOutcomes` pairs them with the data). A reader that drops scans for another reason than a cut file overrides this; a wrapper forwards it. Default: the rows of `index_without_tic` plus `missing`.
    fn losses(&mut self, c: usize) -> Result<Losses> {
        outcome::default_losses(self, c)
    }
    /// True when the engine's reader would not read sample `c` as the file is (another instrument model, a scan type it does not know): the readers module is then the one to open it (`docs/agenti/lettori-modulo.md`).
    #[cfg(feature = "sciex")]
    fn variant(&mut self, _c: usize) -> bool {
        false
    }
}

/// A format in the registry.
pub struct ReaderEntry {
    pub id: &'static str,
    /// Version of the contract the reader was written for (`CONTRATTO`).
    pub contratto: u16,
    /// Extensions that suggest the format (for messages and for the `accept=` of the file pickers), lower case with the dot.
    pub estensioni: &'static [&'static str],
    pub riconosci: fn(&mut FileSet) -> Fiducia,
    pub apri: fn(FileSet, &Limits) -> Result<Box<dyn Reader>>,
}

/// The formats, one line each. A new format is a module that implements `Lettore`, its `ENTRY` and a line here; nothing else changes.
pub static LETTORI: &[ReaderEntry] = &[
    crate::agilent::reader::ENTRY,
    crate::wiff::reader::ENTRY,
    #[cfg(feature = "thermo")]
    crate::rawreader::ENTRY,
];

/// The entry of `registro` that is surest of `fs` (the first among equals), `None` when none is at least `Forse`.
pub fn riconosci_in<'a>(registry: &'a [ReaderEntry], fs: &mut FileSet) -> Option<&'a ReaderEntry> {
    let mut best: Option<(Fiducia, &ReaderEntry)> = None;
    for v in registry.iter().filter(|v| v.contratto == CONTRATTO) {
        let f = (v.riconosci)(fs);
        if f > Fiducia::No && best.is_none_or(|(b, _)| f > b) {
            best = Some((f, v));
        }
    }
    best.map(|(_, v)| v)
}

/// `riconosci_in` over `LETTORI`.
pub fn riconosci(fs: &mut FileSet) -> Option<&'static ReaderEntry> {
    riconosci_in(LETTORI, fs)
}

/// Opens `fs` with the reader that recognises it; `err.file.notAccepted` when none does.
pub fn apri(fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    let mut fs = fs;
    let name = fs
        .membri
        .first()
        .map(|m| m.nome.clone())
        .unwrap_or_default();
    let v =
        riconosci(&mut fs).ok_or_else(|| Error::new("err.file.notAccepted").with("name", name))?;
    (v.apri)(fs, limits)
}

/// Every extension of `registro`, in order and without repeats: the `accept=` of the pickers comes from here.
pub fn estensioni_in(registry: &[ReaderEntry]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for e in registry.iter().flat_map(|v| v.estensioni.iter().copied()) {
        if !out.contains(&e) {
            out.push(e);
        }
    }
    out
}

pub fn estensioni() -> Vec<&'static str> {
    estensioni_in(LETTORI)
}

/// The MS1 retention times and TICs of an index, in scan order (every scan when none is MS1): the trace the page draws while the `.mzidx` of a sample is
/// built, the same rule as for an mzML (`earlyTicInputs` of idx-aiuti.js over the scan table of `upload`).
pub fn tic_ms1(entries: &[ScanEntry]) -> (Vec<f64>, Vec<f64>) {
    let ms1 = entries.iter().any(|v| v.livello == 1);
    entries
        .iter()
        .filter(|v| !ms1 || v.livello == 1)
        .map(|v| (v.rt, v.tic))
        .unzip()
}

mod outcome;

#[cfg(test)]
mod tests;
