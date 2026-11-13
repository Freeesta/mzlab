//! The `Lettore` of a MassLynx `.raw` folder: the functions of `_FUNCTNS.INF`, the scans of the scanning ones (MS, MSe, DDA) by position in `_FUNCnnn.DAT`, and the SIR/MRM channels,
//! the TIC, the lock mass and the analog traces as chromatograms (`chroms.rs`). The lock mass function is also in the spectra (level 1, filter "lock mass", as msconvert does); its correction is not applied.

use super::cuts::{header_cut, in_file, unlisted};
use super::dat::{decode, Calibration, Encoding};
use super::folder::{read_error, text, Folder};
use super::functions::{self, Function};
use super::idx::{self, Row};
use super::inf::{self, Extern, FunctionExt, Inlet, Mode, WatersHeader};
use super::load::Loading;
use super::sides::Table;
use mzlab_core::error::{Error, Result};
use mzlab_core::mzml::{Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::Header;
use mzlab_core::readers::{
    ChromatogramEntry, Fiducia, Field, FileSet, Format, Limits, Losses, Reader, ReaderEntry,
    Sample, ScanEntry, CONTRATTO,
};

const FUNCTNS: &str = "_FUNCTNS.INF";
const FIRST_DAT: &str = "_FUNC001.DAT";

pub const ENTRY: ReaderEntry = ReaderEntry {
    id: "waters-raw",
    contratto: CONTRATTO,
    estensioni: &[".raw"],
    riconosci: recognize,
    apri: open,
};

impl WatersReader {
    fn entry(&mut self, c: usize, k: usize) -> Result<(&Fun, usize, &ScanEntry)> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        self.ready()?;
        let (i, r) = *self
            .map
            .get(k)
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        Ok((&self.funs[i], r, &self.indice[k]))
    }
}

impl Reader for WatersReader {
    fn format(&self) -> Format {
        Format::Altro("waters-raw")
    }

    fn not_carried(&self) -> &'static [(&'static str, &'static str)] {
        use mzlab_core::conv::{IDENTITY, NOT_DECODED, OUT_OF_MODEL};
        &[
            ("drift time and mobility calibration", NOT_DECODED),
            ("lock mass correction of the m/z", OUT_OF_MODEL),
            ("FUNC rows beyond Set Mass and Collision Energy, _HISTORY.INF, ramp parameters beyond _extern.inf, MALDI functions", NOT_DECODED),
            ("User Name, Submitter, Laboratory Name and computer paths", IDENTITY),
        ]
    }

    /// Every scan the index lists and the reader does not read (a DAT cut short, a member missing, a record width or a damaged part), as `losses`.
    fn missing(&mut self, c: usize) -> usize {
        self.losses(c).map_or(0, |l| l.lost())
    }

    fn losses(&mut self, c: usize) -> Result<Losses> {
        let n = self.index_without_tic(c)?.len();
        let damaged = self.bad + self.unlisted;
        Ok(super::counts::losses(&self.funs, c, n, damaged))
    }

    fn samples(&self) -> &[Sample] {
        &self.samples
    }

    fn index(&mut self, c: usize) -> Result<&[ScanEntry]> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        self.ready()?;
        Ok(&self.indice)
    }

    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let (f, r, v) = self.entry(c, k)?;
        let (cod, dat) = (
            f.cod.ok_or_else(|| read_error("record width"))?,
            f.dat.ok_or_else(|| read_error("no DAT"))?,
        );
        let (off, len) = idx::extension(&f.righe, r, cod.width() as u64, f.dat_len)?;
        let tar = f.tar.for_mark(f.righe[r].mark);
        let (livello, polarity, rt, precursor) = (v.livello, v.polarita, v.rt, v.precursore);
        let id = v.id.clone();
        let filter = v.filtro.clone();
        let b = self.c.read_from(dat, off, len)?;
        let (mz, intensity) = decode(cod, &b, &tar)?;
        Ok(Spectrum {
            index: k,
            id,
            level: livello,
            rt,
            polarity,
            profile: cod.profilo(),
            filter,
            precursor,
            mz,
            intensity,
            wide: None,
        })
    }

    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Field>> {
        let (f, r, v) = self.entry(c, k)?;
        let field = |name: &str, value: String, units: Option<&str>| Field {
            nome: name.into(),
            valore: value,
            unita: units.map(Into::into),
            cv: None,
        };
        let mut out = vec![
            field("function", f.f.indice.to_string(), None),
            field("scan", (r + 1).to_string(), None),
        ];
        if let Some((a, b)) = f.f.interval() {
            out.push(field("scan range", format!("{a}-{b}"), Some("m/z")));
        }
        let ce = f
            .sts
            .as_ref()
            .and_then(|t| t.value("Collision Energy", r))
            .filter(|&x| x > 0.0)
            .or(f.ext.ce);
        if let Some(ce) = ce {
            out.push(field("collision energy", ce.to_string(), Some("eV")));
        }
        if let Some((a, b)) = f.ext.rampa {
            out.push(field(
                "collision energy ramp",
                format!("{a}-{b}"),
                Some("eV"),
            ));
        }
        if f.f.riferimento() {
            out.push(field("role", "lock mass reference".into(), None));
        } else if f.livello == 2 && v.precursore.is_none() {
            // MSe: the whole range is fragmented, msconvert's 775 +- 725 m/z "precursor" is only the scan range
            out.push(field(
                "acquisition",
                "MSe, no precursor selection".into(),
                None,
            ));
        }
        if let Some(p) = v.precursore {
            out.push(field("precursor", p.to_string(), Some("m/z")));
        }
        let stored = f.righe[r].mark == idx::CALIBRATED;
        out.push(field("m/z calibration", f.tar.label(stored).into(), None));
        if let Some(t) = &f.sts {
            let note =
                ["(not applied)", "(applied by the acquisition, if at all)"][usize::from(stored)];
            for (n, x) in t.all(r).into_iter().filter(|(n, _)| n.contains("ock mass")) {
                out.push(field(&format!("{n} {note}"), x.to_string(), None));
            }
        }
        Ok(out)
    }

    fn chromatograms(&mut self, c: usize) -> Result<Vec<ChromatogramEntry>> {
        Ok(self
            .all_chromatograms(c)?
            .iter()
            .map(|h| ChromatogramEntry {
                id: h.id.clone(),
                tipo: h.kind,
                unita: h.unit.clone(),
            })
            .collect())
    }

    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.all_chromatograms(c)?
            .iter()
            .find(|h| h.id == id)
            .cloned()
            .ok_or_else(|| Error::new("error.badIndex").with("id", id))
    }

    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        self.ready()?;
        super::doc::file_header(self).map(Some)
    }

    fn method(&mut self, c: usize) -> Result<String> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        self.ready()?;
        super::doc::document(self)
    }
}

impl WatersReader {
    /// The chromatograms of the folder, built once.
    fn all_chromatograms(&mut self, c: usize) -> Result<&[Chromatogram]> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        if self.chroms.is_none() {
            let v = super::chroms::all_of_them(self)?;
            self.chroms = Some(v);
        }
        Ok(self.chroms.as_deref().unwrap_or_default())
    }
}

/// A folder with `_FUNC001.DAT` is a MassLynx folder; it is surer when `_FUNCTNS.INF` is there and has whole records.
fn recognize(fs: &mut FileSet) -> Fiducia {
    let base = |m: &mzlab_core::readers::Member| super::folder::base(&m.nome).to_ascii_lowercase();
    if !fs.membri.iter().any(|m| base(m) == "_func001.dat") {
        return Fiducia::No;
    }
    match fs.membri.iter().find(|m| base(m) == "_functns.inf") {
        Some(m) if m.dim > 0 && m.dim % 416 == 0 => Fiducia::Si,
        _ => Fiducia::Forse,
    }
}

/// One function with everything read of it when the folder is opened (the DAT is read by position).
pub(super) struct Fun {
    pub(super) f: Function,
    pub(super) ext: FunctionExt,
    pub(super) righe: Vec<Row>,
    /// Index of `_FUNCnnn.DAT` among the members.
    pub(super) dat: Option<usize>,
    pub(super) dat_len: u64,
    /// Bytes per record, from the index.
    pub(super) larg: Option<u64>,
    pub(super) cod: Option<Encoding>,
    pub(super) tar: Calibration,
    pub(super) sts: Option<Table>,
    pub(super) ee: Option<Table>,
    pub(super) names: Vec<String>,
    pub(super) livello: u8,
    pub(super) polarita: i8,
    /// Scans of the index past the end of a DAT cut short (emptied in `righe`).
    pub(super) cut: usize,
    /// Rows of the index of a function left out whole as damaged (an index that does not match its DAT, a calibration line lost): no data is read from it.
    pub(super) left_out: Option<usize>,
}

pub struct WatersReader {
    pub(super) c: Folder,
    pub(super) funs: Vec<Fun>,
    pub(super) testata: WatersHeader,
    pub(super) ext: Extern,
    pub(super) inlet: Option<Inlet>,
    pub(super) analogiche: Vec<super::chroms::Analog>,
    pub(super) samples: Vec<Sample>,
    pub(super) indice: Vec<ScanEntry>,
    /// For each row of `indice`: (function, row of its IDX).
    pub(super) map: Vec<(usize, usize)>,
    /// Whether the scans of `indice` were checked (`ready`).
    pub(super) checked: bool,
    /// Scans `ready` left out as damaged.
    pub(super) bad: usize,
    /// Index rows of the functions past the end of a `_FUNCTNS.INF` cut short at a whole record.
    pub(super) unlisted: usize,
    /// What the reader did not decode: (key of the notice, where).
    pub(super) warnings: Vec<(String, String)>,
    pub(super) sha1: Option<Vec<Option<String>>>,
    chroms: Option<Vec<Chromatogram>>,
}

fn level_of(funs: &[Fun], i: usize) -> u8 {
    let (f, x) = (&funs[i].f, &funs[i].ext);
    if f.riferimento() {
        return 1;
    }
    match x.mode {
        Mode::Riferimento => 1,
        Mode::Msms | Mode::Figlie => 2,
        Mode::Ms => {
            let partner = funs.iter().any(|g| {
                g.f.indice != f.indice
                    && !g.f.riferimento()
                    && g.ext.mode == Mode::Ms
                    && g.ext.rampa.is_none()
            });
            if x.rampa.is_some() && partner {
                2
            } else {
                1
            }
        }
        _ if f.indice == 1 || funs.len() == 1 => 1,
        _ => 2,
    }
}

/// A folder that gives nothing (no scan, no channel, no analogue trace) because a function with data has records of a width not read yet is refused with
/// that function named, rather than opened empty.
fn unknown_variant(name: &str, funs: &[Fun], without_analog: bool) -> Result<()> {
    let data = |f: &Fun| f.dat.is_some() && f.righe.iter().any(|r| r.count > 0);
    if !without_analog
        || funs
            .iter()
            .any(|f| data(f) && (f.f.mirato() || f.cod.is_some()))
    {
        return Ok(());
    }
    match funs.iter().find(|f| data(f) && !f.f.mirato()) {
        Some(f) => Err(Error::new("err.file.recordVariant")
            .with("name", name)
            .with("function", f.f.indice.to_string())
            .with("type", format!("0x{:02X}", f.f.kind))),
        None => Ok(()),
    }
}

/// A notice of folder `name`: what is not decoded or was left out.
fn note(w: &mut Vec<(String, String)>, name: &str, a: String) {
    w.push(("meta.nonDecodificato".to_string(), format!("{name}: {a}")));
}

/// The functions `_FUNCTNS.INF` lists. Every run has it: a folder without it is a copy that left files behind.
fn listed(c: &mut Folder) -> Result<Vec<Function>> {
    let inf = c
        .read(FUNCTNS)?
        .ok_or_else(|| in_file(c, "err.file.folderIncomplete", FUNCTNS))?;
    if c.search(FIRST_DAT).is_none() {
        return Err(read_error("_FUNC001.DAT is missing"));
    }
    if inf.is_empty() {
        return Err(Error::new("err.file.functionsEmpty").with("name", &c.name));
    }
    functions::read(&inf, &c.limits)?.ok_or_else(|| in_file(c, "err.file.folderFileCut", FUNCTNS))
}

impl WatersReader {
    /// The index of the spectra and the one sample, named as the acquisition named it.
    fn with_index(mut self) -> Self {
        self.build_index();
        let name = self
            .testata
            .get("Acquired Name")
            .map_or_else(|| self.c.name.clone(), str::to_string);
        self.samples = vec![Sample {
            id: 0,
            nome: name,
            scansioni: self.indice.len(),
            size: None,
        }];
        self
    }
}

fn open(fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    let mut c = Folder::new(fs, *limits);
    let list = listed(&mut c)?;
    let (more, unlisted) = unlisted(&mut c, list.len())?;
    let mut warnings = Vec::new();
    let mut refusal = None;
    if more > 0 {
        refusal = Some(in_file(&c, "err.file.functionsCut", FUNCTNS).with("listed", list.len()));
        let a = format!("{more} functions past the end of {FUNCTNS}, {unlisted} scans left out");
        note(&mut warnings, &c.name, a);
    }
    let header = c.read("_HEADER.TXT")?;
    let has_header = header.is_some();
    let header_cut = header_cut(header.as_deref());
    let testata = header.map_or_else(WatersHeader::default, |b| inf::file_header(&text(&b)));
    let ext = c
        .read("_extern.inf")?
        .map_or_else(Extern::default, |b| inf::extern_inf(&text(&b)));
    let inlet = c.read("_INLET.INF")?.map(|b| inf::inlet(&text(&b)));
    let mut ld = Loading {
        ext: &ext,
        tars: &testata,
        header_cut,
        spent: 0,
        refusal,
    };
    let mut funs = Vec::new();
    for f in list {
        let (fun, av) = ld.load(&mut c, f)?;
        if let Some(a) = av {
            note(&mut warnings, &c.name, a);
        }
        funs.push(fun);
    }
    for i in 0..funs.len() {
        funs[i].livello = level_of(&funs, i);
    }
    warnings.extend(super::counts::notes(&c.name, &funs, &ext, has_header));
    let (analogiche, a) = super::chroms::analog(&mut c);
    if let Some(a) = a {
        note(&mut warnings, &c.name, a);
    }
    // a folder whose every part with data was left out as damaged gives nothing: refused with the key of the first, as before any part was left out
    let data = funs
        .iter()
        .any(|f| f.dat.is_some() && f.righe.iter().any(|r| r.count > 0));
    if let Some(e) = ld.refusal.filter(|_| !data && analogiche.is_empty()) {
        return Err(e);
    }
    unknown_variant(&c.name, &funs, analogiche.is_empty())?;
    let l = WatersReader {
        c,
        funs,
        testata,
        ext,
        inlet,
        analogiche,
        samples: Vec::new(),
        indice: Vec::new(),
        map: Vec::new(),
        checked: false,
        bad: 0,
        unlisted,
        warnings,
        sha1: None,
        chroms: None,
    };
    Ok(Box::new(l.with_index()))
}
