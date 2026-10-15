//! The readers of the vendor files, one module `.wasm` per family of formats (docs/agenti/lettori-modulo.md): the engine (`mzlab-wasm`) carries none of this crate.
//! The readers of a family sit in `src/<family>/` behind the Cargo feature of the same name and implement `mzlab_core::lettori::Lettore`; variants of the readers that live
//! in `mzlab-core` sit behind its `estesi` feature, which this crate turns on and the engine leaves off.
use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{self, Fiducia, FileSet, Lettore, Limiti, VoceLettore};

pub mod srm;

// The readers of each family, one `static` per `VoceLettore` of the registry; a family without readers yet is born with its own package.
#[cfg(any(feature = "agilent", feature = "tutte"))]
static VOCE_AGILENT: VoceLettore = mzlab_core::agilent::lettore::VOCE;
#[cfg(any(feature = "sciex", feature = "tutte"))]
static VOCE_SCIEX: VoceLettore = mzlab_core::wiff::lettore::VOCE;
#[cfg(any(feature = "thermo", feature = "tutte"))]
static VOCE_THERMO: VoceLettore = mzlab_core::rawlettore::VOCE;

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
pub fn registro() -> Vec<&'static VoceLettore> {
    let voci: &[Option<&'static VoceLettore>] = &[
        #[cfg(any(feature = "agilent", feature = "tutte"))]
        Some(&VOCE_AGILENT),
        #[cfg(any(feature = "sciex", feature = "tutte"))]
        Some(&VOCE_SCIEX),
        #[cfg(any(feature = "thermo", feature = "tutte"))]
        Some(&VOCE_THERMO),
    ];
    voci.iter().flatten().copied().collect()
}

/// The reader of `registro` that is surest of `fs` (the first among equals; like `lettori::riconosci_in`, which takes a slice and not this list).
pub fn riconosci(fs: &mut FileSet) -> Option<&'static VoceLettore> {
    let mut best: Option<(Fiducia, &'static VoceLettore)> = None;
    for v in registro()
        .into_iter()
        .filter(|v| v.contratto == lettori::CONTRATTO)
    {
        let f = (v.riconosci)(fs);
        if f > Fiducia::No && best.is_none_or(|(b, _)| f > b) {
            best = Some((f, v));
        }
    }
    best.map(|(_, v)| v)
}

/// Opens `fs` with the reader that recognises it, as it is (the sessions of the modules: the engine decides itself what it does with SRM scans).
pub fn apri_nudo(mut fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    let name = fs
        .membri
        .first()
        .map(|m| m.nome.clone())
        .unwrap_or_default();
    let v =
        riconosci(&mut fs).ok_or_else(|| Error::new("err.file.notAccepted").with("name", name))?;
    (v.apri)(fs, limiti)
}

/// Opens `fs` with the reader that recognises it; the SRM traces of scans with an SRM filter become chromatograms (`srm::SrmDaScansioni`).
pub fn apri(fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    Ok(Box::new(srm::SrmDaScansioni::new(apri_nudo(fs, limiti)?)))
}

/// Every extension of the registry of this build, without repeats.
pub fn estensioni() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for e in registro().iter().flat_map(|v| v.estensioni.iter().copied()) {
        if !out.contains(&e) {
            out.push(e);
        }
    }
    out
}

/// Why the engine does not do for sample `c` of `l`: `"srm"` when its SRM traces exist only as scans.
pub fn motivi(l: &mut dyn Lettore, c: usize) -> Vec<&'static str> {
    let mut m = Vec::new();
    if srm::ha_scansioni_srm(l, c) {
        m.push("srm");
    }
    m
}

/// Why the engine does not do for the dataset `fs` (before it is opened): `"variante"` when it is an Agilent `.D` of a variant the engine's reader cannot read the
/// points of (QqQ MRM, Q-TOF profile or time-of-flight spectra, compressed spectra), so that the page does not try the engine first.
#[cfg_attr(not(any(feature = "agilent", feature = "tutte")), allow(unused_mut, unused_variables))]
pub fn motivi_insieme(fs: &mut FileSet) -> Vec<&'static str> {
    let mut m = Vec::new();
    #[cfg(any(feature = "agilent", feature = "tutte"))]
    if mzlab_core::agilent::lettore::variante(fs) {
        m.push("variante");
    }
    let _ = &fs;
    m
}
