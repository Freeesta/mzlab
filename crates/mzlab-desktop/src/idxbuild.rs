//! Native builder of the `.mzidx` (`mzlab-core::mzidx`, the same code as the site's Rust workers): the source file is memory-mapped (`memmap2`),
//! the builds of several files run on a `rayon` thread pool (as many threads as cores minus one, at most 4: the window keeps its core), and the
//! index is written to the cache folder of the app as `<name>.mzidx` (first as `.mzidx.part`, then renamed; the peaks past the budget wait in
//! `<name>.mzidx.sp.part`, removed at the end). The page asks over `mzlab://`:
//! `idx/build` (POST, body = the scan table, see `decode_inputs`), `idx/read` (the index in pieces, to put it in the archive of the page), `idx/remove`.

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mzlab_core::mzidx::{self, ScanTable, Stats, DEFAULT_BUDGET};
use mzlab_core::{upload, Error};

use crate::mapped::Mapped;
use crate::rpc::MapAt;

pub(crate) fn io_error(e: &std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

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
        Ok(IdxBuilder { dir, pool })
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

impl IdxBuilder {
    /// Builds the index of `source` (an mzML of the disk) as `<name>.mzidx`; waits for a free thread of the pool when several files are being built.
    /// `table`: the scan table the page sent; `None` reads it here (`upload::prepare_parallelo`: the SHA-256 is taken on a second thread meanwhile).
    pub fn build(
        &self,
        source: &Path,
        name: &str,
        table: Option<&ScanTable>,
    ) -> Result<Stats, Error> {
        self.scrivi(name, |out, spill| {
            let map = Arc::new(Mapped::open(source).map_err(|e| io_error(&e))?);
            let size = map.bytes().len() as u64;
            let letta = match table {
                Some(_) => None,
                None => Some(upload::prepare_parallelo(
                    &mut MapAt(map.clone()),
                    MapAt(map.clone()),
                    size,
                    name,
                )?),
            };
            let table = table.unwrap_or_else(|| &letta.as_ref().expect("read above").table);
            mzidx::build_mzml_spill(
                table,
                Cursor::new(map.bytes()),
                out,
                DEFAULT_BUDGET,
                Some(spill),
            )
        })
    }

    /// Runs `f` on a thread of the pool with the index file (`<name>.mzidx.part`, renamed `<name>.mzidx` when `f` succeeds, removed when it fails)
    /// and the scratch file (`<name>.mzidx.sp.part`: the peaks past the budget wait there, so the scans are decoded once; always removed).
    pub(crate) fn scrivi(
        &self,
        name: &str,
        f: impl FnOnce(&mut File, &mut File) -> Result<Stats, Error> + Send,
    ) -> Result<Stats, Error> {
        let dest = self.path(name)?;
        let part = self.dir.join(format!("{}.mzidx.part", clean(name)?));
        let sp = self.dir.join(format!("{}.mzidx.sp.part", clean(name)?));
        let r = self.pool.install(|| -> Result<Stats, Error> {
            let mut out = File::create(&part).map_err(|e| io_error(&e))?;
            let mut spill = File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&sp)
                .map_err(|e| io_error(&e))?;
            f(&mut out, &mut spill)
        });
        let _ = std::fs::remove_file(&sp);
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
