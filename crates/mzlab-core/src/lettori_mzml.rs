//! The one mzML writer on top of `Lettore`: any reader of the registry gives the same file layout (spectra, then chromatograms).
use std::io::Write;

use crate::centroidi::{Buf, Params};
use crate::error::{Error, Result};
use crate::lettori::Lettore;
#[cfg(not(target_family = "wasm"))]
use crate::lettori_centroidi::{picchi, stima};
use crate::mzml::Spectrum;
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

/// Which points of a scan the file gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Picchi {
    /// The scans as the reader gives them (the centroids of the maker where it has them).
    #[default]
    File,
    /// The profile points of the file (`Lettore::scansione_profilo`, else the scans of the reader that are profile); a file without them refuses (`err.conv.noProfile`).
    Profilo,
    /// Our own centroids (`lettori_centroidi`): the profile scans are reduced to one point per peak with parameters estimated on the sample, a scan
    /// already centroided stays as it is; the file says so (`MS:1000035`, the parameters in a userParam of the run).
    Mzlab,
}

/// The parameters of our own centroids when the first scan of sample `c` is profile; the header then says so (`MS:1000035`, the parameters in a userParam).
#[cfg(not(target_family = "wasm"))]
fn centroidi_di(l: &mut dyn Lettore, c: usize, h: &mut Header) -> Result<Option<Params>> {
    if l.indice(c)?.is_empty() || !l.scansione(c, 0)?.profile {
        return Ok(None);
    }
    let p = stima(l, c)?;
    h.centroided = true;
    h.run_params.push((
        "mzLab centroids".into(),
        format!(
            "mode={:?}; unit={}; step={}; w={}; k={}",
            p.mode, p.unit, p.step, p.w, p.k
        ),
    ));
    Ok(Some(p))
}

/// The parameters of our own centroids, when `peaks` asks for them and the sample is profile (the browser build has no peak picking here).
#[cfg(not(target_family = "wasm"))]
fn mzlab_params(
    peaks: Picchi,
    l: &mut dyn Lettore,
    c: usize,
    h: &mut Header,
) -> Result<Option<Params>> {
    match peaks {
        Picchi::Mzlab => centroidi_di(l, c, h),
        _ => Ok(None),
    }
}

#[cfg(target_family = "wasm")]
fn mzlab_params(
    _: Picchi,
    _: &mut dyn Lettore,
    _: usize,
    _: &mut Header,
) -> Result<Option<Params>> {
    Ok(None)
}

/// Scan `(c, k)` as `peaks` says; the browser build has no peak picking and no profile of a reader that does not keep one apart.
#[cfg(not(target_family = "wasm"))]
fn scansione_di(
    peaks: Picchi,
    l: &mut dyn Lettore,
    (c, k): (usize, usize),
    (p, buf): (Option<&Params>, &mut Buf),
) -> Result<Spectrum> {
    match peaks {
        Picchi::File => l.scansione(c, k),
        Picchi::Mzlab => {
            let mut s = l.scansione(c, k)?;
            if let Some(p) = p {
                picchi(&mut s, p, buf);
            }
            Ok(s)
        }
        Picchi::Profilo => match l.scansione_profilo(c, k)? {
            Some(s) => Ok(s),
            // a reader whose scans are the file's own (mzML) has the profile in them
            None => Some(l.scansione(c, k)?)
                .filter(|s| s.profile)
                .ok_or_else(|| Error::new("err.conv.noProfile")),
        },
    }
}

#[cfg(target_family = "wasm")]
fn scansione_di(
    peaks: Picchi,
    l: &mut dyn Lettore,
    (c, k): (usize, usize),
    _: (Option<&Params>, &mut Buf),
) -> Result<Spectrum> {
    match peaks {
        Picchi::Profilo => l
            .scansione_profilo(c, k)?
            .ok_or_else(|| Error::new("err.conv.noProfile")),
        _ => l.scansione(c, k),
    }
}

/// A conversion of one sample to mzML: the one entry of every route (`scrivi*` are its short forms).
#[derive(Debug, Clone, Copy, Default)]
pub struct Conversione<'a> {
    pub peaks: Picchi,
    pub opz: Opzioni,
    /// `Some`: the `indexedmzML` form with one `processingMethod` per step after the conversion.
    pub steps: Option<&'a [ProcStep]>,
}

impl Conversione<'_> {
    /// Writes sample `c` of `l` to `out`; `progress(done, total)` counts the spectra.
    pub fn scrivi<W: Write>(
        &self,
        l: &mut dyn Lettore,
        c: usize,
        out: W,
        progress: &mut dyn FnMut(usize, usize),
    ) -> Result<W> {
        if c >= l.campioni().len() {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        let mut h = testata(l, c)?;
        let params = mzlab_params(self.peaks, l, c, &mut h)?;
        let mut buf = Buf::default();
        h.run_params
            .push(("mzLab conv/1".into(), crate::conv::resoconto(l, c)?));
        let mut w = match self.steps {
            Some(steps) => {
                h.steps = steps.to_vec();
                MzmlWriter::indicizzato(out, &h, self.opz)?
            }
            None => MzmlWriter::con_opzioni(out, &h, self.opz)?,
        };
        let n = l.indice(c)?.len();
        if n > 0 {
            w.open_spectra(n)?;
        }
        progress(0, n);
        for k in 0..n {
            let s = scansione_di(self.peaks, l, (c, k), (params.as_ref(), &mut buf))?;
            let extra = l.extra_scansione(c, k, &s);
            w.spectrum_with(&s, &extra)?;
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
}

/// Writes sample `c` of `l` as mzML to `out`; `progress(done, total)` counts the spectra.
pub fn scrivi<W: Write>(
    l: &mut dyn Lettore,
    c: usize,
    out: W,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<W> {
    Conversione::default().scrivi(l, c, out, progress)
}

/// `scrivi` with the zlib level and the numpress choice of `opz` (`mzmlwrite::Opzioni`).
pub fn scrivi_con<W: Write>(
    l: &mut dyn Lettore,
    c: usize,
    out: W,
    progress: &mut dyn FnMut(usize, usize),
    opz: Opzioni,
) -> Result<W> {
    let conv = Conversione {
        opz,
        ..Conversione::default()
    };
    conv.scrivi(l, c, out, progress)
}

/// The mzML of the `convert.mzml` step: `indexedmzML` (PSI-MS 4.2.6, offsets, SHA-1 checksum) with one `processingMethod` per entry of `steps`
/// after the conversion. The spectra and chromatograms are those of `scrivi_con`.
pub fn scrivi_indicizzato<W: Write>(
    l: &mut dyn Lettore,
    c: usize,
    out: W,
    progress: &mut dyn FnMut(usize, usize),
    opz: Opzioni,
    steps: &[ProcStep],
) -> Result<W> {
    let conv = Conversione {
        opz,
        steps: Some(steps),
        ..Conversione::default()
    };
    conv.scrivi(l, c, out, progress)
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
