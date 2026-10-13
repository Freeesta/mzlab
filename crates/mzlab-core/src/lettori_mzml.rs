//! The one mzML writer on top of `Lettore`: any reader of the registry gives the same file layout (spectra, then chromatograms).
use std::io::Write;

use crate::error::{Error, Result};
use crate::lettori::Lettore;
use crate::mzmlwrite::{Header, MzmlWriter};

/// The header when the reader gives none: the run is the sample, the program is mzLab.
fn semplice(l: &dyn Lettore, c: usize) -> Header {
    let nome = l
        .campioni()
        .get(c)
        .map(|s| s.nome.clone())
        .unwrap_or_default();
    Header {
        run: nome.clone(),
        sources: vec![nome],
        software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
        ..Header::default()
    }
}

/// Writes sample `c` of `l` as mzML to `out`; `progress(done, total)` counts the spectra.
pub fn scrivi<W: Write>(
    l: &mut dyn Lettore,
    c: usize,
    out: W,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<W> {
    if c >= l.campioni().len() {
        return Err(Error::new("error.badIndex").with("c", c));
    }
    let testata = match l.testata_mzml(c)? {
        Some(h) => h,
        None => semplice(l, c),
    };
    let mut w = MzmlWriter::new(out, &testata)?;
    let n = l.indice(c)?.len();
    if n > 0 {
        w.open_spectra(n)?;
    }
    progress(0, n);
    for k in 0..n {
        let s = l.scansione(c, k)?;
        w.spectrum_with(&s, &l.extra_mzml(&s))?;
        if (k + 1) % 32 == 0 {
            progress(k + 1, n);
        }
    }
    progress(n, n);
    if n > 0 {
        w.close_spectra()?;
    }
    let mut tracce = Vec::new();
    for v in l.cromatogrammi(c)? {
        let mut t = l.cromatogramma(c, &v.id)?;
        if !t.time.is_empty() {
            t.index = tracce.len();
            tracce.push(t);
        }
    }
    if !tracce.is_empty() {
        w.open_chromatograms(tracce.len())?;
        for t in &tracce {
            w.chromatogram(t)?;
        }
        w.close_chromatograms()?;
    }
    w.finish()
}
