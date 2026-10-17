// Ported from Attributio src/attributio/review/libraries.py.
//
// The spectral libraries of the lab. Parsing is `mzlab_lib::parse` (MGF, MSP, MassBank records, JSON arrays and JSON lines); the index is a
// `.mzlib` built in memory by `mzlab_lib::build` (precursor-ordered table with the polarity, Flash fragment and neutral-loss indexes), next to
// the records as read: the review scores the reference peaks as the file gives them, not the cleaned and quantised ones of the `.mzlib`.
//
// Correspondence with Attributio's `FileIndex` (meta.sqlite + npy files in a cache folder):
//   meta.sqlite row ................ `LibRecord` (the metadata Attributio keeps, in parse order)
//   prec / pol / off / mz / it ..... `Library::rec` / `Library::ranges` of the `.mzlib` (sorted by polarity and precursor) and `LibRecord::{mz, it}`
//   fbin / lbin inverted indexes ... the fragment and neutral-loss indexes of the `.mzlib`, searched by `analog_candidates` (Flash hybrid search)
//   build_file_index / cache dir ... `LibraryIndex::build` (the page keeps the `.mzlib` bytes in OPFS; nothing is cached in a folder here)

use super::adduct::adduct_key;
use super::matching::Ms2;
use crate::sortc::SortC;
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::parse::{self, Format};
use mzlab_lib::read::Library;
use mzlab_lib::search::{flash_scores, FlashMode, Query};
use mzlab_lib::spec::Spectrum;

/// Folders of the libraries root that hold other kinds of files and are never read.
pub const SKIP_DIRS: [&str; 10] = [
    "Liste regolatorie",
    "Registro composti",
    "MZmine",
    "mzmine",
    "msnlib",
    "gnps_lib",
    "MassBank_NORMAN",
    "MoNA",
    "LipidBlast",
    "__pycache__",
];

/// The message of the site for a `.siriusdb` (the reader lives in the desktop app only).
pub const SIRIUSDB_SITE_MESSAGE: &str = ".siriusdb is read by the desktop app";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibFormat {
    Mgf,
    Msp,
    Json,
    MassBank,
    SiriusDb,
}

impl LibFormat {
    /// The name Attributio gives the format.
    pub fn name(self) -> &'static str {
        match self {
            LibFormat::Mgf => "mgf",
            LibFormat::Msp => "msp",
            LibFormat::Json => "json",
            LibFormat::MassBank => "massbank",
            LibFormat::SiriusDb => "siriusdb",
        }
    }
}

/// Format of a file from its name and its first bytes, or `None` when it is not a spectral library (sniffed from the content, not only the extension).
pub fn sniff(name: &str, head: &[u8]) -> Option<LibFormat> {
    let suf = name
        .rsplit_once('.')
        .map_or(String::new(), |(_, e)| format!(".{}", e.to_lowercase()));
    if suf == ".siriusdb" {
        return Some(LibFormat::SiriusDb);
    }
    let head: String = String::from_utf8_lossy(head).chars().take(4096).collect();
    match suf.as_str() {
        ".mgf" if head.contains("BEGIN IONS") => Some(LibFormat::Mgf),
        ".msp" => Some(LibFormat::Msp),
        ".json" | ".jsonl"
            if ["peaks", "peaks_json", "spectrum"]
                .iter()
                .any(|k| has_key(&head, k)) =>
        {
            Some(LibFormat::Json)
        }
        ".txt" if head.starts_with("ACCESSION:") => Some(LibFormat::MassBank),
        _ => None,
    }
}

/// `"key"` followed by optional white space and a colon, anywhere in the text.
fn has_key(text: &str, key: &str) -> bool {
    let pat = format!("\"{key}\"");
    text.match_indices(&pat)
        .any(|(i, _)| text[i + pat.len()..].trim_start().starts_with(':'))
}

/// `(path, format)` of every spectral library in a listing of `(path relative to the libraries root, first bytes)`, in the order Attributio walks
/// the folders: the files of a folder (sorted) before its sub-folders (sorted); folders in `SKIP_DIRS` and hidden ones are skipped.
pub fn find_library_files(listing: &[(String, Vec<u8>)]) -> Vec<(String, LibFormat)> {
    #[allow(clippy::type_complexity)]
    let mut found: Vec<(Vec<(bool, String)>, String, LibFormat)> = Vec::new();
    for (path, head) in listing {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let Some((file, dirs)) = parts.split_last() else {
            continue;
        };
        if dirs
            .iter()
            .any(|d| SKIP_DIRS.contains(d) || d.starts_with('.'))
        {
            continue;
        }
        if let Some(f) = sniff(file, head) {
            let mut key: Vec<(bool, String)> =
                dirs.iter().map(|d| (true, (*d).to_string())).collect();
            key.push((false, (*file).to_string()));
            found.push((key, path.clone(), f));
        }
    }
    found.sortc_by(|a, b| a.0.cmp(&b.0));
    found.into_iter().map(|(_, p, f)| (p, f)).collect()
}

/// One spectrum of a library with the metadata Attributio keeps, peaks sorted by m/z.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LibRecord {
    pub name: String,
    pub inchikey: String,
    /// First block of the InChIKey (14 characters), empty when the key is shorter.
    pub ik1: String,
    pub smiles: String,
    pub formula: String,
    pub prec: f64,
    pub adduct: String,
    /// `"POS"`, `"NEG"` or empty.
    pub pol: String,
    pub ce: String,
    pub instrument: String,
    pub entry: String,
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
}

impl LibRecord {
    pub fn ms2(&self) -> Ms2 {
        Ms2 {
            mz: self.mz.clone(),
            it: self.it.clone(),
            prec: self.prec,
        }
    }
}

fn record_of(s: &Spectrum) -> LibRecord {
    let ik = s.meta.inchikey.trim().to_uppercase();
    let mut order: Vec<usize> = (0..s.mz.len()).collect();
    order.sortc_by(|a, b| s.mz[*a].total_cmp(&s.mz[*b]));
    LibRecord {
        name: s.meta.name.trim().to_string(),
        ik1: if ik.chars().count() >= 14 {
            ik.chars().take(14).collect()
        } else {
            String::new()
        },
        inchikey: ik,
        smiles: s.meta.smiles.trim().to_string(),
        formula: s.meta.formula.trim().to_string(),
        prec: s.prec,
        adduct: s.meta.adduct.trim().to_string(),
        pol: match s.pol {
            p if p > 0 => "POS",
            p if p < 0 => "NEG",
            _ => "",
        }
        .to_string(),
        ce: s.meta.ce.trim().to_string(),
        instrument: s.meta.instrument.trim().to_string(),
        entry: s.meta.accession.trim().to_string(),
        mz: order.iter().map(|k| s.mz[*k]).collect(),
        it: order.iter().map(|k| s.it[*k]).collect(),
    }
}

fn parse_format(f: LibFormat) -> Option<Format> {
    match f {
        LibFormat::Mgf => Some(Format::Mgf),
        LibFormat::Msp => Some(Format::Msp),
        LibFormat::Json => Some(Format::Json),
        LibFormat::MassBank => Some(Format::MassBank),
        LibFormat::SiriusDb => None,
    }
}

/// The spectra of a library file with the counts of what the parser dropped. A `.siriusdb` is not read here (`SIRIUSDB_SITE_MESSAGE`).
pub fn read_library(
    format: LibFormat,
    bytes: &[u8],
) -> Result<(Vec<Spectrum>, parse::Stats), String> {
    let f = parse_format(format).ok_or_else(|| SIRIUSDB_SITE_MESSAGE.to_string())?;
    Ok(parse::parse_all(f, bytes))
}

/// The records of a library file (`iter_mgf`, `iter_msp`, `iter_massbank_record`, `iter_json` of Attributio).
pub fn read_records(format: LibFormat, bytes: &[u8]) -> Result<Vec<LibRecord>, String> {
    Ok(read_library(format, bytes)?
        .0
        .iter()
        .map(record_of)
        .collect())
}

/// A file of the index.
pub struct LibFile {
    pub label: String,
    pub records: Vec<LibRecord>,
    /// `None` when no spectrum has a peak left after the cleaning of the `.mzlib`.
    lib: Option<Library<Vec<u8>>>,
}

/// A spectrum of the index: the file and the position in its records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ref {
    pub file: usize,
    pub row: usize,
}

/// A library file to index.
pub struct LibInput {
    pub label: String,
    pub format: LibFormat,
    pub bytes: Vec<u8>,
}

#[derive(Default)]
pub struct LibraryIndex {
    pub files: Vec<LibFile>,
    pub warnings: Vec<String>,
}

impl LibraryIndex {
    /// Parses and indexes the files; the same bytes under two labels count once, a `.siriusdb` leaves a warning.
    pub fn build(inputs: Vec<LibInput>) -> LibraryIndex {
        let mut idx = LibraryIndex::default();
        let mut seen: Vec<(usize, u64)> = Vec::new();
        for inp in inputs {
            if inp.format == LibFormat::SiriusDb {
                idx.warnings
                    .push(format!("{}: {SIRIUSDB_SITE_MESSAGE}", inp.label));
                continue;
            }
            let d = digest(&inp.bytes);
            if seen.contains(&d) {
                continue;
            }
            seen.push(d);
            let Ok((spectra, _)) = read_library(inp.format, &inp.bytes) else {
                continue;
            };
            let records: Vec<LibRecord> = spectra.iter().map(record_of).collect();
            let lib = build_in_memory(&spectra, BuildOpts::default())
                .ok()
                .and_then(|(b, _)| Library::open(b, 1 << 26).ok());
            idx.files.push(LibFile {
                label: inp.label,
                records,
                lib,
            });
        }
        idx
    }

    pub fn len(&self) -> usize {
        self.files.iter().map(|f| f.records.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `(label, number of spectra)` of every file.
    pub fn summary(&self) -> Vec<(String, usize)> {
        self.files
            .iter()
            .map(|f| (f.label.clone(), f.records.len()))
            .collect()
    }
}

fn digest(b: &[u8]) -> (usize, u64) {
    (
        b.len(),
        b.iter().fold(0xcbf29ce484222325u64, |h, x| {
            (h ^ *x as u64).wrapping_mul(0x100000001b3)
        }),
    )
}

impl LibraryIndex {
    /// The record of an `.mzlib` spectrum index of a file.
    fn row_of(f: &LibFile, i: u32) -> Option<usize> {
        let r = f.lib.as_ref()?.rec(i).ok()?;
        Some(r.orig as usize).filter(|o| *o < f.records.len())
    }

    /// Spectra of the same precursor ion: |dm| <= `ppm`, same polarity (or unknown), same adduct when both are known; by precursor, then position.
    pub fn exact(&self, prec: f64, pol: &str, ppm: f64, adduct: Option<&str>) -> Vec<Ref> {
        let tol = prec * ppm * 1e-6;
        let want = adduct.and_then(adduct_key);
        let mut out: Vec<(f64, Ref)> = Vec::new();
        for (k, f) in self.files.iter().enumerate() {
            let Some(lib) = f.lib.as_ref() else { continue };
            let Ok(ranges) = lib.ranges(pol_code(pol), prec - tol, prec + tol) else {
                continue;
            };
            for i in ranges.into_iter().flatten() {
                let Some(row) = Self::row_of(f, i) else {
                    continue;
                };
                let r = &f.records[row];
                if let (Some(w), Some(g)) = (want, adduct_key(&r.adduct)) {
                    if w != g {
                        continue;
                    }
                }
                out.push((r.prec, Ref { file: k, row }));
            }
        }
        out.sortc_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then((a.1.file, a.1.row).cmp(&(b.1.file, b.1.row)))
        });
        out.into_iter().map(|(_, r)| r).collect()
    }
}

fn pol_code(pol: &str) -> i8 {
    match pol {
        "POS" => 1,
        "NEG" => -1,
        _ => 0,
    }
}

impl LibraryIndex {
    /// The `n` library spectra within ± `window_da` of the precursor (same polarity) and outside the exact window with the best Flash hybrid score
    /// (fragments and neutral losses, the peaks a modified cosine can match), for the exact scoring of `matching`.
    pub fn analog_candidates(
        &self,
        q: &Ms2,
        pol: &str,
        window_da: f64,
        n: usize,
        exclude_ppm: f64,
    ) -> Vec<Ref> {
        let Some(query) = Query::new(q.prec, pol_code(pol), &q.mz, &q.it) else {
            return Vec::new();
        };
        let mut scored: Vec<(f64, Ref)> = Vec::new();
        for (k, f) in self.files.iter().enumerate() {
            let Some(lib) = f.lib.as_ref() else { continue };
            let Ok(ranges) = lib.ranges(pol_code(pol), q.prec - window_da, q.prec + window_da)
            else {
                continue;
            };
            let Ok(sc) = flash_scores(lib, &query, FlashMode::Hybrid, 0.02, &ranges) else {
                continue;
            };
            for s in sc {
                let Some(row) = Self::row_of(f, s.index) else {
                    continue;
                };
                if (f.records[row].prec - q.prec).abs() > q.prec * exclude_ppm * 1e-6 {
                    scored.push((s.score, Ref { file: k, row }));
                }
            }
        }
        scored.sortc_by(|a, b| {
            b.0.total_cmp(&a.0)
                .then((a.1.file, a.1.row).cmp(&(b.1.file, b.1.row)))
        });
        scored.into_iter().take(n).map(|(_, r)| r).collect()
    }

    pub fn record(&self, r: Ref) -> &LibRecord {
        &self.files[r.file].records[r.row]
    }

    pub fn ms2(&self, r: Ref) -> Ms2 {
        self.record(r).ms2()
    }

    /// The first spectrum whose entry (accession) is `entry`.
    pub fn find_entry(&self, entry: &str) -> Option<Ref> {
        self.files.iter().enumerate().find_map(|(k, f)| {
            f.records
                .iter()
                .position(|r| r.entry == entry)
                .map(|row| Ref { file: k, row })
        })
    }
}
