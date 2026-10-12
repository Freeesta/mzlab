//! The contract of the readers of vendor files (`docs/agenti/estendere.md`): a `Lettore` serves the scans of a file by position, one at a time, so that the pages
//! read a `.raw`, a `.wiff` or a `.D` as they read an mzML, and the conversion to mzML is one export among the others (`scrivi_mzml`, the one writer).
//! A format is a `VoceLettore` (how to recognise it, how to open it) in the static registry `LETTORI`; nothing is loaded at run time.
//! `Limiti` is passed to every `apri`: depth, entries and bytes that a damaged or hostile file may ask for are bounded by construction.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::mzml::{ChromKind, Chromatogram, Spectrum};

/// Version of this contract; a `VoceLettore` states the one it was written for.
pub const CONTRATTO: u16 = 1;

/// The formats the readers know (the `id` is the one of the registry and of `metodo/1.origine.formato`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formato {
    ThermoRaw,
    SciexWiff,
    SciexDam,
    AgilentD,
    Mzml,
    Altro(&'static str),
}

impl Formato {
    pub fn id(&self) -> &'static str {
        match self {
            Formato::ThermoRaw => "thermo-raw",
            Formato::SciexWiff => "sciex-wiff",
            Formato::SciexDam => "sciex-dam",
            Formato::AgilentD => "agilent-d",
            Formato::Mzml => "mzml",
            Formato::Altro(id) => id,
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
pub struct Limiti {
    /// Nesting of elements, directories, chains.
    pub profondita: usize,
    /// Entries of any table (scans, directory entries, records).
    pub voci: usize,
    /// Bytes held in memory at once.
    pub byte: u64,
}

impl Limiti {
    /// The browser: 256 MB per WASM instance, 192 of them for work (`budget::WASM_BUDGET_MB`).
    pub const fn sito() -> Limiti {
        Limiti {
            profondita: 256,
            voci: 5_000_000,
            byte: (crate::budget::WASM_BUDGET_MB as u64) << 20,
        }
    }
    /// The desktop app.
    pub const fn desktop() -> Limiti {
        Limiti {
            profondita: 256,
            voci: 50_000_000,
            byte: (crate::budget::NATIVE_BUDGET_MB as u64) << 20,
        }
    }
    /// `n` entries asked by a file: an error with a key when they are more than the limit (and so never an allocation).
    pub fn voci_ok(&self, n: u64) -> Result<usize> {
        usize::try_from(n)
            .ok()
            .filter(|&n| n <= self.voci)
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

impl Default for Limiti {
    fn default() -> Limiti {
        Limiti::sito()
    }
}

/// One file of a set, with the bytes at any offset.
pub struct Membro {
    pub nome: String,
    pub dim: u64,
    pub dati: Box<dyn ReadAt>,
}

/// The files that make one dataset: `X.wiff` + `X.wiff.scan`, the members of a `.raw` or `.D` folder, or a single file. The names are logical (relative to the dataset).
#[derive(Default)]
pub struct FileSet {
    pub membri: Vec<Membro>,
}

impl FileSet {
    /// Index of the member whose name ends with `suffisso`, ignoring case.
    pub fn trova(&self, suffisso: &str) -> Option<usize> {
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
pub struct Campione {
    pub id: u32,
    pub nome: String,
    pub scansioni: usize,
}

/// Where the bytes of a scan lie in the original: member of the `FileSet`, offset, length (what the `.mzidx` of a vendor file will store).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Posizione {
    pub membro: u16,
    pub offset: u64,
    pub lunghezza: u32,
}

/// One row of the index of a sample: what is known of a scan before its peaks are read.
#[derive(Debug, Clone, PartialEq)]
pub struct VoceScansione {
    pub id: String,
    /// Minutes.
    pub rt: f64,
    pub livello: u8,
    /// +1, -1 or 0 (not said).
    pub polarita: i8,
    pub tic: f64,
    pub picco_base: Option<f64>,
    pub filtro: String,
    pub precursore: Option<f64>,
    pub posizione: Option<Posizione>,
}

/// A name, a value and, when the file or the CV says so, a unit and an accession (`MS:1000927`): what the header of a scan holds besides its peaks.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Campo {
    pub nome: String,
    pub valore: String,
    pub unita: Option<String>,
    pub cv: Option<String>,
}

/// A chromatogram of the file without its points.
#[derive(Debug, Clone, PartialEq)]
pub struct VoceCromatogramma {
    pub id: String,
    pub tipo: ChromKind,
    pub unita: String,
}

/// A reader of one dataset. Random access: `indice` is read once, `scansione` reads only the bytes of one scan; streaming is `scansione(c, 0..n)`.
/// An error is a key (`err.file.*`), never a panic; what a file lacks is absent from the answer, not an error.
pub trait Lettore {
    fn formato(&self) -> Formato;
    /// At least one.
    fn campioni(&self) -> &[Campione];
    /// The scans of sample `c`, in the order of time.
    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]>;
    /// Scan `k` of the index of sample `c`.
    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum>;
    /// The header fields of scan `k` (collision energy, isolation, trailer): `nome`, `valore`, `unita`, `cv`.
    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>>;
    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>>;
    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram>;
    /// The `metodo/1` document of the sample (`docs/agenti/metodo.md`), anonymous when the reader was asked for it.
    fn metodo(&mut self, c: usize) -> Result<String>;
}

/// A format in the registry.
pub struct VoceLettore {
    pub id: &'static str,
    /// Version of the contract the reader was written for (`CONTRATTO`).
    pub contratto: u16,
    /// Extensions that suggest the format (for messages and for the `accept=` of the file pickers), lower case with the dot.
    pub estensioni: &'static [&'static str],
    pub riconosci: fn(&mut FileSet) -> Fiducia,
    pub apri: fn(FileSet, &Limiti) -> Result<Box<dyn Lettore>>,
}

/// The formats, one line each. A new format is a module that implements `Lettore`, its `VOCE` and a line here; nothing else changes.
pub static LETTORI: &[VoceLettore] = &[];

/// The entry of `registro` that is surest of `fs` (the first among equals), `None` when none is at least `Forse`.
pub fn riconosci_in<'a>(registro: &'a [VoceLettore], fs: &mut FileSet) -> Option<&'a VoceLettore> {
    let mut best: Option<(Fiducia, &VoceLettore)> = None;
    for v in registro.iter().filter(|v| v.contratto == CONTRATTO) {
        let f = (v.riconosci)(fs);
        if f > Fiducia::No && best.is_none_or(|(b, _)| f > b) {
            best = Some((f, v));
        }
    }
    best.map(|(_, v)| v)
}

/// `riconosci_in` over `LETTORI`.
pub fn riconosci(fs: &mut FileSet) -> Option<&'static VoceLettore> {
    riconosci_in(LETTORI, fs)
}

/// Opens `fs` with the reader that recognises it; `err.file.notAccepted` when none does.
pub fn apri(fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    let mut fs = fs;
    let name = fs
        .membri
        .first()
        .map(|m| m.nome.clone())
        .unwrap_or_default();
    let v =
        riconosci(&mut fs).ok_or_else(|| Error::new("err.file.notAccepted").with("name", name))?;
    (v.apri)(fs, limiti)
}

/// Every extension of `registro`, in order and without repeats: the `accept=` of the pickers comes from here.
pub fn estensioni_in(registro: &[VoceLettore]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for e in registro.iter().flat_map(|v| v.estensioni.iter().copied()) {
        if !out.contains(&e) {
            out.push(e);
        }
    }
    out
}

pub fn estensioni() -> Vec<&'static str> {
    estensioni_in(LETTORI)
}

#[cfg(test)]
mod tests;
