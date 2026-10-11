//! The files of the session by slot (the index `k` of the page) and the answers of the engine, with the same arithmetic and the same
//! rounding as `mzlab-wasm` (RT to 4 decimals, intensities to 1, m/z to 3) so that the page gets the numbers it gets from Python.

use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use mzlab_core::{analysis, mzml, raw, Error};

use crate::mapped::Mapped;

fn r1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
fn r3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}
fn r4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

/// A file chosen by the user (system dialog or drop): the page only ever sees its number.
#[derive(Clone, Debug)]
pub struct Picked {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

/// How far a `.raw` conversion has got (read by `rawprogress` while `raw_to_mzml` runs in another thread).
#[derive(Default)]
pub struct Progress {
    pub total: AtomicU64,
    pub read: AtomicU64,
    pub written: AtomicU64,
}

/// A reader that tells how far the source has been read.
struct CountRead<'a> {
    inner: Cursor<&'a [u8]>,
    progress: Arc<Progress>,
}
impl Read for CountRead<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.progress
            .read
            .fetch_max(self.inner.position(), Ordering::Relaxed);
        Ok(n)
    }
}
impl Seek for CountRead<'_> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

/// A writer that counts the bytes written.
struct CountWrite<W: Write> {
    inner: W,
    progress: Arc<Progress>,
}
impl<W: Write> Write for CountWrite<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.progress.written.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[derive(Default)]
struct Inner {
    /// progress of the conversion of each `.raw` (by the number of the picked file)
    progress: HashMap<u32, Arc<Progress>>,
    next_id: u32,
    picked: HashMap<u32, Picked>,
    runs: HashMap<u32, mzml::Run>,
    /// mzML files written by `raw2mzml` (also in `picked`, so that the page reads and opens them as any file); removed at exit.
    temps: Vec<PathBuf>,
}

/// Shared by the protocol handler (any thread) and the Tauri commands.
#[derive(Default)]
pub struct State {
    inner: Mutex<Inner>,
    /// the folder of the spectral libraries (`librerie.rs`)
    pub libs: crate::librerie::Folder,
    /// where the `.mzidx` of the picked files are built (`set_index_folder`; a folder of the system temp dir until then)
    idx: Mutex<Option<Arc<crate::idxbuild::IdxBuilder>>>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl State {
    pub fn new() -> State {
        State::default()
    }

    /// The cache folder of the app for the indexes (the page asks for the builder through `idx/...`).
    pub fn set_index_folder(&self, dir: PathBuf) -> Result<()> {
        let b = crate::idxbuild::IdxBuilder::new(dir)?;
        *self.idx.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(b));
        Ok(())
    }

    pub fn indexer(&self) -> Result<Arc<crate::idxbuild::IdxBuilder>> {
        let mut g = self.idx.lock().unwrap_or_else(|e| e.into_inner());
        if g.is_none() {
            *g = Some(Arc::new(crate::idxbuild::IdxBuilder::new(
                std::env::temp_dir().join("mzlab-mzidx"),
            )?));
        }
        Ok(g.as_ref().cloned().expect("just set"))
    }

    /// `idx/build`: the index of the picked file `id`, built in the cache folder as `<name>.mzidx`. Returns the statistics as JSON.
    pub fn build_index(&self, id: u32, name: &str, body: &[u8]) -> Result<String> {
        let p = self.picked(id)?;
        let table = crate::idxbuild::decode_inputs(body)?;
        let st = self.indexer()?.build(&p.path, name, &table)?;
        Ok(format!(
            "{{\"scans\":{},\"peaks\":{},\"sections\":{},\"passes\":{},\"size\":{}}}",
            st.scans, st.peaks, st.sections, st.passes, st.bytes
        ))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Registers a file of the disk and returns its number.
    pub fn pick(&self, path: PathBuf) -> Result<(u32, Picked)> {
        let meta = std::fs::metadata(&path).map_err(|e| io_error(&e))?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut g = self.lock();
        g.next_id += 1;
        let id = g.next_id;
        let p = Picked {
            path,
            name,
            size: meta.len(),
        };
        g.picked.insert(id, p.clone());
        Ok((id, p))
    }

    fn picked(&self, id: u32) -> Result<Picked> {
        self.lock()
            .picked
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::new("err.file.raw").with("detail", "unknown file"))
    }

    /// Opens an mzML of the disk in `slot`. Returns the summary JSON that `mzlab-wasm` gives:
    /// `{"spectra": n, "levels": {"1": [scans, profile scans], ...}, "unit": bool, "chromatograms": m}`.
    pub fn open(&self, slot: u32, id: u32) -> Result<String> {
        let p = self.picked(id)?;
        let map = Mapped::open(&p.path).map_err(|e| io_error(&e))?;
        let run = if is_raw(&p.name) {
            raw::read(Cursor::new(map.bytes()))?
        } else {
            mzml::read(Cursor::new(map.bytes()))?
        };
        let mut levels: BTreeMap<u8, (usize, usize)> = BTreeMap::new();
        for s in &run.spectra {
            let e = levels.entry(s.level).or_default();
            e.0 += 1;
            e.1 += s.profile as usize;
        }
        let lv: Vec<String> = levels
            .iter()
            .map(|(l, (n, p))| format!("\"{l}\":[{n},{p}]"))
            .collect();
        let out = format!(
            "{{\"spectra\":{},\"levels\":{{{}}},\"unit\":{},\"chromatograms\":{}}}",
            run.spectra.len(),
            lv.join(","),
            analysis::is_unit_resolution(&run.analyzers, &run.spectra),
            run.chromatograms.len()
        );
        self.lock().runs.insert(slot, run);
        Ok(out)
    }

    pub fn close(&self, slot: u32) {
        self.lock().runs.remove(&slot);
    }

    pub fn reset(&self) {
        self.lock().runs.clear();
    }

    /// Converts a Thermo `.raw` of the disk (any size: the file is mapped, not read into memory) to an mzML in the temporary
    /// folder, registered like a picked file. Returns `(number, size)`; the page reads it with `chunk`.
    pub fn raw_to_mzml(&self, id: u32) -> Result<(u32, u64)> {
        let p = self.picked(id)?;
        let map = Mapped::open(&p.path).map_err(|e| io_error(&e))?;
        let progress = Arc::new(Progress::default());
        progress
            .total
            .store(map.bytes().len() as u64, Ordering::Relaxed);
        self.lock().progress.insert(id, progress.clone());
        let tmp = std::env::temp_dir().join(format!("mzlab-{}-{}.mzML", std::process::id(), id));
        let mut out = CountWrite {
            inner: std::io::BufWriter::new(std::fs::File::create(&tmp).map_err(|e| io_error(&e))?),
            progress: progress.clone(),
        };
        let stem = p.name.rsplit_once('.').map_or(p.name.as_str(), |(s, _)| s);
        let source = CountRead {
            inner: Cursor::new(map.bytes()),
            progress: progress.clone(),
        };
        raw::to_mzml(source, &mut out, stem)?;
        progress
            .read
            .store(map.bytes().len() as u64, Ordering::Relaxed);
        out.flush().map_err(|e| io_error(&e))?;
        drop(out);
        let size = std::fs::metadata(&tmp).map_err(|e| io_error(&e))?.len();
        let mut g = self.lock();
        g.next_id += 1;
        let tid = g.next_id;
        g.picked.insert(
            tid,
            Picked {
                path: tmp.clone(),
                name: format!("{stem}.mzML"),
                size,
            },
        );
        g.temps.push(tmp);
        Ok((tid, size))
    }

    /// `(total, read, written)` bytes of the conversion of the `.raw` with this number (None if it never started).
    pub fn raw_progress(&self, id: u32) -> Option<(u64, u64, u64)> {
        self.lock().progress.get(&id).map(|p| {
            (
                p.total.load(Ordering::Relaxed),
                p.read.load(Ordering::Relaxed),
                p.written.load(Ordering::Relaxed),
            )
        })
    }

    /// Copies a converted mzML (or any picked file) to `dest`; returns the size.
    pub fn save_copy(&self, id: u32, dest: &std::path::Path) -> Result<u64> {
        let p = self.picked(id)?;
        std::fs::copy(&p.path, dest).map_err(|e| io_error(&e))
    }

    /// The number of the file with this name and size (the page knows files by name and size only), the newest first.
    pub fn find(&self, name: &str, size: u64) -> Option<u32> {
        let g = self.lock();
        g.picked
            .iter()
            .filter(|(_, p)| p.name == name && p.size == size)
            .map(|(id, _)| *id)
            .max()
    }

    /// Removes the temporary mzML files (at exit).
    pub fn cleanup(&self) {
        for p in self.lock().temps.drain(..) {
            let _ = std::fs::remove_file(p);
        }
    }

    /// `len` bytes from `off` of a file picked by the user or converted by `raw_to_mzml`.
    pub fn chunk(&self, id: u32, off: u64, len: u64) -> Result<Vec<u8>> {
        let map = Mapped::open(&self.picked(id)?.path).map_err(|e| io_error(&e))?;
        let b = map.bytes();
        let a = (off as usize).min(b.len());
        let z = (off.saturating_add(len) as usize).min(b.len());
        Ok(b[a..z].to_vec())
    }

    fn with_run<T>(&self, slot: u32, f: impl FnOnce(&mzml::Run) -> T) -> Result<T> {
        let g = self.lock();
        g.runs
            .get(&slot)
            .map(f)
            .ok_or_else(|| Error::new("err.file.raw").with("detail", "slot not open"))
    }

    /// `[rt(n), y(n)]` of the TIC of one level (`px > 0`: at most two points per pixel column).
    pub fn tic(&self, slot: u32, level: u8, px: u32) -> Result<Vec<f64>> {
        self.with_run(slot, |r| {
            let (rt, y) = analysis::tic(&r.spectra, level);
            trace(&rt, &y, px)
        })
    }

    /// `[rt(n), y(n)]` of the XIC over `mz ± tol`.
    pub fn xic(&self, slot: u32, level: u8, mz: f64, tol: f64, px: u32) -> Result<Vec<f64>> {
        self.with_run(slot, |r| {
            let (rt, y) = analysis::xic(&r.spectra, level, mz - tol, mz + tol);
            trace(&rt, &y, px)
        })
    }

    /// Centroid scans `i0..=i1` of one level in bins of `bin_da`: `[n_total, count, (sid, rt, k, mz(k), y(k)) per scan]`;
    /// empty if `i0` is out of range.
    pub fn spectra(&self, slot: u32, level: u8, i0: u32, i1: u32, bin_da: f64) -> Result<Vec<f64>> {
        self.with_run(slot, |r| {
            let all: Vec<&mzml::Spectrum> = r.spectra.iter().filter(|s| s.level == level).collect();
            if i0 as usize >= all.len() {
                return Vec::new();
            }
            let mut out = vec![all.len() as f64, 0.0];
            let mut count = 0.0;
            for (pos, s) in all.iter().enumerate() {
                if (pos as u32) < i0 || pos as u32 > i1 {
                    continue;
                }
                let (mz, y) = if s.mz.is_empty() {
                    (Vec::new(), Vec::new())
                } else {
                    analysis::bin_centroids(&s.mz, &s.intensity, bin_da)
                };
                out.push(s.index as f64);
                out.push(r4(s.rt));
                out.push(mz.len() as f64);
                out.extend(mz.iter().map(|v| r3(*v)));
                out.extend(y.iter().map(|v| r1(*v)));
                count += 1.0;
            }
            out[1] = count;
            out
        })
    }
}

fn trace(rt: &[f64], y: &[f64], px: u32) -> Vec<f64> {
    let keep = analysis::minmax_indices(rt, y, px as usize);
    let n = keep.len();
    let mut out = vec![0.0; 2 * n];
    for (j, &i) in keep.iter().enumerate() {
        out[j] = r4(rt[i]);
        out[n + j] = r1(y[i]);
    }
    out
}

fn is_raw(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".raw")
}

fn io_error(e: &std::io::Error) -> Error {
    Error::new("err.file.raw").with("detail", e.to_string())
}
