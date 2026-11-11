//! The one mzML writer on top of `Lettore`: any reader of the registry gives the same file layout (spectra, then chromatograms).
use std::io::Write;

#[cfg(not(target_family = "wasm"))]
use crate::centroids::{Buf, Params};
use crate::error::{Error, Result};
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter, Options, ProcStep};
use crate::readers::Reader;
#[cfg(not(target_family = "wasm"))]
use crate::readers_centroids::{picchi, stima};

/// The header when the reader gives none: the run is the sample, the program is mzLab.
fn simple(l: &dyn Reader, c: usize) -> Header {
    let name = l
        .samples()
        .get(c)
        .map(|s| s.nome.clone())
        .unwrap_or_default();
    Header {
        run: name.clone(),
        sources: vec![name],
        software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
        ..Header::default()
    }
}

/// The header `scrivi` writes for sample `c`.
pub(crate) fn file_header(l: &mut dyn Reader, c: usize) -> Result<Header> {
    Ok(match l.mzml_header(c)? {
        Some(h) => h,
        None => simple(l, c),
    })
}

/// The header the scan table reads (`Lettore::table_header`).
pub(crate) fn table_header(l: &mut dyn Reader, c: usize) -> Result<Header> {
    Ok(match l.table_header(c)? {
        Some(h) => h,
        None => simple(l, c),
    })
}

/// Which points of a scan the file gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Peaks {
    /// The scans as the reader gives them (the centroids of the maker where it has them).
    #[default]
    File,
    /// The profile points of the file (`Lettore::scan_profile`, else the scans of the reader that are profile); a file without them refuses (`err.conv.noProfile`).
    Profilo,
    /// Our own centroids (`lettori_centroidi`): the profile scans are reduced to one point per peak with parameters estimated on the sample, a scan
    /// already centroided stays as it is; the file says so (`MS:1000035`, the parameters in a userParam of the run).
    Mzlab,
}

/// The parameters of our own centroids when the first scan of sample `c` is profile; the header then says so (`MS:1000035`, the parameters in a userParam,
/// with `w_basis` = `measured` on N isolated peaks or `fallback`: a fallback width is never written as measured).
#[cfg(not(target_family = "wasm"))]
fn centroids_of(l: &mut dyn Reader, c: usize, h: &mut Header) -> Result<Option<Params>> {
    if l.index_without_tic(c)?.is_empty() || !l.scan(c, 0)?.profile {
        return Ok(None);
    }
    let e = stima(l, c)?;
    let p = e.params;
    let basis = if e.peaks > 0 {
        format!("measured on {} isolated peaks", e.peaks)
    } else {
        "fallback".to_string()
    };
    h.centroided = true;
    h.run_params.push((
        "mzLab centroids".into(),
        format!(
            "mode={:?}; unit={}; step={}; w={}; w_basis={basis}; k={}",
            p.mode, p.unit, p.step, p.w, p.k
        ),
    ));
    Ok(Some(p))
}

/// What the peak picking of our own centroids needs while the scans go by (nothing in the browser build, which has none).
#[cfg(not(target_family = "wasm"))]
struct Pick(Option<Params>, Buf);
#[cfg(target_family = "wasm")]
struct Pick;

/// The `Pick` of a conversion: the parameters only when `peaks` asks for our centroids and the sample is profile.
#[cfg(not(target_family = "wasm"))]
fn pick_di(peaks: Peaks, l: &mut dyn Reader, c: usize, h: &mut Header) -> Result<Pick> {
    let p = match peaks {
        Peaks::Mzlab => centroids_of(l, c, h)?,
        _ => None,
    };
    Ok(Pick(p, Buf::default()))
}

#[cfg(target_family = "wasm")]
fn pick_di(_: Peaks, _: &mut dyn Reader, _: usize, _: &mut Header) -> Result<Pick> {
    Ok(Pick)
}

/// Scan `(c, k)` as `peaks` says; the browser build has no peak picking and no profile of a reader that does not keep one apart.
#[cfg(not(target_family = "wasm"))]
fn scansione_di(
    peaks: Peaks,
    l: &mut dyn Reader,
    (c, k): (usize, usize),
    pick: &mut Pick,
) -> Result<Spectrum> {
    match peaks {
        Peaks::File => l.scan(c, k),
        Peaks::Mzlab => {
            let mut s = l.scan(c, k)?;
            if let Some(p) = &pick.0 {
                picchi(&mut s, p, &mut pick.1);
            }
            Ok(s)
        }
        Peaks::Profilo => match l.scan_profile(c, k)? {
            Some(s) => Ok(s),
            // a reader whose scans are the file's own (mzML) has the profile in them
            None => Some(l.scan(c, k)?)
                .filter(|s| s.profile)
                .ok_or_else(|| Error::new("err.conv.noProfile")),
        },
    }
}

#[cfg(target_family = "wasm")]
fn scansione_di(
    peaks: Peaks,
    l: &mut dyn Reader,
    (c, k): (usize, usize),
    _: &mut Pick,
) -> Result<Spectrum> {
    match peaks {
        Peaks::Profilo => l
            .scan_profile(c, k)?
            .ok_or_else(|| Error::new("err.conv.noProfile")),
        _ => l.scan(c, k),
    }
}

/// The optical matrix of sample `c` when `opz.dad` asks for it (in the browser only the `sciex` readers module writes it; the engine has none).
#[cfg(any(not(target_family = "wasm"), feature = "sciex"))]
fn dad_di(opz: Options, l: &mut dyn Reader, c: usize) -> Result<Option<crate::wiff::dad::DadData>> {
    if opz.dad {
        l.dad_data(c)
    } else {
        Ok(None)
    }
}

#[cfg(all(target_family = "wasm", not(feature = "sciex")))]
fn dad_di(_: Options, _: &mut dyn Reader, _: usize) -> Result<Option<crate::wiff::dad::DadData>> {
    Ok(None)
}

/// The rows of the optical matrix as `MS:1000804` spectra, after the scans.
#[cfg(any(not(target_family = "wasm"), feature = "sciex"))]
fn uv_spettri<W: Write>(
    w: &mut MzmlWriter<W>,
    dad: Option<&crate::wiff::dad::DadData>,
) -> Result<()> {
    if let Some(d) = dad {
        for (i, &t) in d.time_min.iter().enumerate() {
            w.uv_spectrum(i, t, &d.wavelengths_nm, d.spectrum_at(i).unwrap_or(&[]))?;
        }
    }
    Ok(())
}

#[cfg(all(target_family = "wasm", not(feature = "sciex")))]
fn uv_spettri<W: Write>(
    _: &mut MzmlWriter<W>,
    _: Option<&crate::wiff::dad::DadData>,
) -> Result<()> {
    Ok(())
}

/// A conversion of one sample to mzML: the one entry of every route (`scrivi*` are its short forms).
#[derive(Debug, Clone, Copy, Default)]
pub struct Conversion<'a> {
    pub peaks: Peaks,
    pub opz: Options,
    /// `Some`: the `indexedmzML` form with one `processingMethod` per step after the conversion.
    pub steps: Option<&'a [ProcStep]>,
}

impl Conversion<'_> {
    /// Writes sample `c` of `l` to `out`; `progress(done, total)` counts the spectra.
    pub fn write<W: Write>(
        &self,
        l: &mut dyn Reader,
        c: usize,
        out: W,
        progress: &mut dyn FnMut(usize, usize),
    ) -> Result<W> {
        if c >= l.samples().len() {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        let mut h = file_header(l, c)?;
        let mut pick = pick_di(self.peaks, l, c, &mut h)?;
        h.run_params
            .push(("mzLab conv/1".into(), crate::conv::resoconto(l, c)?));
        let mut w = match self.steps {
            Some(steps) => {
                h.steps = steps.to_vec();
                MzmlWriter::indicizzato(out, &h, self.opz)?
            }
            None => MzmlWriter::with_options(out, &h, self.opz)?,
        };
        let n = l.index_without_tic(c)?.len();
        let dad = dad_di(self.opz, l, c)?;
        let uv = dad.as_ref().map_or(0, |d| d.points());
        if n + uv > 0 {
            w.open_spectra(n + uv)?;
        }
        progress(0, n);
        spettri(&mut w, l, (c, n), (self.peaks, &mut pick), progress)?;
        progress(n, n);
        uv_spettri(&mut w, dad.as_ref())?;
        if n + uv > 0 {
            w.close_spectra()?;
        }
        chromatograms(l, c, &mut w)?;
        w.finish()
    }
}

/// The `n` scans of sample `c` into `w`: with `Opzioni::thread` above 1 the reading stays on this thread and the encoding goes to the pool of the writer
/// (`MzmlWriter::spettri`), otherwise one scan at a time. The file is the same; `progress` counts the scans read.
fn spettri<W: Write>(
    w: &mut MzmlWriter<W>,
    l: &mut dyn Reader,
    (c, n): (usize, usize),
    (peaks, pick): (Peaks, &mut Pick),
    progress: &mut dyn FnMut(usize, usize),
) -> Result<()> {
    #[cfg(not(target_family = "wasm"))]
    if w.thread() > 1 {
        return w.spettri((0..n).map(|k| {
            let s = scansione_di(peaks, l, (c, k), pick)?;
            let extra = l.scan_extra(c, k, &s);
            if (k + 1) % 32 == 0 {
                progress(k + 1, n);
            }
            Ok((s, extra))
        }));
    }
    for k in 0..n {
        let s = scansione_di(peaks, l, (c, k), pick)?;
        let extra = l.scan_extra(c, k, &s);
        w.spectrum_with(&s, &extra)?;
        if (k + 1) % 32 == 0 {
            progress(k + 1, n);
        }
    }
    Ok(())
}

/// Writes sample `c` of `l` as mzML to `out`; `progress(done, total)` counts the spectra.
pub fn scrivi<W: Write>(
    l: &mut dyn Reader,
    c: usize,
    out: W,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<W> {
    Conversion::default().write(l, c, out, progress)
}

/// `scrivi` with the zlib level and the numpress choice of `opz` (`mzmlwrite::Opzioni`).
pub fn write_with<W: Write>(
    l: &mut dyn Reader,
    c: usize,
    out: W,
    progress: &mut dyn FnMut(usize, usize),
    opz: Options,
) -> Result<W> {
    let conv = Conversion {
        opz,
        ..Conversion::default()
    };
    conv.write(l, c, out, progress)
}

/// The mzML of the `convert.mzml` step: `indexedmzML` (PSI-MS 4.2.6, offsets, SHA-1 checksum) with one `processingMethod` per entry of `steps`
/// after the conversion. The spectra and chromatograms are those of `scrivi_con`.
pub fn write_indexed<W: Write>(
    l: &mut dyn Reader,
    c: usize,
    out: W,
    progress: &mut dyn FnMut(usize, usize),
    opz: Options,
    steps: &[ProcStep],
) -> Result<W> {
    let conv = Conversion {
        opz,
        steps: Some(steps),
        ..Conversion::default()
    };
    conv.write(l, c, out, progress)
}

/// The chromatograms of sample `c` that hold points, numbered again from 0 as `scrivi` writes them.
fn chromatograms<W: Write>(l: &mut dyn Reader, c: usize, w: &mut MzmlWriter<W>) -> Result<()> {
    let mut traces = Vec::new();
    for v in l.chromatograms(c)? {
        let mut t = l.chromatogram(c, &v.id)?;
        if !t.time.is_empty() {
            t.index = traces.len();
            traces.push(t);
        }
    }
    if !traces.is_empty() {
        w.open_chromatograms(traces.len())?;
        for t in &traces {
            w.chromatogram(t)?;
        }
        w.close_chromatograms()?;
    }
    Ok(())
}

/// The mzML of sample `c` without its spectra: the header and the chromatograms exactly as `scrivi` writes them, then a comment that names the
/// polarities of the scans (what a whole mzML says of them where `mzmlhead::Mzml::header_polarity` looks). What the routes that read the header
/// and the chromatograms of a file (`mzmlhead::Mzml`) read for a vendor file; its scans come from the reader (`ReadAt::lettore`).
pub fn guscio(l: &mut dyn Reader, c: usize) -> Result<Vec<u8>> {
    if c >= l.samples().len() {
        return Err(Error::new("error.badIndex").with("c", c));
    }
    let mut w = MzmlWriter::new(Vec::new(), &file_header(l, c)?)?;
    let n = l.index_without_tic(c)?.len();
    if n > 0 {
        w.open_spectra(n)?;
        w.close_spectra()?;
    }
    chromatograms(l, c, &mut w)?;
    let (pos, neg) = l
        .index_without_tic(c)?
        .iter()
        .fold((false, false), |(p, q), v| {
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
