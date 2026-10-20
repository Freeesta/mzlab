//! The one mzML writer on top of `Lettore`: any reader of the registry gives the same file layout (spectra, then chromatograms).
use std::io::Write;

use crate::error::{Error, Result};
use crate::lettori::Lettore;
use crate::mzmlwrite::{Header, MzmlWriter, Opzioni, ProcStep};

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

/// The header `scrivi` writes for sample `c`.
pub(crate) fn testata(l: &mut dyn Lettore, c: usize) -> Result<Header> {
    Ok(match l.testata_mzml(c)? {
        Some(h) => h,
        None => semplice(l, c),
    })
}

/// Writes sample `c` of `l` as mzML to `out`; `progress(done, total)` counts the spectra.
pub fn scrivi<W: Write>(
    l: &mut dyn Lettore,
    c: usize,
    out: W,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<W> {
    scrivi_con(l, c, out, progress, Opzioni::default())
}

/// `scrivi` with the zlib level and the numpress choice of `opz` (`mzmlwrite::Opzioni`).
pub fn scrivi_con<W: Write>(
    l: &mut dyn Lettore,
    c: usize,
    out: W,
    progress: &mut dyn FnMut(usize, usize),
    opz: Opzioni,
) -> Result<W> {
    scrivi_in(l, c, out, progress, opz, None)
}

/// The mzML of the `convert.mzml` step: `indexedmzML` (PSI-MS 4.2.6, offsets, SHA-1 checksum) with one `processingMethod` per entry of `steps`
/// after the conversion. The spectra and chromatograms are those of `scrivi_con`.
#[cfg(not(target_family = "wasm"))]
pub fn scrivi_indicizzato<W: Write>(
    l: &mut dyn Lettore,
    c: usize,
    out: W,
    progress: &mut dyn FnMut(usize, usize),
    opz: Opzioni,
    steps: &[ProcStep],
) -> Result<W> {
    scrivi_in(l, c, out, progress, opz, Some(steps))
}

fn scrivi_in<W: Write>(
    l: &mut dyn Lettore,
    c: usize,
    out: W,
    progress: &mut dyn FnMut(usize, usize),
    opz: Opzioni,
    indexed: Option<&[ProcStep]>,
) -> Result<W> {
    if c >= l.campioni().len() {
        return Err(Error::new("error.badIndex").with("c", c));
    }
    let mut h = testata(l, c)?;
    let mut w = match indexed {
        Some(steps) => {
            h.steps = steps.to_vec();
            MzmlWriter::indicizzato(out, &h, opz)?
        }
        None => MzmlWriter::con_opzioni(out, &h, opz)?,
    };
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
    cromatogrammi(l, c, &mut w)?;
    w.finish()
}

/// The chromatograms of sample `c` that hold points, numbered again from 0 as `scrivi` writes them.
fn cromatogrammi<W: Write>(l: &mut dyn Lettore, c: usize, w: &mut MzmlWriter<W>) -> Result<()> {
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
    Ok(())
}

/// The mzML of sample `c` without its spectra: the header and the chromatograms exactly as `scrivi` writes them, then a comment that names the
/// polarities of the scans (what a whole mzML says of them where `mzmlhead::Mzml::header_polarity` looks). What the routes that read the header
/// and the chromatograms of a file (`mzmlhead::Mzml`) read for a vendor file; its scans come from the reader (`ReadAt::lettore`).
pub fn guscio(l: &mut dyn Lettore, c: usize) -> Result<Vec<u8>> {
    if c >= l.campioni().len() {
        return Err(Error::new("error.badIndex").with("c", c));
    }
    let mut w = MzmlWriter::new(Vec::new(), &testata(l, c)?)?;
    let n = l.indice(c)?.len();
    if n > 0 {
        w.open_spectra(n)?;
        w.close_spectra()?;
    }
    cromatogrammi(l, c, &mut w)?;
    let (pos, neg) = l.indice(c)?.iter().fold((false, false), |(p, q), v| {
        (p || v.polarita > 0, q || v.polarita < 0)
    });
    let mut doc = w.finish()?;
    let tail = b"</run>\n</mzML>\n".len();
    let end = doc.len().saturating_sub(tail);
    let mut note = String::new();
    if pos || neg {
        note += "<!-- scans:";
        if pos {
            note += " accession=\"MS:1000130\"";
        }
        if neg {
            note += " accession=\"MS:1000129\"";
        }
        note += " -->\n";
    }
    doc.splice(end..end, note.into_bytes());
    Ok(doc)
}
