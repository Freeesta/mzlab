//! The family `shimadzu`: LabSolutions / GCMSsolution files, OLE2 containers read with `mzlab_core::ole`. `.qgd` (GC/MS, `GCMS Raw Data`) and `.lcd` of a single quadrupole (`Mass Raw Data`:
//! scans and SIM) and of a triple quadrupole (`TLM Raw Data`: MRM as SRM chromatograms). The Q-TOF `.lcd` (`QTFL RawData`) is recognised and refused with `err.file.shimadzu`.
//! Written from the files themselves (stream layout in docs/agenti/shimadzu.md); the Apache-2.0 crate `openszraw` was read to learn where the streams are, not copied.

mod dir;
mod load;
mod meta;
mod mrm;
mod prop;
mod reader;
mod rec;

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::ole::Ole;
use mzlab_core::readers::{Fiducia, FileSet, Limits, Reader, ReaderEntry, Sample, CONTRATTO};

use load::{Data, Fam};
use meta::Meta;

/// Above this intensity an `f32` loses counts: the spectrum then keeps the exact values.
const F32_EXACT: u64 = 1 << 24;

pub const ENTRY: ReaderEntry = ReaderEntry {
    id: "shimadzu",
    contratto: CONTRATTO,
    estensioni: &[".qgd", ".lcd"],
    riconosci: recognize,
    apri,
};

/// What a container holds, from the names of its directory (a storage can be there and empty: the data are told by `carica::scegli`). `Ignoto` and `Rotto`: a `.qgd`/`.lcd`
/// that is an OLE2 container of other storages (another instrument's data), or whose directory lies beyond its end: refused with what was found, or as `err.file.cutShort` when its header points past the end (a cut copy).
enum Content {
    Data,
    QTof,
    LcOnly,
    Ignoto(String),
    Broken,
}

fn content(names: &[String]) -> Option<Content> {
    let ha = |n: &str| names.iter().any(|x| x == n);
    if load::families().iter().any(|f| ha(f.storage())) {
        Some(Content::Data)
    } else if ha("QTFL RawData") {
        Some(Content::QTof)
    } else if ha("LSS Raw Data") {
        Some(Content::LcOnly)
    } else {
        None
    }
}

/// The member of `fs` that is a Shimadzu container, and what it holds; failing that, a `.qgd`/`.lcd` OLE2 container it cannot read the data of.
fn find(fs: &mut FileSet) -> Option<(usize, Content)> {
    (0..fs.membri.len())
        .find_map(|i| {
            let c = content(&dir::names(&mut fs.membri[i])?)?;
            Some((i, c))
        })
        .or_else(|| (0..fs.membri.len()).find_map(|i| Some((i, other(&mut fs.membri[i])?))))
}

/// A member named `.qgd`/`.lcd` that starts as an OLE2 container: the storages of its root (`Ignoto`), or `Rotto` when its directory cannot be followed.
fn other(m: &mut mzlab_core::readers::Member) -> Option<Content> {
    let n = m.nome.to_ascii_lowercase();
    if !(n.ends_with(".qgd") || n.ends_with(".lcd")) || !dir::ole(m) {
        return None;
    }
    Some(match dir::names(m) {
        Some(v) => Content::Ignoto(
            v.iter()
                .skip(1)
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        ),
        None => Content::Broken,
    })
}

fn recognize(fs: &mut FileSet) -> Fiducia {
    match find(fs) {
        Some((_, Content::Data | Content::QTof)) => Fiducia::Si,
        Some((_, Content::LcOnly | Content::Ignoto(_) | Content::Broken)) => Fiducia::Forse,
        None => Fiducia::No,
    }
}

fn refusal(detail: &str) -> Error {
    Error::new("err.file.shimadzu").with("detail", detail)
}

/// A copy cut before its directory (its header points past the end, `Ole::header_cut`): the same notice as a cut found after `open`.
fn cut_before(m: &mut mzlab_core::readers::Member) -> Option<Error> {
    let mut h = vec![0u8; m.dim.min(512) as usize];
    m.dati.read_at(0, &mut h).ok()?;
    let need = Ole::header_cut(&h).filter(|&n| n > m.dim)?;
    Some(
        Error::new("err.file.cutShort")
            .with("name", &m.nome)
            .with("size", m.dim)
            .with("needed", need),
    )
}

fn apri(mut fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    let (i, c) = find(&mut fs).ok_or_else(|| Error::new("err.file.notAccepted"))?;
    match &c {
        Content::Ignoto(v) => {
            return Err(refusal(&format!(
                "not a LabSolutions/GCMSsolution data file (storages: {v})"
            )))
        }
        Content::Broken => {
            let m = &mut fs.membri[i];
            if let Some(e) = cut_before(m) {
                return Err(e);
            }
            let d = m.dim;
            return Err(refusal(&format!("the file is cut short or damaged: its directory lies beyond its {d} bytes; copy the whole file again")));
        }
        _ => {}
    }
    let mut m = fs.membri.swap_remove(i);
    // the container and the raw stream are both in memory
    if m.dim.saturating_mul(2) > limits.byte {
        return Err(Error::memory_budget());
    }
    let mut data = Vec::new();
    data.try_reserve_exact(m.dim as usize)
        .map_err(|_| Error::memory_budget())?;
    data.resize(m.dim as usize, 0);
    m.dati.read_at(0, &mut data)?;
    let ole = Ole::open(data, &m.nome)?;
    // sectors in use past the end: the raw stream would read short and the file would open with fewer scans, or none, unsaid
    if let Some(need) = ole.cut_short() {
        return Err(Error::new("err.file.cutShort")
            .with("name", &m.nome)
            .with("size", m.dim)
            .with("needed", need));
    }
    let Some(fam) = load::choose(&ole) else {
        return Err(refusal(match c {
            Content::QTof => "Q-TOF data (QTFL RawData) are not read: their m/z could not be checked against a reference",
            _ => "the file holds no mass spectrometer data",
        }));
    };
    let d = load::load(&ole, fam, limits)?;
    let mut meta = meta::collect(&ole, fam);
    if d.unreadable > 0 {
        meta.warnings.push(format!(
            "{} of {} records not decoded",
            d.unreadable, d.records
        ));
    }
    // the file name without its folder, as the other readers keep it
    let name = m
        .nome
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&m.nome)
        .to_string();
    Ok(Box::new(ShimadzuReader {
        samples: vec![Sample {
            id: 0,
            nome: name.clone(),
            scansioni: d.indice.len(),
            size: None,
        }],
        sha1: ole.sha1_hex(),
        nome: name,
        dim: m.dim,
        fam,
        d,
        meta,
    }))
}

pub struct ShimadzuReader {
    nome: String,
    dim: u64,
    sha1: String,
    fam: Fam,
    samples: Vec<Sample>,
    d: Data,
    meta: Meta,
}

impl ShimadzuReader {
    fn one(&self, c: usize) -> Result<()> {
        (c == 0)
            .then_some(())
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    fn id(&self) -> (&str, u64, &str) {
        (&self.nome, self.dim, &self.sha1)
    }
}
