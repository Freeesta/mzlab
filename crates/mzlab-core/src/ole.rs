//! Minimal reader of OLE2 compound files (the container of Analyst `.wiff` and `.dam` files): `mzlab/reader/ole.py`.
//! Only what is needed to list and read streams; the file is held in memory (`open`) or read by sector on demand (`open_at`: a `.wiff` of hundreds of MB
//! whose streams are mostly a detector trace nobody asked for; only the FAT, directory and mini stream are held). Every chain and walk is bounded by the bytes
//! present and by `Limits` (bytes, directory entries, storage depth), so a damaged or hostile header costs at most a linear pass over the file.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::readers::Limits;

const END: u32 = 0xFFFF_FFFE;
const FREE: u32 = 0xFFFF_FFFF;

impl Ole {
    /// `name` is the file name, only for the error of a file that is not OLE2. `limits`: the bytes of the file, the directory entries and the
    /// nesting of storages it may ask for (`error.fixedLimit`, `err.dam.unreadable` past them).
    pub fn open(data: Vec<u8>, name: &str, limits: &Limits) -> Result<Ole> {
        if data.len() as u64 > limits.byte {
            return Err(Error::fixed_limit("OLE file", limits.byte));
        }
        let len = data.len() as u64;
        Self::build(
            Src(std::cell::RefCell::new(Box::new(data)), len),
            name,
            limits,
        )
    }

    /// `open` over `len` bytes read at offsets: only the FAT, the directory, the mini FAT and the mini stream are held (bounded by `limits.byte`), a stream is
    /// read when it is asked for. The same errors and the same stream bytes as `open` of the whole file.
    pub fn open_at(src: Box<dyn ReadAt>, len: u64, name: &str, limits: &Limits) -> Result<Ole> {
        Self::build(Src(std::cell::RefCell::new(src), len), name, limits)
    }

    fn build(src: Src, name: &str, limits: &Limits) -> Result<Ole> {
        let data = src.read(0, 512);
        if !matches!(
            data.get(..8),
            Some([0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1])
        ) {
            return Err(Error::new("err.file.notDam").with("name", name));
        }
        // the standard allows sectors of 512 or 4096 bytes and mini sectors of 64: anything else is a damaged or hostile header
        let (ss_shift, mss_shift) = (u16_at(&data, 0x1E)?, u16_at(&data, 0x20)?);
        if !matches!(ss_shift, 9 | 12) || mss_shift != 6 {
            return Err(Error::new("err.file.notDam").with("name", name));
        }
        let (ss, mss) = (1usize << ss_shift, 1usize << mss_shift);
        let (n_fat, dir0, cutoff) = (
            u32_at(&data, 0x2C)?,
            u32_at(&data, 0x30)?,
            u32_at(&data, 0x38)?,
        );
        let (mfat0, dif0, n_dif) = (
            u32_at(&data, 0x3C)?,
            u32_at(&data, 0x44)?,
            u32_at(&data, 0x48)?,
        );
        let mut o = Ole {
            src,
            sha1: Default::default(),
            last_big: Default::default(),
            ss,
            mss,
            cutoff,
            fat: vec![],
            dir: vec![],
            ministream: vec![],
            minifat: vec![],
            streams: vec![],
        };
        let mut difat: Vec<u32> = (0..109)
            .map(|i| u32_at(&data, 0x4C + 4 * i))
            .collect::<Result<_>>()?;
        let mut s = dif0;
        // a chain of DIFAT sectors is at most as long as the file has sectors, and never visits a sector twice (a loop would grow `difat` for ever)
        let mut visited = std::collections::HashSet::new();
        for _ in 0..(n_dif as usize).min(o.len() / ss) {
            if s >= END || !visited.insert(s) {
                break;
            }
            let sec = o.sector(s);
            for i in 0..ss / 4 - 1 {
                difat.push(u32_at(&sec, 4 * i)?);
            }
            s = u32_at(&sec, ss - 4)?;
        }
        // a FAT sector is listed once: listing one again would make the FAT as large as the DIFAT, which the file does not bound
        let mut fat_sectors = std::collections::HashSet::new();
        for &sid in difat.iter().take(n_fat as usize) {
            if sid == END || sid == FREE || !fat_sectors.insert(sid) {
                continue;
            }
            let sec = o.sector(sid);
            for i in 0..ss / 4 {
                o.fat.push(u32_at(&sec, 4 * i)?);
            }
        }
        let held = usize::try_from(limits.byte).unwrap_or(usize::MAX);
        o.dir = o.parse_dir(dir0, limits)?;
        let root = o.dir.first().cloned().ok_or_else(bad)?;
        if root.start < END {
            o.ministream = o.chain_bytes(root.start, held);
        }
        if mfat0 < END {
            let raw = o.chain_bytes(mfat0, held);
            o.minifat = (0..raw.len() / 4)
                .map(|i| u32_at(&raw, 4 * i))
                .collect::<Result<_>>()?;
        }
        o.streams = o.paths(limits)?;
        Ok(o)
    }

    fn sector(&self, sid: u32) -> Vec<u8> {
        let off = (sid as u64)
            .saturating_add(1)
            .saturating_mul(self.ss as u64);
        self.src.read(off, self.ss)
    }

    fn len(&self) -> usize {
        usize::try_from(self.src.len()).unwrap_or(usize::MAX)
    }

    /// The sectors of the chain from `start`, up to `cap` bytes: a chain is at most as long as the file has sectors, so a loop ends after one pass.
    fn chain_bytes(&self, start: u32, cap: usize) -> Vec<u8> {
        let (mut out, mut sid, mut seen) = (Vec::new(), start, 0);
        while sid < END && out.len() < cap && seen <= self.fat.len().min(self.len() / self.ss) {
            out.extend(self.sector(sid));
            sid = self.fat.get(sid as usize).copied().unwrap_or(END);
            seen += 1;
        }
        out
    }

    fn parse_dir(&self, start: u32, limits: &Limits) -> Result<Vec<Entry>> {
        let raw = self.chain_bytes(start, usize::try_from(limits.byte).unwrap_or(usize::MAX));
        limits.ok_entries((raw.len() / 128) as u64)?;
        Ok(raw
            .chunks_exact(128)
            .map(|e| {
                let b = |a: usize| e.get(a).copied().unwrap_or(0);
                let nlen = u16::from_le_bytes([b(0x40), b(0x41)]) as usize;
                let units: Vec<u16> = e
                    .get(..nlen.saturating_sub(2).min(64))
                    .unwrap_or_default()
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes(c.try_into().unwrap_or_default()))
                    .collect();
                let w = |a: usize| u32::from_le_bytes([b(a), b(a + 1), b(a + 2), b(a + 3)]);
                Entry {
                    name: String::from_utf16_lossy(&units),
                    typ: b(0x42),
                    left: w(0x44),
                    right: w(0x48),
                    child: w(0x4C),
                    start: w(0x74),
                    size: w(0x78),
                }
            })
            .collect())
    }

    /// `(path, directory index)` of every stream, in the order of an in-order walk of the directory tree (left, entry, children, right). Iterative, with
    /// each entry visited once: a directory that loops, or a chain of 131 000 entries, costs neither stack nor quadratic time. Storages nested deeper
    /// than `limits.profondita` are refused: each level copies the path above it, so an unbounded nesting would cost the square of the entries.
    fn paths(&self, limits: &Limits) -> Result<Vec<(String, usize)>> {
        enum Todo {
            Visit(u32, Vec<String>),
            Emit(u32, Vec<String>),
            Right(u32, Vec<String>),
        }
        let mut seen = vec![false; self.dir.len()];
        let mut out = Vec::new();
        let mut stack = vec![Todo::Visit(
            self.dir.first().map_or(END, |e| e.child),
            Vec::new(),
        )];
        while let Some(t) = stack.pop() {
            match t {
                Todo::Visit(idx, prefix) => {
                    let (Some(e), Some(done)) =
                        (self.dir.get(idx as usize), seen.get_mut(idx as usize))
                    else {
                        continue;
                    };
                    if std::mem::replace(done, true) {
                        continue;
                    }
                    // pushed in reverse: left first, then the entry, then its right sibling
                    stack.push(Todo::Right(e.right, prefix.clone()));
                    stack.push(Todo::Emit(idx, prefix.clone()));
                    stack.push(Todo::Visit(e.left, prefix));
                }
                Todo::Emit(idx, prefix) => {
                    let Some(e) = self.dir.get(idx as usize) else {
                        continue;
                    };
                    let mut p = prefix;
                    if !e.name.is_empty() {
                        p.push(e.name.clone());
                    }
                    if e.typ == 2 {
                        out.push((p.join("/"), idx as usize));
                    } else if e.typ == 1 {
                        limits.profondita_ok(p.len())?;
                        stack.push(Todo::Visit(e.child, p));
                    }
                }
                Todo::Right(idx, prefix) => stack.push(Todo::Visit(idx, prefix)),
            }
        }
        Ok(out)
    }

    /// Paths of all the streams.
    pub fn paths_list(&self) -> impl Iterator<Item = &str> {
        self.streams.iter().map(|(p, _)| p.as_str())
    }

    /// The bytes the header of an OLE2 file `data` points into (its directory, mini FAT, DIFAT chain and every FAT sector the header and the DIFAT list),
    /// when they lie past its end: a copy cut before `open` can follow its directory. `None` for a file without the OLE2 magic or whose header sectors are
    /// all there. A file with the magic but shorter than its header is cut too. The DIFAT walk is bounded as in `open`: by the DIFAT count, by the sectors
    /// the file holds and by never visiting a sector twice, so a hostile header costs one linear pass.
    pub fn header_cut(data: &[u8]) -> Option<u64> {
        if data.get(..8) != Some(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1][..]) {
            return None;
        }
        const HEADER: u64 = 512;
        let Ok(shift) = u16_at(data, 0x1E) else {
            return Some(HEADER);
        };
        let ss: usize = match shift {
            9 => 512,
            12 => 4096,
            _ => return None,
        };
        let len = data.len();
        // the header sector itself: its 109 FAT slots end at 0x200, so a shorter file lacks part of what it points to
        if (len as u64) < HEADER {
            return Some(HEADER);
        }
        let word = |o: usize| u32_at(data, o).ok();
        let (n_fat, n_dif) = (word(0x2C)? as usize, word(0x48)? as usize);
        let mut top = [0x30, 0x3C, 0x44]
            .into_iter()
            .filter_map(word)
            .filter(|&s| s < END)
            .max();
        let mut note = |sid: u32| {
            if sid < END {
                top = top.max(Some(sid));
            }
        };
        let mut left = n_fat; // FAT sectors still to list; `open` takes the first `n_fat` of the DIFAT in the same way
        for i in 0..left.min(109) {
            if let Some(sid) = word(0x4C + 4 * i) {
                note(sid);
            }
        }
        left = left.saturating_sub(109);
        let (mut s, mut visited) = (word(0x44)?, std::collections::HashSet::new());
        for _ in 0..n_dif.min(len / ss) {
            if s >= END || left == 0 || !visited.insert(s) {
                break;
            }
            note(s);
            let off = (s as usize).saturating_add(1).saturating_mul(ss);
            let Some(sec) = data.get(off..off.saturating_add(ss)) else {
                break; // the DIFAT sector is not all there: its own end is what is needed, and the entries after it are unknown
            };
            for i in 0..(ss / 4 - 1).min(left) {
                if let Ok(sid) = u32_at(sec, 4 * i) {
                    note(sid);
                }
            }
            left = left.saturating_sub(ss / 4 - 1);
            s = u32_at(sec, ss - 4).unwrap_or(END);
        }
        let need = (u64::from(top?) + 2).saturating_mul(ss as u64);
        (need > len as u64).then_some(need)
    }

    /// The bytes the file needs to hold every sector its FAT marks as used, when it is shorter (a copy cut short): the streams of the missing
    /// sectors would read empty, so a container that lists no sample is told as cut, not as empty.
    pub fn cut_short(&self) -> Option<u64> {
        let used = self.fat.iter().rposition(|&v| v != FREE)?;
        let need = (used as u64 + 2).saturating_mul(self.ss as u64);
        (need > self.src.len() + self.ss as u64 - 1).then_some(need)
    }

    /// Takes the SHA-1 of the whole file (lower-case hex) from the caller, who already has it, instead of reading the file for it.
    pub fn with_sha1(self, sha1: Option<String>) -> Ole {
        *self.sha1.borrow_mut() = sha1;
        self
    }

    /// Bytes of the container file.
    pub fn byte_len(&self) -> u64 {
        self.src.len()
    }

    /// SHA-1 (lower-case hex) of the whole container file.
    pub fn sha1_hex(&self) -> String {
        if let Some(h) = self.sha1.borrow().as_ref() {
            return h.clone();
        }
        let h =
            crate::method::sha1_dyn(&mut **self.src.0.borrow_mut(), self.src.1).unwrap_or_default();
        *self.sha1.borrow_mut() = Some(h.clone());
        h
    }

    pub fn has(&self, path: &str) -> bool {
        self.streams.iter().any(|(p, _)| p == path)
    }

    /// The bytes of a stream (`None` when the path is not one). A later stream of the same path wins, as in a Python dict.
    pub fn read(&self, path: &str) -> Option<Vec<u8>> {
        if let Some((p, b)) = self.last_big.borrow().as_ref()
            && p == path
        {
            return Some(b.clone());
        }
        let b = self.read_stream(path)?;
        if b.len() >= 1 << 20 {
            *self.last_big.borrow_mut() = Some((path.to_string(), b.clone()));
        }
        Some(b)
    }

    fn read_stream(&self, path: &str) -> Option<Vec<u8>> {
        let e = self
            .dir
            .get(self.streams.iter().rev().find(|(p, _)| p == path)?.1)?;
        let size = e.size as usize;
        if e.size < self.cutoff {
            // the chain stops at the size of the stream: a mini FAT that loops would otherwise copy 64 bytes per entry, 16 times the file
            let (mut out, mut sid, mut n) = (Vec::new(), e.start, 0);
            while sid < END && out.len() < size && n < self.minifat.len() + 1 {
                let off = (sid as usize).saturating_mul(self.mss);
                let a = off.min(self.ministream.len());
                let end = off.saturating_add(self.mss).min(self.ministream.len());
                out.extend_from_slice(self.ministream.get(a..end).unwrap_or_default());
                sid = self.minifat.get(sid as usize).copied().unwrap_or(END);
                n += 1;
            }
            out.truncate(size);
            return Some(out);
        }
        let mut b = self.chain_bytes(e.start, size);
        b.truncate(size);
        Some(b)
    }
}

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
struct Entry {
    name: String,
    typ: u8,
    left: u32,
    right: u32,
    child: u32,
    start: u32,
    size: u32,
}

/// The bytes of the file, read at offsets (a `Vec<u8>` is one too).
struct Src(std::cell::RefCell<Box<dyn ReadAt>>, u64);

impl Src {
    fn len(&self) -> u64 {
        self.1
    }

    /// Up to `n` bytes at `off`, cut at the end of the file (a read that fails gives what a cut file would: nothing).
    #[inline(never)]
    fn read(&self, off: u64, n: usize) -> Vec<u8> {
        let end = off.saturating_add(n as u64).min(self.1);
        if off >= end {
            return Vec::new();
        }
        let mut b = vec![0u8; usize::try_from(end - off).unwrap_or(n)];
        match self.0.borrow_mut().read_at(off, &mut b) {
            Ok(()) => b,
            Err(_) => Vec::new(),
        }
    }
}

pub struct Ole {
    src: Src,
    sha1: std::cell::RefCell<Option<String>>,
    /// The last stream of a megabyte or more that `read` gave: the DAD is asked for by the identity, the losses and the chromatograms of one injection.
    last_big: std::cell::RefCell<Option<(String, Vec<u8>)>>,
    ss: usize,
    mss: usize,
    cutoff: u32,
    fat: Vec<u32>,
    dir: Vec<Entry>,
    ministream: Vec<u8>,
    minifat: Vec<u32>,
    /// Path of every stream (`Dir/Sub/Name`) with its directory entry, in the order of the tree walk.
    streams: Vec<(String, usize)>,
}

fn u16_at(d: &[u8], at: usize) -> Result<u16> {
    d.get(at..at + 2)
        .and_then(|b| b.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(bad)
}

fn u32_at(d: &[u8], at: usize) -> Result<u32> {
    d.get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(bad)
}

fn bad() -> Error {
    Error::new("err.dam.unreadable").with("message", "truncated or damaged OLE2 file")
}
