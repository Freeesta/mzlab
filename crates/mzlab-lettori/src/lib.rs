//! The readers module: formats and variants that the engine (`mzlab-wasm`) does not carry. A new format is a module here that implements
//! `mzlab_core::lettori::Lettore`, its `VOCE` and a line in `LETTORI_EXTRA`; variants of the readers that live in `mzlab-core` sit behind
//! its `estesi` feature, which this crate turns on and the engine leaves off (`docs/agenti/lettori-modulo.md`).
use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{self, Fiducia, FileSet, Lettore, Limiti, VoceLettore};

pub mod srm;

/// The formats of the module, in order of preference among equals.
pub static LETTORI_EXTRA: &[VoceLettore] = &[];

/// `LETTORI_EXTRA` followed by the registry of the engine (the module also serves the variants of the engine's readers).
pub fn registro() -> Vec<&'static VoceLettore> {
    LETTORI_EXTRA
        .iter()
        .chain(lettori::LETTORI.iter())
        .collect()
}

/// The reader of `registro` that is surest of `fs` (the first among equals).
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

/// True when the readers of the module (and not only the engine's) recognise `fs`.
pub fn riconosciuto_dal_modulo(fs: &mut FileSet) -> bool {
    lettori::riconosci_in(LETTORI_EXTRA, fs).is_some()
}

/// Opens `fs` with the reader that recognises it; the SRM traces of scans with an SRM filter become chromatograms (`srm::SrmDaScansioni`).
pub fn apri(mut fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    let name = fs
        .membri
        .first()
        .map(|m| m.nome.clone())
        .unwrap_or_default();
    let v =
        riconosci(&mut fs).ok_or_else(|| Error::new("err.file.notAccepted").with("name", name))?;
    Ok(Box::new(srm::SrmDaScansioni::new((v.apri)(fs, limiti)?)))
}

/// Every extension the module accepts (its own formats and the engine's), without repeats.
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
