//! The readers of the vendor files, one module `.wasm` per family of formats (docs/agenti/lettori-modulo.md): the engine (`mzlab-wasm`) carries none of this crate.
//! The readers of a family sit in `src/<family>/` behind the Cargo feature of the same name and implement `mzlab_core::readers::Lettore`; variants of the readers that live
//! in `mzlab-core` sit behind its `estesi` feature, which this crate turns on and the engine leaves off.
use mzlab_core::error::{Error, Result};
use mzlab_core::readers::{self, Fiducia, FileSet, Limits, Reader, ReaderEntry};

#[cfg(any(feature = "andi", feature = "tutte"))]
pub mod andi;
#[cfg(any(feature = "bruker", feature = "tutte"))]
pub mod bruker;
pub use mzlab_core::readers_finite as finite;
pub mod json;
pub mod not_carried;
#[cfg(any(feature = "aperti", feature = "tutte"))]
pub mod open;
#[cfg(any(feature = "shimadzu", feature = "tutte"))]
pub mod shimadzu;
pub mod srm;
#[cfg(any(feature = "waters", feature = "tutte"))]
pub mod waters;

// The readers of each family, one `static` per `ReaderEntry` of the registry; a family without readers yet is born with its own package.
#[cfg(any(feature = "aperti", feature = "tutte"))]
static ENTRY_MZML_NUMPRESS: ReaderEntry = open::mzml_numpress::ENTRY;
#[cfg(any(feature = "aperti", feature = "tutte"))]
static ENTRY_MZXML: ReaderEntry = open::mzxml::ENTRY;
#[cfg(any(feature = "aperti", feature = "tutte"))]
static ENTRY_MZML_GZ: ReaderEntry = open::mzml_gz::ENTRY;
// the page gunzips an mzXML.gz itself (OPFS): only the app and the CLI carry this voice, so the module `aperti` keeps its size
#[cfg(feature = "tutte")]
static ENTRY_MZXML_GZ: ReaderEntry = open::mzml_gz::ENTRY_MZXML;
#[cfg(any(feature = "andi", feature = "tutte"))]
static ENTRY_ANDI: ReaderEntry = andi::ms::ENTRY;
#[cfg(any(feature = "agilent", feature = "tutte"))]
static ENTRY_AGILENT: ReaderEntry = mzlab_core::agilent::reader::ENTRY;
#[cfg(any(feature = "sciex", feature = "tutte"))]
static ENTRY_SCIEX: ReaderEntry = mzlab_core::wiff::reader::ENTRY;
#[cfg(any(feature = "shimadzu", feature = "tutte"))]
static ENTRY_SHIMADZU: ReaderEntry = shimadzu::ENTRY;
#[cfg(any(feature = "thermo", feature = "tutte"))]
static ENTRY_THERMO: ReaderEntry = mzlab_core::rawreader::ENTRY;
#[cfg(any(feature = "bruker", feature = "tutte"))]
static ENTRY_BRUKER_TDF: ReaderEntry = bruker::tdf::ENTRY;
#[cfg(any(feature = "bruker", feature = "tutte"))]
static ENTRY_BRUKER_TSF: ReaderEntry = bruker::tdf::ENTRY_TSF;
#[cfg(any(feature = "waters", feature = "tutte"))]
static ENTRY_WATERS: ReaderEntry = waters::ENTRY;

/// The name of the family this build serves (`"tutte"` for the desktop app and for tests).
pub fn famiglia() -> &'static str {
    if cfg!(feature = "tutte") {
        "tutte"
    } else if cfg!(feature = "agilent") {
        "agilent"
    } else if cfg!(feature = "sciex") {
        "sciex"
    } else if cfg!(feature = "thermo") {
        "thermo"
    } else if cfg!(feature = "aperti") {
        "aperti"
    } else if cfg!(feature = "andi") {
        "andi"
    } else if cfg!(feature = "waters") {
        "waters"
    } else if cfg!(feature = "shimadzu") {
        "shimadzu"
    } else if cfg!(feature = "bruker") {
        "bruker"
    } else {
        ""
    }
}

/// The formats of the families that are on: `agilent` and `sciex` repeat the ones of the engine's registry (and serve their variants), `thermo` is the only home of the `.raw` reader of the web engine (the desktop app has it in the core with the default features).
pub fn registry() -> Vec<&'static ReaderEntry> {
    let entries: &[Option<&'static ReaderEntry>] = &[
        #[cfg(any(feature = "aperti", feature = "tutte"))]
        Some(&ENTRY_MZML_NUMPRESS),
        #[cfg(any(feature = "aperti", feature = "tutte"))]
        Some(&ENTRY_MZXML),
        #[cfg(any(feature = "aperti", feature = "tutte"))]
        Some(&ENTRY_MZML_GZ),
        #[cfg(feature = "tutte")]
        Some(&ENTRY_MZXML_GZ),
        #[cfg(any(feature = "andi", feature = "tutte"))]
        Some(&ENTRY_ANDI),
        #[cfg(any(feature = "agilent", feature = "tutte"))]
        Some(&ENTRY_AGILENT),
        #[cfg(any(feature = "sciex", feature = "tutte"))]
        Some(&ENTRY_SCIEX),
        #[cfg(any(feature = "shimadzu", feature = "tutte"))]
        Some(&ENTRY_SHIMADZU),
        #[cfg(any(feature = "thermo", feature = "tutte"))]
        Some(&ENTRY_THERMO),
        #[cfg(any(feature = "bruker", feature = "tutte"))]
        Some(&ENTRY_BRUKER_TDF),
        #[cfg(any(feature = "bruker", feature = "tutte"))]
        Some(&ENTRY_BRUKER_TSF),
        #[cfg(any(feature = "waters", feature = "tutte"))]
        Some(&ENTRY_WATERS),
    ];
    entries.iter().flatten().copied().collect()
}

/// The reader of `registro` that is surest of `fs` (the first among equals; like `readers::riconosci_in`, which takes a slice and not this list).
pub fn riconosci(fs: &mut FileSet) -> Option<&'static ReaderEntry> {
    let mut best: Option<(Fiducia, &'static ReaderEntry)> = None;
    for v in registry()
        .into_iter()
        .filter(|v| v.contratto == readers::CONTRATTO)
    {
        let f = (v.riconosci)(fs);
        if f > Fiducia::No && best.is_none_or(|(b, _)| f > b) {
            best = Some((f, v));
        }
    }
    best.map(|(_, v)| v)
}

/// Opens `fs` with the reader that recognises it, as it is (the sessions of the modules: the engine decides itself what it does with SRM scans), except
/// a Thermo `.raw`: a TSQ keeps its SRM transitions only as scans, so its reader is wrapped to give them as chromatograms (`srm::SrmFromScans`). Every reader
/// answers through `finite::Finished`: a NaN or infinite number of the file is an error, not a value.
pub fn open_bare(mut fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    let name = fs
        .membri
        .first()
        .map(|m| m.nome.clone())
        .unwrap_or_default();
    let v = riconosci(&mut fs).ok_or_else(|| refusal(&fs, name))?;
    let mut l = (v.apri)(fs, limits)?;
    if l.format() == readers::Format::ThermoRaw && srm::has_srm_scans(&mut *l, 0) {
        l = Box::new(srm::SrmFromScans::new(l));
    }
    Ok(Box::new(finite::Finished::new(l)))
}

/// The extensions that name one format read only by this crate: a file so named that no reader takes is a cut or damaged copy, not another format (`.mzML`, `.mzML.gz`,
/// `.wiff` and the folders are left out: the engine reads the plain ones, a `.wiff` needs its pair, a folder member is named by its folder).
const ONLY_HERE: [&str; 8] = [
    ".mzxml",
    ".mzxml.gz",
    ".cdf",
    ".nc",
    ".qgd",
    ".lcd",
    ".raw",
    ".wiff.scan",
];

/// Why no reader takes `fs`: a member named like a format of the registry that is empty (`err.file.empty`), or whose start is not that of its format
/// (`err.file.notThisFormat`, a cut or damaged copy); anything else is `err.file.notAccepted` (the engine of the site then asks the readers module).
fn refusal(fs: &FileSet, name: String) -> Error {
    let short = |n: &str| n.rsplit(['/', '\\']).next().unwrap_or(n).to_string();
    for m in &fs.membri {
        let n = m.nome.to_ascii_lowercase();
        let all_of_them = estensioni();
        let Some(e) = all_of_them
            .iter()
            .filter(|e| n.ends_with(**e))
            .max_by_key(|e| e.len())
        else {
            continue;
        };
        if m.dim == 0 {
            return Error::new("err.file.empty").with("name", short(&m.nome));
        }
        if ONLY_HERE.contains(e) {
            return Error::new("err.file.notThisFormat")
                .with("name", short(&m.nome))
                .with("ext", e)
                .with("size", m.dim);
        }
    }
    Error::new("err.file.notAccepted").with("name", name)
}

/// Opens `fs` with the reader that recognises it; the SRM traces of scans with an SRM filter become chromatograms (`srm::SrmFromScans`).
pub fn apri(fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    Ok(Box::new(srm::SrmFromScans::new(open_bare(fs, limits)?)))
}

/// Every extension of the registry of this build, without repeats.
pub fn estensioni() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for e in registry().iter().flat_map(|v| v.estensioni.iter().copied()) {
        if !out.contains(&e) {
            out.push(e);
        }
    }
    out
}

/// Why the engine does not do for sample `c` of `l`: `"srm"` when its SRM traces exist only as scans, `"variante"` when the file is of an instrument or a scan type the engine's reader
/// was not written for (SCIEX other than the 3200 QTRAP, scheduled MRM), in sample `c` or in any other sample: the page hands the whole pair to one reader.
pub fn motivi(l: &mut dyn Reader, c: usize) -> Vec<&'static str> {
    let mut m = Vec::new();
    if srm::has_srm_scans(l, c) {
        m.push("srm");
    }
    #[cfg(feature = "sciex")]
    if l.variant(c) || (0..l.samples().len()).any(|k| k != c && l.variant(k)) {
        m.push("variante");
    }
    m
}

/// Why the engine does not do for the dataset `fs` (before it is opened): `"variante"` when it is an Agilent `.D` of a variant the engine's reader cannot read the
/// points of (QqQ MRM, Q-TOF profile or time-of-flight spectra, compressed spectra), so that the page does not try the engine first.
#[cfg(any(feature = "agilent", feature = "tutte"))]
pub fn reasons_together(fs: &mut FileSet) -> Vec<&'static str> {
    let mut m = Vec::new();
    if mzlab_core::agilent::reader::variant(fs) {
        m.push("variante");
    }
    m
}
