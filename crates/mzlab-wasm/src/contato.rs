//! The reader of a dataset that counts the scans it decodes for the page (`build_mzidx_native` in `nativi.rs`).
use js_sys::Function;
use mzlab_core::lettori::{self, Lettore};
use wasm_bindgen::JsValue;

/// The reader of a dataset that tells `step(done, total)` how many scans of sample `c` it has decoded, about once per 1 % of them: the page shows how far the index
/// of the sample has got by its own scans, not by the bytes of the members (a `.wiff.scan` holds every sample, the open reads the heads of all of them).
/// A later pass of the builder counts on from where the first one ended; the page counts from the start of each phase.
struct Contato {
    l: Box<dyn Lettore>,
    step: Function,
    c: usize,
    n: u32,
    fatte: u32,
    detto: u32,
}

impl Contato {
    fn nuovo(mut l: Box<dyn Lettore>, step: Function, c: usize) -> Contato {
        let n = l.indice(c).map_or(0, |v| v.len() as u32);
        drop(step.call2(&JsValue::NULL, &0.into(), &n.into()));
        Contato {
            l,
            step,
            c,
            n,
            fatte: 0,
            detto: 0,
        }
    }
    fn conta(&mut self, c: usize) {
        if c != self.c {
            return;
        }
        self.fatte += 1;
        if self.fatte - self.detto >= (self.n / 100).max(1) {
            self.detto = self.fatte;
            drop(
                self.step
                    .call2(&JsValue::NULL, &self.fatte.into(), &self.n.into()),
            );
        }
    }
}

impl Lettore for Contato {
    fn formato(&self) -> lettori::Formato {
        self.l.formato()
    }
    fn campioni(&self) -> &[lettori::Campione] {
        self.l.campioni()
    }
    fn indice(&mut self, c: usize) -> mzlab_core::Result<&[lettori::VoceScansione]> {
        self.l.indice(c)
    }
    fn scansione(&mut self, c: usize, k: usize) -> mzlab_core::Result<mzlab_core::mzml::Spectrum> {
        let s = self.l.scansione(c, k);
        self.conta(c);
        s
    }
    fn scansione_dati(
        &mut self,
        c: usize,
        k: usize,
    ) -> mzlab_core::Result<mzlab_core::mzml::Spectrum> {
        let s = self.l.scansione_dati(c, k);
        self.conta(c);
        s
    }
    fn intestazione(&mut self, c: usize, k: usize) -> mzlab_core::Result<Vec<lettori::Campo>> {
        self.l.intestazione(c, k)
    }
    fn cromatogrammi(&mut self, c: usize) -> mzlab_core::Result<Vec<lettori::VoceCromatogramma>> {
        self.l.cromatogrammi(c)
    }
    fn cromatogramma(
        &mut self,
        c: usize,
        id: &str,
    ) -> mzlab_core::Result<mzlab_core::mzml::Chromatogram> {
        self.l.cromatogramma(c, id)
    }
    fn metodo(&mut self, c: usize) -> mzlab_core::Result<String> {
        self.l.metodo(c)
    }
    fn testata_mzml(
        &mut self,
        c: usize,
    ) -> mzlab_core::Result<Option<mzlab_core::mzmlwrite::Header>> {
        self.l.testata_mzml(c)
    }
    fn extra_mzml(&self, s: &mzlab_core::mzml::Spectrum) -> mzlab_core::mzmlwrite::SpectrumExtra {
        self.l.extra_mzml(s)
    }
}

/// The reader of the dataset for the index of sample `c`: phase 4 (`tell`) begins, `apri` opens it, and it counts its scans for `step` when the page gave one.
pub(crate) fn apri(
    apri: impl FnOnce() -> mzlab_core::Result<Box<dyn Lettore>>,
    tell: &dyn Fn(u32),
    step: Option<Function>,
    c: usize,
) -> Result<Box<dyn Lettore>, JsValue> {
    tell(4);
    let l = apri().map_err(|e| crate::error_json(&e))?;
    Ok(match step {
        Some(step) => Box::new(Contato::nuovo(l, step, c)),
        None => l,
    })
}
