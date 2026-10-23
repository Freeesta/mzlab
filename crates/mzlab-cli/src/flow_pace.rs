//! The native index build and the counters it reports its progress with: a reader that counts the scans it decodes (`Scans`) and a `Read` that counts bytes
//! (`Counted`). They forward everything and only tell a `Pace` (`mzlab_core::pace`) how far the job is, so the index made is the same bytes. The `Gate` is
//! what a live run (`flow_live.rs`) hands a job: the flag that cancels it and where it tells its progress; the readers and writers below stop with
//! `err.flow.cancelled` at the next piece once the flag is up, and are the very same bytes while it is down.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limiti;
use mzlab_core::mzidx::build_mzml_late;
use mzlab_core::upload::{impronte, indicizza_lettore, prepare_con, prepare_late, probe};

use crate::{flow_set, io};

use mzlab_core::lettori::{Campione, Campo, Formato, Lettore, VoceCromatogramma, VoceScansione};
use mzlab_core::mzml::{Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};
use mzlab_core::pace::Pace;

/// The flag that cancels a live job (none: a job of `flow run`).
pub(crate) type Stop = Option<Arc<AtomicBool>>;

/// Tells the share done (0 to 1) of the job.
pub(crate) type Report = Arc<dyn Fn(f64) + Send + Sync>;

pub(crate) fn cancelled() -> Error {
    Error::new("err.flow.cancelled")
}

fn up(stop: &Stop) -> bool {
    stop.as_ref().is_some_and(|f| f.load(Ordering::Relaxed))
}

/// What a live run gives a job: its cancel flag and where it reports. `Gate::none()` is a job with neither.
#[derive(Clone, Default)]
pub(crate) struct Gate {
    pub(crate) stop: Stop,
    pub(crate) report: Option<Report>,
}

impl Gate {
    pub(crate) fn none() -> Gate {
        Gate::default()
    }

    pub(crate) fn stopped(&self) -> bool {
        up(&self.stop)
    }

    /// The `(done, total)` callback of the job: the share done goes to `report`.
    pub(crate) fn pace(&self) -> impl FnMut(usize, usize) + '_ {
        move |d, t| {
            if let Some(r) = &self.report {
                r(if t == 0 { 1.0 } else { d as f64 / t as f64 });
            }
        }
    }
}

/// What a live run knows about the jobs it runs: the gate of each.
pub(crate) trait Watch: Send + Sync {
    fn gate(&self, instance: &str, item: Option<&str>) -> Gate;
}

/// A file (or any reader or writer) that fails with `err.flow.cancelled` once the flag is up; the bytes that go through are untouched.
pub(crate) struct Stopping<T> {
    inner: T,
    stop: Stop,
}

impl<T> Stopping<T> {
    pub(crate) fn new(inner: T, stop: Stop) -> Self {
        Stopping { inner, stop }
    }

    fn check(&self) -> std::io::Result<()> {
        match up(&self.stop) {
            true => Err(std::io::Error::other("cancelled")),
            false => Ok(()),
        }
    }
}

impl<T: Read> Read for Stopping<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.check()?;
        self.inner.read(buf)
    }
}

impl<T: Write> Write for Stopping<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.check()?;
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// An index being read by a detection: stops at the next read once the flag is up and reports the share of the file read, as far as 99 % (the job
/// reads parts again after the first pass, so the end is the job's own).
pub(crate) struct Gated<R> {
    inner: R,
    gate: Gate,
    size: u64,
    read: u64,
    pct: u64,
}

impl<R> Gated<R> {
    pub(crate) fn new(inner: R, gate: &Gate, size: u64) -> Self {
        Gated {
            inner,
            gate: gate.clone(),
            size,
            read: 0,
            pct: 0,
        }
    }
}

impl<R: ReadAt> ReadAt for Gated<R> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        if self.gate.stopped() {
            return Err(cancelled());
        }
        self.inner.read_at(off, buf)?;
        self.read += buf.len() as u64;
        let pct = (self.read.min(self.size) * 99 / self.size.max(1)).min(99);
        if pct > self.pct {
            self.pct = pct;
            if let Some(r) = &self.gate.report {
                r(pct as f64 / 100.0);
            }
        }
        Ok(())
    }
}

type Late = Box<dyn FnMut() -> Result<String>>;

/// The scan table of an mzML and, when its fingerprints are not known, the function that gives the `source` object once a second thread has taken
/// them (the index is built meanwhile); without threads the fingerprints are taken at once.
fn prepare_unknown(
    path: &Path,
    file: &mut File,
    size: u64,
    name: &str,
) -> Result<(mzlab_core::mzidx::ScanTable, Option<Late>)> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (table, late) = prepare_late(file, File::open(path).map_err(io)?, size, name)?;
        Ok((table, Some(Box::new(late))))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        Ok((prepare_con(file, size, name, None)?.table, None))
    }
}

/// The `.mzidx` of one file (an mzML, or the sample `label` of any dataset a reader takes) at `idx`.
pub(crate) fn build_index_of(path: &Path, label: &str, idx: &Path, budget: usize) -> Result<()> {
    build_index_with(path, label, idx, budget, &mut |_, _| {}, &None)
}

/// `build_index_of` telling `step(done, total)` how far pass 1 has got (scans of the sample, bytes of an mzML), at most once per 1 %; same bytes with any
/// `step`. With the flag `stop` up the build ends at the next piece it reads (the file at `idx` is then partial: the caller removes it).
pub(crate) fn build_index_with(
    path: &Path,
    label: &str,
    idx: &Path,
    budget: usize,
    step: &mut dyn FnMut(usize, usize),
    stop: &Stop,
) -> Result<()> {
    let mut pace = mzlab_core::pace::Pace::new(step);
    let name = path
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let mut out = File::create(idx).map_err(io)?;
    if !path.is_dir() && name.to_lowercase().ends_with(".mzml") {
        let size = std::fs::metadata(path).map_err(io)?.len();
        let mut file = File::open(path).map_err(io)?;
        let (table, mut late) = match flow_set::known_sha(path) {
            Some(sha) => {
                let known = Some((sha, probe(&mut file, size)?));
                (prepare_con(&mut file, size, &name, known)?.table, None)
            }
            None => prepare_unknown(path, &mut file, size, &name)?,
        };
        build_mzml_late(
            &table,
            BufReader::new(
                Counted::new(File::open(path).map_err(io)?, size, &mut pace).stopping(stop),
            ),
            &mut out,
            budget,
            None,
            late.as_mut()
                .map(|l| l as &mut dyn FnMut() -> Result<String>),
        )?;
    } else {
        let mut fs = flow_set::file_set(path)?;
        let imp = impronte(&mut fs)?;
        let l = mzlab_lettori::apri(fs, &Limiti::desktop())?;
        let ids: Vec<u32> = l.campioni().iter().map(|s| s.id).collect();
        let sample = flow_set::sample_index(label, &ids)?;
        let mut l = Scans::new(l, sample, &mut pace).stopping(stop);
        indicizza_lettore(&mut l, sample, &imp, &name, &mut out, budget, &mut |_| {})?;
    }
    pace.at(1, 1);
    Ok(())
}

/// A reader that tells `pace` how many scans of sample `c` it has decoded (as many as the sample has in pass 1; a later pass counts past the total and is clamped).
pub(crate) struct Scans<'a, 'b> {
    l: Box<dyn Lettore>,
    c: usize,
    n: usize,
    done: usize,
    pace: &'a mut Pace<'b>,
    stop: Stop,
}

impl<'a, 'b> Scans<'a, 'b> {
    pub(crate) fn new(mut l: Box<dyn Lettore>, c: usize, pace: &'a mut Pace<'b>) -> Self {
        let n = l.indice(c).map_or(0, |v| v.len());
        pace.at(0, n);
        Scans {
            l,
            c,
            n,
            done: 0,
            pace,
            stop: None,
        }
    }

    /// The reader fails with `err.flow.cancelled` at the next scan once `stop` is up.
    pub(crate) fn stopping(mut self, stop: &Stop) -> Self {
        self.stop = stop.clone();
        self
    }

    fn gone(&self) -> Result<()> {
        match up(&self.stop) {
            true => Err(cancelled()),
            false => Ok(()),
        }
    }

    fn count(&mut self, c: usize) {
        if c == self.c {
            self.done += 1;
            self.pace.at(self.done, self.n);
        }
    }
}

#[rustfmt::skip]
impl Lettore for Scans<'_, '_> {
    fn formato(&self) -> Formato { self.l.formato() }
    fn campioni(&self) -> &[Campione] { self.l.campioni() }
    fn indice(&mut self, c: usize) -> mzlab_core::Result<&[VoceScansione]> { self.l.indice(c) }
    fn scansione(&mut self, c: usize, k: usize) -> mzlab_core::Result<Spectrum> { self.gone()?; let s = self.l.scansione(c, k); self.count(c); s }
    fn scansione_dati(&mut self, c: usize, k: usize) -> mzlab_core::Result<Spectrum> { self.gone()?; let s = self.l.scansione_dati(c, k); self.count(c); s }
    fn scansione_profilo(&mut self, c: usize, k: usize) -> mzlab_core::Result<Option<Spectrum>> { self.l.scansione_profilo(c, k) }
    fn intestazione(&mut self, c: usize, k: usize) -> mzlab_core::Result<Vec<Campo>> { self.l.intestazione(c, k) }
    fn cromatogrammi(&mut self, c: usize) -> mzlab_core::Result<Vec<VoceCromatogramma>> { self.l.cromatogrammi(c) }
    fn cromatogramma(&mut self, c: usize, id: &str) -> mzlab_core::Result<Chromatogram> { self.l.cromatogramma(c, id) }
    fn dad_map(&mut self, c: usize) -> mzlab_core::Result<Option<String>> { self.l.dad_map(c) }
    fn metodo(&mut self, c: usize) -> mzlab_core::Result<String> { self.l.metodo(c) }
    fn testata_mzml(&mut self, c: usize) -> mzlab_core::Result<Option<Header>> { self.l.testata_mzml(c) }
    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra { self.l.extra_mzml(s) }
    fn extra_scansione(&mut self, c: usize, k: usize, s: &Spectrum) -> SpectrumExtra { self.l.extra_scansione(c, k, s) }
    fn non_portati(&self) -> &'static [(&'static str, &'static str)] { self.l.non_portati() }
    fn mancanti(&mut self, c: usize) -> usize { self.l.mancanti(c) }
    fn variante(&mut self, c: usize) -> bool { self.l.variante(c) }
}

/// A `Read` that tells `pace` how many of the `size` bytes of the file it has handed out.
pub(crate) struct Counted<'a, 'b, R> {
    r: R,
    size: usize,
    read: usize,
    pace: &'a mut Pace<'b>,
    stop: Stop,
}

impl<'a, 'b, R> Counted<'a, 'b, R> {
    pub(crate) fn new(r: R, size: u64, pace: &'a mut Pace<'b>) -> Self {
        Counted {
            r,
            size: size as usize,
            read: 0,
            pace,
            stop: None,
        }
    }

    /// The reader fails at the next read once `stop` is up.
    pub(crate) fn stopping(mut self, stop: &Stop) -> Self {
        self.stop = stop.clone();
        self
    }
}

impl<R: Seek> Seek for Counted<'_, '_, R> {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let at = self.r.seek(to)?;
        self.read = at as usize;
        Ok(at)
    }
}

impl<R: Read> Read for Counted<'_, '_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if up(&self.stop) {
            return Err(std::io::Error::other("cancelled"));
        }
        let n = self.r.read(buf)?;
        self.read += n;
        self.pace.at(self.read, self.size);
        Ok(n)
    }
}
