//! The `Lettore` of a MassLynx `.raw` folder: the functions of `_FUNCTNS.INF`, the scans of the scanning ones (MS, MSe, DDA) by position in `_FUNCnnn.DAT`, and the SIR/MRM channels,
//! the TIC, the lock mass and the analog traces as chromatograms (`chroms.rs`). The lock mass function is also in the spectra (level 1, filter "lock mass", as msconvert does); its correction is not applied.

use super::cuts::{cuts, nel_file, whole_header, whole_list};
use super::dat::{decode, Calibration, Encoding};
use super::folder::{errore, text, Folder};
use super::functions::{self, Function};
use super::idx::{self, Row};
use super::inf::{self, Extern, FunctionExt, Inlet, Mode, WatersHeader};
use super::sides::{compound_names, Table};
use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{
    ChromatogramEntry, Fiducia, Field, FileSet, Format, Limits, Position, Reader, ReaderEntry,
    Sample, ScanEntry, CONTRATTO,
};
use mzlab_core::mzml::{Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::Header;

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
    /// The spectra: every scan with points of a scanning function (the lock mass included, marked in its filter), in the order of time.
    fn build_index(&mut self) {
        let mut v: Vec<(f32, usize, usize)> = Vec::new();
        for (i, f) in self.funs.iter().enumerate() {
            if f.f.mirato() || f.cod.is_none() || f.dat.is_none() {
                continue;
            }
            v.extend(
                f.righe
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.count > 0)
                    .map(|(k, r)| (r.rt, i, k)),
            );
        }
        v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        for &(_, i, k) in &v {
            let f = &self.funs[i];
            let r = f.righe[k];
            let precursor = (f.livello == 2)
                .then(|| {
                    f.sts
                        .as_ref()
                        .and_then(|t| t.value("Set Mass", k))
                        .filter(|&m| m > 0.0)
                        .or(f.ext.set_mass)
                })
                .flatten();
            self.indice.push(ScanEntry {
                id: format!("function={} process=0 scan={}", f.f.indice, k + 1),
                rt: f64::from(r.rt),
                livello: f.livello,
                polarita: f.polarita,
                tic: f64::from(r.tic),
                base_peak: None,
                filtro: if f.f.riferimento() {
                    format!("function {} lock mass", f.f.indice)
                } else {
                    format!("function {}", f.f.indice)
                },
                precursore: precursor,
                position: f.dat.map(|m| Position {
                    membro: u16::try_from(m).unwrap_or(0),
                    offset: r.offset,
                    lunghezza: u32::try_from(
                        u64::from(r.count) * f.cod.map_or(0, |c| c.width() as u64),
                    )
                    .unwrap_or(u32::MAX),
                }),
            });
            self.map.push((i, k));
        }
    }

    fn entry(&self, c: usize, k: usize) -> Result<(&Fun, usize, &ScanEntry)> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
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
        use mzlab_core::conv::{IDENTITA, NON_DECODIFICATO, OUT_OF_MODEL};
        &[
            ("drift time and mobility calibration", NON_DECODIFICATO),
            ("lock mass correction of the m/z", OUT_OF_MODEL),
            ("FUNC rows beyond Set Mass and Collision Energy, _HISTORY.INF, ramp parameters beyond _extern.inf, MALDI functions", NON_DECODIFICATO),
            ("User Name, Submitter, Laboratory Name and computer paths", IDENTITA),
        ]
    }

    fn missing(&mut self, c: usize) -> usize {
        if c == 0 {
            self.funs.iter().map(|f| f.cut).sum()
        } else {
            0
        }
    }

    fn samples(&self) -> &[Sample] {
        &self.samples
    }

    fn index(&mut self, c: usize) -> Result<&[ScanEntry]> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        Ok(&self.indice)
    }

    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let (f, r, v) = self.entry(c, k)?;
        let (cod, dat) = (
            f.cod.ok_or_else(|| errore("record width"))?,
            f.dat.ok_or_else(|| errore("no DAT"))?,
        );
        let (off, len) = idx::extension(&f.righe, r, cod.width() as u64, f.dat_len)?;
        let (tar, livello, polarity, rt, precursor) =
            (f.tar.clone(), v.livello, v.polarita, v.rt, v.precursore);
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
        if let Some(t) = &f.sts {
            for (n, x) in t.all(r).into_iter().filter(|(n, _)| n.contains("ock mass")) {
                out.push(field(&format!("{n} (not applied)"), x.to_string(), None));
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
        super::doc::file_header(self).map(Some)
    }

    fn method(&mut self, c: usize) -> Result<String> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
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
    let base = |m: &mzlab_core::lettori::Member| super::folder::base(&m.nome).to_ascii_lowercase();
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
}

pub struct WatersReader {
    pub(super) c: Folder,
    pub(super) funs: Vec<Fun>,
    pub(super) testata: WatersHeader,
    pub(super) ext: Extern,
    pub(super) inlet: Option<Inlet>,
    pub(super) analogiche: Vec<super::chroms::Analog>,
    samples: Vec<Sample>,
    indice: Vec<ScanEntry>,
    /// For each row of `indice`: (function, row of its IDX).
    map: Vec<(usize, usize)>,
    /// What the reader did not decode: (key of the notice, where).
    pub(super) warnings: Vec<(String, String)>,
    pub(super) sha1: Option<Vec<Option<String>>>,
    chroms: Option<Vec<Chromatogram>>,
}

fn opzionale(c: &mut Folder, name: &str) -> Option<Vec<u8>> {
    c.read(name).ok().flatten()
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

/// Everything of function `f`: its index, the DAT and the side tables.
fn load(
    c: &mut Folder,
    f: Function,
    ext: &Extern,
    tars: &WatersHeader,
    spent: &mut u64,
) -> Result<(Fun, Option<String>)> {
    let n = f.indice;
    let name = |e: &str| format!("_FUNC{n:03}.{e}");
    let idx_b = opzionale(c, &name("IDX"));
    let ha_idx = idx_b.is_some();
    let mut rows = match idx_b {
        Some(b) => idx::read(&b, &c.limits)?
            .ok_or_else(|| nel_file(c, "err.file.indexCut", &name("IDX")))?,
        None => Vec::new(),
    };
    *spent += rows.len() as u64;
    c.limits.ok_entries(*spent)?;
    let dat = c.search(&name("DAT"));
    let dat_len = dat.map_or(0, |i| c.fs.membri[i].dim);
    let larg = idx::width(&rows, dat_len);
    let cod = if f.mirato() {
        None
    } else {
        larg.and_then(Encoding::from_width)
    };
    let tagliate = cuts(&mut rows, larg, dat.is_some(), dat_len, ha_idx)
        .ok_or_else(|| nel_file(c, "err.file.indexCut", &name("IDX")))?;
    let lim = c.limits;
    let tab = |c: &mut Folder, e: &str| opzionale(c, &name(e)).and_then(|b| Table::read(&b, &lim));
    let (sts, ee) = (tab(c, "STS"), tab(c, "EE"));
    let nomi = opzionale(c, &name("CMP")).map_or(Vec::new(), |b| compound_names(&b, &lim));
    let x = ext.funzioni.get(&n).cloned().unwrap_or_default();
    let warning = if dat.is_none() || rows.is_empty() {
        Some(format!("function {n}: no scans"))
    } else if !f.mirato() && cod.is_none() {
        Some(format!("function {n}: record width"))
    } else {
        None
    };
    let polarity = x.polarita.or(ext.polarita).unwrap_or(0);
    let tar = tars.tarature.get(&n).cloned().unwrap_or_default();
    let fun = Fun {
        f,
        ext: x,
        righe: rows,
        dat,
        dat_len,
        larg,
        cod,
        tar,
        sts,
        ee,
        names: nomi,
        livello: 1,
        polarita: polarity,
        cut: tagliate,
    };
    Ok((fun, warning))
}

fn open(fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    let mut c = Folder::new(fs, *limits);
    // Every run has `_FUNCTNS.INF`: a folder without it is a copy that left files behind.
    let inf = c
        .read(FUNCTNS)?
        .ok_or_else(|| nel_file(&c, "err.file.folderIncomplete", FUNCTNS))?;
    if c.search(FIRST_DAT).is_none() {
        return Err(errore("_FUNC001.DAT is missing"));
    }
    if inf.is_empty() {
        return Err(Error::new("err.file.functionsEmpty").with("name", &c.name));
    }
    let list = functions::read(&inf, limits)?
        .ok_or_else(|| nel_file(&c, "err.file.folderFileCut", FUNCTNS))?;
    whole_list(&c, list.len())?;
    let header = opzionale(&mut c, "_HEADER.TXT");
    whole_header(&c, header.as_deref())?;
    let testata = header.map_or_else(WatersHeader::default, |b| inf::file_header(&text(&b)));
    let ext = opzionale(&mut c, "_extern.inf")
        .map_or_else(Extern::default, |b| inf::extern_inf(&text(&b)));
    let inlet = opzionale(&mut c, "_INLET.INF").map(|b| inf::inlet(&text(&b)));
    let (mut funs, mut warnings, mut spent) = (Vec::new(), Vec::new(), 0u64);
    for f in list {
        let (fun, av) = load(&mut c, f, &ext, &testata, &mut spent)?;
        if let Some(a) = av {
            warnings.push((
                "meta.nonDecodificato".to_string(),
                format!("{}: {a}", c.name),
            ));
        }
        funs.push(fun);
    }
    for i in 0..funs.len() {
        funs[i].livello = level_of(&funs, i);
    }
    if funs.iter().any(|f| f.ext.mobilita) {
        warnings.push((
            "meta.nonDecodificato".into(),
            format!("{}: ion mobility", c.name),
        ));
    }
    let analogiche = super::chroms::analog(&mut c);
    unknown_variant(&c.name, &funs, analogiche.is_empty())?;
    let mut l = WatersReader {
        c,
        funs,
        testata,
        ext,
        inlet,
        analogiche,
        samples: Vec::new(),
        indice: Vec::new(),
        map: Vec::new(),
        warnings,
        sha1: None,
        chroms: None,
    };
    l.build_index();
    let name = l
        .testata
        .get("Acquired Name")
        .map_or_else(|| l.c.name.clone(), str::to_string);
    l.samples = vec![Sample {
        id: 0,
        nome: name,
        scansioni: l.indice.len(),
        size: None,
    }];
    Ok(Box::new(l))
}
