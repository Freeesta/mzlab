//! Native builder of the `.mzidx` (`mzlab-core::mzidx`, the same code as the site's Rust workers): the source file is memory-mapped (`memmap2`),
//! the builds of several files run on a `rayon` thread pool (as many threads as cores minus one, at most 4: the window keeps its core), and the
//! index is written to the cache folder of the app as `<name>.mzidx` (first as `.mzidx.part`, then renamed; the peaks past the budget wait in
//! `<name>.mzidx.sp.part`, removed at the end). The page asks over `mzlab://`:
//! `idx/build` (POST, body = the scan table, see `decode_inputs`), `idx/read` (the index in pieces, to put it in the archive of the page), `idx/remove`.
//! «Cancel» of the loading (`lr_cancel`, `cancel`) stops the builds of a name at their next read or write and leaves no file of them, as on the site.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use mzlab_core::idxread::ReadAt;
use mzlab_core::mzidx::{self, ScanTable, Stats};
use mzlab_core::{budget_auto, upload, Error};

use crate::mapped::Mapped;
use crate::rpc::MapAt;

/// Threads of the build pool: cores − 1, at least 1, at most 4.
pub fn pool_size() -> usize {
    std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .saturating_sub(1)
        .clamp(1, 4)
}

pub struct IdxBuilder {
    pub(crate) dir: PathBuf,
    pub(crate) pool: rayon::ThreadPool,
    /// name -> its stop flag: set by «Cancel», replaced by a new upload of the name (a stopped build keeps the flag it took)
    pub(crate) stops: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

/// The error of a build stopped by «Cancel»: the page then removes the file (`lr_remove`), as the site does.
pub(crate) fn cancelled() -> Error {
    Error::new("err.load.cancelled")
}

/// A file or source of a build that fails at its next read or write once the build is stopped.
pub(crate) struct Fermabile<'a, T> {
    inner: T,
    stop: &'a AtomicBool,
}

impl<'a, T> Fermabile<'a, T> {
    pub(crate) fn new(inner: T, stop: &'a AtomicBool) -> Self {
        Fermabile { inner, stop }
    }
    fn go(&self) -> std::io::Result<()> {
        if self.stop.load(Ordering::Relaxed) {
            return Err(std::io::Error::other("cancelled"));
        }
        Ok(())
    }
}

impl<T: Read> Read for Fermabile<'_, T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.go()?;
        self.inner.read(buf)
    }
}

impl<T: Write> Write for Fermabile<'_, T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.go()?;
        self.inner.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl<T: Seek> Seek for Fermabile<'_, T> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl<T: ReadAt> ReadAt for Fermabile<'_, T> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<(), Error> {
        self.go().map_err(|_| cancelled())?;
        self.inner.read_at(off, buf)
    }
}

/// The body of `idx/build`: eight items, each as a little-endian `u32` length and the bytes: levels (u8), polarities (i8), RT (f64 LE), TIC (f64 LE),
/// the JSON rows of the scans, the `source` object, the `run` object (UTF-8), and one byte for `hr1`.
pub fn decode_inputs(body: &[u8]) -> Result<ScanTable, Error> {
    let bad = || Error::new("err.file.index").with("detail", "bad scan table");
    let mut at = 0usize;
    let mut items: Vec<&[u8]> = Vec::new();
    for _ in 0..8 {
        let head = body.get(at..at + 4).ok_or_else(bad)?;
        let len = u32::from_le_bytes([head[0], head[1], head[2], head[3]]) as usize;
        at += 4;
        items.push(body.get(at..at + len).ok_or_else(bad)?);
        at += len;
    }
    let f64s = |b: &[u8]| -> Vec<f64> {
        b.chunks_exact(8)
            .map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
            .collect()
    };
    let text = |b: &[u8]| String::from_utf8(b.to_vec()).map_err(|_| bad());
    let table = ScanTable {
        level: items[0].to_vec(),
        polarity: items[1].iter().map(|&b| b as i8).collect(),
        rt: f64s(items[2]),
        tic: f64s(items[3]),
        scans_json: items[4].to_vec(),
        source_json: text(items[5])?,
        run_json: text(items[6])?,
        hr1: items[7].first().is_some_and(|&b| b != 0),
        declared: 0,
    };
    if table.polarity.len() != table.level.len()
        || table.rt.len() != table.level.len()
        || table.tic.len() != table.level.len()
    {
        return Err(bad());
    }
    Ok(table)
}

impl IdxBuilder {
    pub fn new(dir: PathBuf) -> Result<IdxBuilder, Error> {
        std::fs::create_dir_all(&dir).map_err(|e| io_error(&e))?;
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(pool_size())
            .build()
            .map_err(|e| Error::new("err.file.index").with("detail", e))?;
        Ok(IdxBuilder {
            dir,
            pool,
            stops: Mutex::default(),
        })
    }

    fn stop_of(&self, name: &str) -> Arc<AtomicBool> {
        let mut g = self.stops.lock().unwrap_or_else(|e| e.into_inner());
        g.entry(name.to_string()).or_default().clone()
    }

    /// «Cancel» of the loading of `name`: its builds (the index and the one as centroids) stop now and the ones asked later fail, until `resume`.
    pub fn cancel(&self, name: &str) {
        for n in [name.to_string(), format!("{name}.cen")] {
            self.stop_of(&n).store(true, Ordering::Relaxed);
        }
    }

    /// A new upload of `name` (`lr_put`) builds afresh.
    pub fn resume(&self, name: &str) {
        let mut g = self.stops.lock().unwrap_or_else(|e| e.into_inner());
        g.remove(name);
        g.remove(&format!("{name}.cen"));
    }

    pub fn path(&self, name: &str) -> Result<PathBuf, Error> {
        Ok(self.dir.join(format!("{}.mzidx", clean(name)?)))
    }

    /// `len` bytes of the index from `off`.
    pub fn read(&self, name: &str, off: u64, len: u64) -> Result<Vec<u8>, Error> {
        let mut f = File::open(self.path(name)?).map_err(|e| io_error(&e))?;
        let size = f.metadata().map_err(|e| io_error(&e))?.len();
        let len = len.min(size.saturating_sub(off)).min(64 << 20);
        let mut buf = Vec::new();
        buf.try_reserve_exact(len as usize)
            .map_err(|_| Error::memory_budget())?;
        buf.resize(len as usize, 0);
        f.seek(SeekFrom::Start(off)).map_err(|e| io_error(&e))?;
        f.read_exact(&mut buf).map_err(|e| io_error(&e))?;
        Ok(buf)
    }

    pub fn remove(&self, name: &str) {
        if let Ok(p) = self.path(name) {
            let _ = std::fs::remove_file(p);
        }
    }

    /// The index of `name` and its index as centroids (`lr_remove`).
    pub fn remove_all(&self, name: &str) {
        self.remove(name);
        self.remove(&format!("{name}.cen"));
    }

    /// `(size)` of an index in the folder, if there is one.
    pub fn size(&self, name: &str) -> Option<u64> {
        std::fs::metadata(self.path(name).ok()?)
            .ok()
            .map(|m| m.len())
    }
}

/// A name that is only a file name (the page never chooses a path).
pub(crate) fn clean(name: &str) -> Result<&str, Error> {
    if name.is_empty()
        || name.contains(['/', '\\'])
        || name == "."
        || name == ".."
        || name.contains('\0')
    {
        return Err(Error::new("err.file.index").with("detail", "bad name"));
    }
    Ok(name)
}

pub(crate) fn io_error(e: &std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

impl IdxBuilder {
    /// Builds the index of `source` (an mzML of the disk) as `<name>.mzidx`; waits for a free thread of the pool when several files are being built.
    /// `table`: the scan table the page sent; `None` reads it here (`upload::prepare_parallelo`: the SHA-256 is taken on a second thread meanwhile).
    pub fn build(
        &self,
        source: &Path,
        name: &str,
        table: Option<&ScanTable>,
    ) -> Result<Stats, Error> {
        self.write(name, |out, spill, stop| {
            let map = Arc::new(Mapped::open(source).map_err(|e| io_error(&e))?);
            let size = map.bytes().len() as u64;
            let read_back = match table {
                Some(_) => None,
                None => Some(upload::prepare_parallelo(
                    &mut Fermabile::new(MapAt(map.clone()), stop),
                    Fermabile::new(MapAt(map.clone()), stop),
                    size,
                    name,
                )?),
            };
            let table = table.unwrap_or_else(|| &read_back.as_ref().expect("read above").table);
            crate::ram::check_index(table)?;
            mzidx::build_mzml_spill(
                table,
                Fermabile::new(Cursor::new(map.bytes()), stop),
                out,
                budget_auto::native_index_budget(),
                Some(spill),
            )
        })
    }

    /// Runs `f` on a thread of the pool with the index file (`<name>.mzidx.<n>.part`, renamed `<name>.mzidx` when `f` succeeds, removed when it fails)
    /// and the scratch file (`<name>.mzidx.<n>.sp.part`: the peaks past the budget wait there, so the scans are decoded once; always removed).
    /// Both fail once «Cancel» stops `name` (`stop`, for the sources `f` reads); a build stopped, even one that ended past the cancel, leaves no file.
    pub(crate) fn write(
        &self,
        name: &str,
        f: impl FnOnce(
                &mut Fermabile<&mut File>,
                &mut Fermabile<&mut File>,
                &AtomicBool,
            ) -> Result<Stats, Error>
            + Send,
    ) -> Result<Stats, Error> {
        let _indexing = crate::conv_coda::Indexing::new();
        let dest = self.path(name)?;
        let stop = self.stop_of(name);
        let stopped = || stop.load(Ordering::Relaxed);
        // a suffix per build: two builds of one name must not truncate each other's files
        static BUILD_SEQ: AtomicU64 = AtomicU64::new(0);
        let n = BUILD_SEQ.fetch_add(1, Ordering::Relaxed);
        let part = self.dir.join(format!("{}.mzidx.{n}.part", clean(name)?));
        let sp = self.dir.join(format!("{}.mzidx.{n}.sp.part", clean(name)?));
        let r = self.pool.install(|| -> Result<Stats, Error> {
            if stopped() {
                return Err(cancelled()); // cancelled while it waited for a thread of the pool
            }
            let mut out = File::create(&part).map_err(|e| io_error(&e))?;
            let mut spill = File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&sp)
                .map_err(|e| io_error(&e))?;
            f(
                &mut Fermabile::new(&mut out, &stop),
                &mut Fermabile::new(&mut spill, &stop),
                &stop,
            )
        });
        let _ = std::fs::remove_file(&sp);
        let r = if stopped() { Err(cancelled()) } else { r };
        match r {
            Ok(st) => {
                std::fs::rename(&part, &dest).map_err(|e| io_error(&e))?;
                Ok(st)
            }
            Err(e) => {
                let _ = std::fs::remove_file(&part);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
#[path = "idxbuild_tests.rs"]
mod tests;
