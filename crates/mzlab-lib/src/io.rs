//! Byte access the library needs, independent of where the bytes live (a slice, a file, an OPFS handle through the WASM bridge).

use crate::error::{Error, Result};
use std::cell::RefCell;
use std::io::{Read, Seek, SeekFrom};

/// Random read access to a file.
pub trait Source {
    fn len(&self) -> u64;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Fills `buf` from `off`; reading past the end is `lib.err.corrupt` (a truncated file), never a panic.
    fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<()>;
}

/// Random write access (the library being built, scratch space).
pub trait Sink {
    /// Writes at `off`, growing the file if needed (the gap, if any, reads as zeros).
    fn write_at(&mut self, off: u64, data: &[u8]) -> Result<()>;
}

/// Scratch space of the builder: readable and writable.
pub trait Scratch: Source + Sink {}
impl<T: Source + Sink> Scratch for T {}

impl Source for [u8] {
    fn len(&self) -> u64 {
        <[u8]>::len(self) as u64
    }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<()> {
        let end = off
            .checked_add(buf.len() as u64)
            .ok_or_else(|| Error::corrupt("truncated"))?;
        if end > <[u8]>::len(self) as u64 {
            return Err(Error::corrupt("truncated"));
        }
        buf.copy_from_slice(&self[off as usize..end as usize]);
        Ok(())
    }
}

impl Source for Vec<u8> {
    fn len(&self) -> u64 {
        self.as_slice().len() as u64
    }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.as_slice().read_at(off, buf)
    }
}

impl<T: Source + ?Sized> Source for &T {
    fn len(&self) -> u64 {
        (**self).len()
    }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<()> {
        (**self).read_at(off, buf)
    }
}

impl Sink for Vec<u8> {
    fn write_at(&mut self, off: u64, data: &[u8]) -> Result<()> {
        let end = usize::try_from(off)
            .ok()
            .and_then(|o| o.checked_add(data.len()))
            .ok_or_else(Error::memory_budget)?;
        if end > self.len() {
            self.try_reserve(end - self.len())
                .map_err(|_| Error::memory_budget())?;
            self.resize(end, 0);
        }
        self[end - data.len()..end].copy_from_slice(data);
        Ok(())
    }
}

/// Any `Read + Seek` (a file on disk) as a `Source`.
pub struct ReadSeekSource<R: Read + Seek> {
    inner: RefCell<R>,
    len: u64,
}

impl<R: Read + Seek> ReadSeekSource<R> {
    pub fn new(mut r: R) -> Result<Self> {
        let len = r
            .seek(SeekFrom::End(0))
            .map_err(|_| Error::new("lib.err.io"))?;
        Ok(ReadSeekSource {
            inner: RefCell::new(r),
            len,
        })
    }
}

impl<R: Read + Seek> Source for ReadSeekSource<R> {
    fn len(&self) -> u64 {
        self.len
    }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<()> {
        let end = off
            .checked_add(buf.len() as u64)
            .ok_or_else(|| Error::corrupt("truncated"))?;
        if end > self.len {
            return Err(Error::corrupt("truncated"));
        }
        let mut r = self.inner.borrow_mut();
        r.seek(SeekFrom::Start(off))
            .and_then(|_| r.read_exact(buf))
            .map_err(|_| Error::new("lib.err.io"))
    }
}

const FLUSH_AT: usize = 64 << 10;

struct Stream {
    /// (logical start, physical offset, length)
    extents: Vec<(u64, u64, u32)>,
    len: u64,
    buf: Vec<u8>,
}

/// Append-only byte streams sharing one scratch file: the builder keeps its bucket streams here without one file each.
pub struct ExtentStore<T: Scratch> {
    t: T,
    end: u64,
    streams: Vec<Stream>,
}

impl<T: Scratch> ExtentStore<T> {
    pub fn new(t: T, n_streams: usize) -> Self {
        let end = t.len();
        ExtentStore {
            t,
            end,
            streams: (0..n_streams)
                .map(|_| Stream {
                    extents: Vec::new(),
                    len: 0,
                    buf: Vec::new(),
                })
                .collect(),
        }
    }

    pub fn len(&self, s: usize) -> u64 {
        self.streams[s].len
    }

    pub fn append(&mut self, s: usize, data: &[u8]) -> Result<()> {
        let st = &mut self.streams[s];
        st.len += data.len() as u64;
        st.buf.extend_from_slice(data);
        if st.buf.len() >= FLUSH_AT {
            self.flush(s)?;
        }
        Ok(())
    }

    pub fn flush(&mut self, s: usize) -> Result<()> {
        let st = &mut self.streams[s];
        if st.buf.is_empty() {
            return Ok(());
        }
        let logical = st.len - st.buf.len() as u64;
        self.t.write_at(self.end, &st.buf)?;
        st.extents.push((logical, self.end, st.buf.len() as u32));
        self.end += st.buf.len() as u64;
        st.buf.clear();
        st.buf.shrink_to(FLUSH_AT);
        Ok(())
    }

    pub fn flush_all(&mut self) -> Result<()> {
        for s in 0..self.streams.len() {
            self.flush(s)?;
        }
        Ok(())
    }

    /// Reads from the flushed part of a stream (call `flush` first).
    pub fn read_at(&self, s: usize, off: u64, buf: &mut [u8]) -> Result<()> {
        let st = &self.streams[s];
        let mut pos = off;
        let mut done = 0usize;
        if off.checked_add(buf.len() as u64).is_none_or(|e| e > st.len) {
            return Err(Error::corrupt("scratch"));
        }
        while done < buf.len() {
            let k = st
                .extents
                .partition_point(|e| e.0 <= pos)
                .checked_sub(1)
                .ok_or_else(|| Error::corrupt("scratch"))?;
            let (ls, ps, l) = st.extents[k];
            let inside = pos - ls;
            let take = ((l as u64 - inside) as usize).min(buf.len() - done);
            self.t.read_at(ps + inside, &mut buf[done..done + take])?;
            done += take;
            pos += take as u64;
        }
        Ok(())
    }

    pub fn into_inner(self) -> T {
        self.t
    }
}
