//! Minimal reader of OLE2 compound files (the container of Analyst `.wiff` and `.dam` files): `mzlab/reader/ole.py`.
//! Only what is needed to list and read streams; the whole file is held in memory (a method is a few KB). Every chain and walk is bounded by the bytes
//! present and by `Limits` (bytes, directory entries, storage depth), so a damaged or hostile header costs at most a linear pass over the file.

use crate::error::{Error, Result};
use crate::readers::Limits;

const END: u32 = 0xFFFF_FFFE;
const FREE: u32 = 0xFFFF_FFFF;

impl Ole {
    /// `name` is the file name, only for the error of a file that is not OLE2. `limits`: the bytes of the file, the directory entries and the
    /// nesting of storages it may ask for (`error.memoryBudget`, `err.dam.unreadable` past them).
    pub fn open(data: Vec<u8>, name: &str, limits: &Limits) -> Result<Ole> {
        if data.len() as u64 > limits.byte {
            return Err(Error::memory_budget());
        }
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
            data,
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
            .map(|i| u32_at(&o.data, 0x4C + 4 * i))
            .collect::<Result<_>>()?;
        let mut s = dif0;
        // a chain of DIFAT sectors is at most as long as the file has sectors, and never visits a sector twice (a loop would grow `difat` for ever)
        let mut visited = std::collections::HashSet::new();
        for _ in 0..(n_dif as usize).min(o.data.len() / ss) {
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
        o.dir = o.parse_dir(dir0, limits)?;
        let root = o.dir.first().cloned().ok_or_else(bad)?;
        if root.start < END {
            o.ministream = o.chain_bytes(root.start, usize::MAX);
        }
        if mfat0 < END {
            let raw = o.chain_bytes(mfat0, usize::MAX);
            o.minifat = (0..raw.len() / 4)
                .map(|i| u32_at(&raw, 4 * i))
                .collect::<Result<_>>()?;
        }
        o.streams = o.paths(limits)?;
        Ok(o)
    }

    fn sector(&self, sid: u32) -> Vec<u8> {
        let off = (sid as usize).saturating_add(1).saturating_mul(self.ss);
        let a = off.min(self.data.len());
        self.data
            .get(a..off.saturating_add(self.ss).min(self.data.len()))
            .unwrap_or_default()
            .to_vec()
    }

    /// The sectors of the chain from `start`, up to `cap` bytes: a chain is at most as long as the file has sectors, so a loop ends after one pass.
    fn chain_bytes(&self, start: u32, cap: usize) -> Vec<u8> {
        let (mut out, mut sid, mut seen) = (Vec::new(), start, 0);
        while sid < END && out.len() < cap && seen <= self.fat.len().min(self.data.len() / self.ss)
        {
            out.extend(self.sector(sid));
            sid = self.fat.get(sid as usize).copied().unwrap_or(END);
            seen += 1;
        }
        out
    }

    fn parse_dir(&self, start: u32, limits: &Limits) -> Result<Vec<Entry>> {
        let raw = self.chain_bytes(start, usize::MAX);
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

    /// The bytes the header of an OLE2 file `data` points into (its directory, mini FAT, DIFAT and first FAT sectors), when they lie past its end: a
    /// copy cut before `open` can follow its directory. `None` for a file that is not OLE2 or whose header sectors are all there.
    pub fn header_cut(data: &[u8]) -> Option<u64> {
        if data.get(..8) != Some(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1][..]) {
            return None;
        }
        let ss = match u16_at(data, 0x1E).ok()? {
            9 => 512u64,
            12 => 4096,
            _ => return None,
        };
        let n_fat = u32_at(data, 0x2C).ok()? as usize;
        let fat = (0..n_fat.min(109)).filter_map(|i| u32_at(data, 0x4C + 4 * i).ok());
        let sid = [0x30, 0x3C, 0x44]
            .into_iter()
            .filter_map(|o| u32_at(data, o).ok())
            .chain(fat)
            .filter(|&s| s < END)
            .max()?;
        let need = (sid as u64 + 2).saturating_mul(ss);
        (need > data.len() as u64).then_some(need)
    }

    /// The bytes the file needs to hold every sector its FAT marks as used, when it is shorter (a copy cut short): the streams of the missing
    /// sectors would read empty, so a container that lists no sample is told as cut, not as empty.
    pub fn cut_short(&self) -> Option<u64> {
        let used = self.fat.iter().rposition(|&v| v != FREE)?;
        let need = (used as u64 + 2).saturating_mul(self.ss as u64);
        (need > self.data.len() as u64 + self.ss as u64 - 1).then_some(need)
    }

    /// SHA-1 (lower-case hex) of the whole container file.
    pub fn sha1_hex(&self) -> String {
        crate::method::sha1_hex_bytes(&self.data)
    }

    pub fn has(&self, path: &str) -> bool {
        self.streams.iter().any(|(p, _)| p == path)
    }

    /// The bytes of a stream (`None` when the path is not one). A later stream of the same path wins, as in a Python dict.
    pub fn read(&self, path: &str) -> Option<Vec<u8>> {
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

pub struct Ole {
    data: Vec<u8>,
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
