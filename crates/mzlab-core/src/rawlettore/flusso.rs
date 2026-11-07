//! `Stream`: the `ReadAt` of a `.raw` member seen as `Read + Seek`, what `opentfraw` reads from. `opentfraw` reads a few bytes at a time and on the
//! site every `read_at` is a call into JS, so a window serves the short reads; it starts small after a seek (a header, a centroid list) and grows
//! while the reads go on in order (a profile).

use std::io::{Read, Seek, SeekFrom};

use crate::idxread::ReadAt;

/// The largest window of `Stream`: `opentfraw` reads a few bytes at a time and a read of the source can cost a call into JS (the logic of `window::Windowed`, owned here because `Stream` owns its source).
pub(super) const FINESTRA: usize = 1 << 16;
/// The window after a seek: a scan's header or the centroid list behind a skipped profile need a few KB, not 64 KB of the profile.
pub(super) const FINESTRA_SALTO: usize = 1 << 12;

/// A `ReadAt` seen as `Read + Seek` (what `opentfraw` reads from), with a window that serves the short reads.
pub(super) struct Stream {
    pub(super) inner: Box<dyn ReadAt>,
    pos: u64,
    pub(super) len: u64,
    at: u64,
    buf: Vec<u8>,
    /// The size of the next refill: small after a seek, doubled while the reads go on in order (read-ahead).
    passo: usize,
    /// A seek since the last refill: the reader moved to a header or a list, not along a profile.
    saltato: bool,
}

impl Stream {
    pub(super) fn new(inner: Box<dyn ReadAt>, len: u64) -> Stream {
        Stream {
            inner,
            pos: 0,
            len,
            at: 0,
            buf: Vec::new(),
            passo: FINESTRA_SALTO,
            saltato: true,
        }
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = usize::try_from(self.len.saturating_sub(self.pos))
            .unwrap_or(usize::MAX)
            .min(buf.len());
        let Some(out) = buf.get_mut(..n) else {
            return Ok(0);
        };
        let fine = self.pos + n as u64;
        if n > FINESTRA {
            self.inner
                .read_at(self.pos, out)
                .map_err(|e| std::io::Error::other(e.key))?;
        } else if n > 0 {
            if self.pos < self.at || fine > self.at + self.buf.len() as u64 {
                // reads that go on without a seek grow the window; after a seek it starts small again
                self.passo = if self.saltato {
                    FINESTRA_SALTO
                } else {
                    (self.passo * 2).min(FINESTRA)
                };
                self.saltato = false;
                let w = (self.len - self.pos).min(self.passo.max(n) as u64) as usize;
                self.buf.resize(w, 0);
                self.at = self.pos;
                if let Err(e) = self.inner.read_at(self.pos, &mut self.buf) {
                    self.buf.clear();
                    return Err(std::io::Error::other(e.key));
                }
            }
            let a = (self.pos - self.at) as usize;
            out.copy_from_slice(&self.buf[a..a + n]);
        }
        self.pos = fine;
        Ok(n)
    }

    /// The opening reads the index and the events a few bytes at a time: one that falls in the window is a copy, without the loop of the default `read_exact`.
    fn read_exact(&mut self, mut buf: &mut [u8]) -> std::io::Result<()> {
        let a = self.pos.wrapping_sub(self.at) as usize;
        if self.pos >= self.at && a <= self.buf.len() && buf.len() <= self.buf.len() - a {
            buf.copy_from_slice(&self.buf[a..a + buf.len()]);
            self.pos += buf.len() as u64;
            return Ok(());
        }
        while !buf.is_empty() {
            match self.read(buf)? {
                0 => return Err(std::io::ErrorKind::UnexpectedEof.into()),
                k => buf = &mut buf[k..],
            }
        }
        Ok(())
    }
}

impl Seek for Stream {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let p = match to {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::End(d) => self.len.checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        self.pos = p.ok_or_else(|| std::io::Error::other("seek before the start"))?;
        self.saltato = true;
        Ok(self.pos)
    }
}
