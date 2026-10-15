//! The family `shimadzu`: LabSolutions / GCMSsolution files, OLE2 containers read with `mzlab_core::ole`. `.qgd` (GC/MS, `GCMS Raw Data`) and `.lcd` of a single quadrupole (`Mass Raw Data`:
//! scans and SIM) and of a triple quadrupole (`TLM Raw Data`: MRM as SRM chromatograms). The Q-TOF `.lcd` (`QTFL RawData`) is recognised and refused with `err.file.shimadzu`.
//! Written from the files themselves (stream layout in docs/agenti/shimadzu.md); the Apache-2.0 crate `openszraw` was read to learn where the streams are, not copied.

mod carica;
mod dir;
mod lettore;
mod meta;
mod mrm;
mod prop;
mod rec;

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{Campione, Fiducia, FileSet, Lettore, Limiti, VoceLettore, CONTRATTO};
use mzlab_core::ole::Ole;

use carica::{Dati, Fam};
use meta::Meta;

/// Above this intensity an `f32` loses counts: the spectrum then keeps the exact values.
const F32_EXACT: u64 = 1 << 24;

pub const VOCE: VoceLettore = VoceLettore {
    id: "shimadzu",
    contratto: CONTRATTO,
    estensioni: &[".qgd", ".lcd"],
    riconosci,
    apri,
};

/// What a container holds, from the names of its directory (a storage can be there and empty: the data are told by `carica::scegli`).
enum Contenuto {
    Dati,
    QTof,
    SoloLc,
}

fn contenuto(nomi: &[String]) -> Option<Contenuto> {
    let ha = |n: &str| nomi.iter().any(|x| x == n);
    if carica::famiglie().iter().any(|f| ha(f.storage())) {
        Some(Contenuto::Dati)
    } else if ha("QTFL RawData") {
        Some(Contenuto::QTof)
    } else if ha("LSS Raw Data") {
        Some(Contenuto::SoloLc)
    } else {
        None
    }
}

/// The member of `fs` that is a Shimadzu container, and what it holds.
fn trova(fs: &mut FileSet) -> Option<(usize, Contenuto)> {
    (0..fs.membri.len()).find_map(|i| {
        let c = contenuto(&dir::nomi(&mut fs.membri[i])?)?;
        Some((i, c))
    })
}

fn riconosci(fs: &mut FileSet) -> Fiducia {
    match trova(fs) {
        Some((_, Contenuto::Dati | Contenuto::QTof)) => Fiducia::Si,
        Some((_, Contenuto::SoloLc)) => Fiducia::Forse,
        None => Fiducia::No,
    }
}

fn rifiuto(detail: &str) -> Error {
    Error::new("err.file.shimadzu").with("detail", detail)
}

fn apri(mut fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    let (i, c) = trova(&mut fs).ok_or_else(|| Error::new("err.file.notAccepted"))?;
    let mut m = fs.membri.swap_remove(i);
    // the container and the raw stream are both in memory
    if m.dim.saturating_mul(2) > limiti.byte {
        return Err(Error::memory_budget());
    }
    let mut data = Vec::new();
    data.try_reserve_exact(m.dim as usize)
        .map_err(|_| Error::memory_budget())?;
    data.resize(m.dim as usize, 0);
    m.dati.read_at(0, &mut data)?;
    let ole = Ole::open(data, &m.nome)?;
    let Some(fam) = carica::scegli(&ole) else {
        return Err(rifiuto(match c {
            Contenuto::QTof => "Q-TOF data (QTFL RawData) are not read: their m/z could not be checked against a reference",
            _ => "the file holds no mass spectrometer data",
        }));
    };
    let d = carica::carica(&ole, fam, limiti)?;
    let mut meta = meta::raccogli(&ole, fam);
    if d.illeggibili > 0 {
        meta.avvisi.push(format!(
            "{} of {} records not decoded",
            d.illeggibili, d.records
        ));
    }
    Ok(Box::new(LettoreShimadzu {
        campioni: vec![Campione {
            id: 0,
            nome: m.nome.clone(),
            scansioni: d.indice.len(),
        }],
        sha1: ole.sha1_hex(),
        nome: m.nome,
        dim: m.dim,
        fam,
        d,
        meta,
    }))
}

pub struct LettoreShimadzu {
    nome: String,
    dim: u64,
    sha1: String,
    fam: Fam,
    campioni: Vec<Campione>,
    d: Dati,
    meta: Meta,
}

impl LettoreShimadzu {
    fn uno(&self, c: usize) -> Result<()> {
        (c == 0)
            .then_some(())
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }

    fn id(&self) -> (&str, u64, &str) {
        (&self.nome, self.dim, &self.sha1)
    }
}
