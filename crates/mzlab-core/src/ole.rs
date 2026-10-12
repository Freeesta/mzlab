//! Minimal reader of OLE2 compound files (the container of Analyst `.wiff` and `.dam` files): `mzlab/reader/ole.py`.
//! Only what is needed to list and read streams; the whole file is held in memory (a method is a few KB).

use crate::error::{Error, Result};

const END: u32 = 0xFFFF_FFFE;
const FREE: u32 = 0xFFFF_FFFF;

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
    d.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(bad)
}

fn u32_at(d: &[u8], at: usize) -> Result<u32> {
    d.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(bad)
}

fn bad() -> Error {
    Error::new("err.dam.unreadable").with("message", "truncated or damaged OLE2 file")
}

impl Ole {
    /// `name` is the file name, only for the error of a file that is not OLE2.
    pub fn open(data: Vec<u8>, name: &str) -> Result<Ole> {
        if data.len() < 8 || data[..8] != [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] {
            return Err(Error::new("err.file.notDam").with("name", name));
        }
        let ss = 1usize << u16_at(&data, 0x1E)?.min(24);
        let mss = 1usize << u16_at(&data, 0x20)?.min(24);
        let (n_fat, dir0, cutoff) = (u32_at(&data, 0x2C)?, u32_at(&data, 0x30)?, u32_at(&data, 0x38)?);
        let (mfat0, dif0, n_dif) = (u32_at(&data, 0x3C)?, u32_at(&data, 0x44)?, u32_at(&data, 0x48)?);
        let mut o = Ole { data, ss, mss, cutoff, fat: vec![], dir: vec![], ministream: vec![], minifat: vec![], streams: vec![] };
        let mut difat: Vec<u32> = (0..109).map(|i| u32_at(&o.data, 0x4C + 4 * i)).collect::<Result<_>>()?;
        let mut s = dif0;
        for _ in 0..n_dif {
            let sec = o.sector(s);
            for i in 0..ss / 4 - 1 {
                difat.push(u32_at(&sec, 4 * i)?);
            }
            s = u32_at(&sec, ss - 4)?;
        }
        for &sid in difat.iter().take(n_fat as usize) {
            if sid == END || sid == FREE {
                continue;
            }
            let sec = o.sector(sid);
            for i in 0..ss / 4 {
                o.fat.push(u32_at(&sec, 4 * i)?);
            }
        }
        o.dir = o.parse_dir(dir0);
        let root = o.dir.first().cloned().ok_or_else(bad)?;
        if root.start < END {
            o.ministream = o.chain_bytes(root.start);
        }
        if mfat0 < END {
            let raw = o.chain_bytes(mfat0);
            o.minifat = (0..raw.len() / 4).map(|i| u32_at(&raw, 4 * i)).collect::<Result<_>>()?;
        }
        o.streams = o.paths();
        Ok(o)
    }

    fn sector(&self, sid: u32) -> Vec<u8> {
        let off = (sid as usize + 1).saturating_mul(self.ss);
        let a = off.min(self.data.len());
        self.data[a..off.saturating_add(self.ss).min(self.data.len())].to_vec()
    }

    fn chain_bytes(&self, start: u32) -> Vec<u8> {
        let (mut out, mut sid, mut seen) = (Vec::new(), start, 0);
        while sid < END && seen < self.fat.len() + 1 {
            out.extend(self.sector(sid));
            sid = self.fat.get(sid as usize).copied().unwrap_or(END);
            seen += 1;
        }
        out
    }

    fn parse_dir(&self, start: u32) -> Vec<Entry> {
        let raw = self.chain_bytes(start);
        (0..raw.len() / 128)
            .map(|i| {
                let e = &raw[i * 128..(i + 1) * 128];
                let nlen = u16::from_le_bytes([e[0x40], e[0x41]]) as usize;
                let units: Vec<u16> = e[..nlen.saturating_sub(2).min(64)].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                let w = |a: usize| u32::from_le_bytes([e[a], e[a + 1], e[a + 2], e[a + 3]]);
                Entry { name: String::from_utf16_lossy(&units), typ: e[0x42], left: w(0x44), right: w(0x48), child: w(0x4C), start: w(0x74), size: w(0x78) }
            })
            .collect()
    }

    fn paths(&self) -> Vec<(String, usize)> {
        fn walk(o: &Ole, idx: u32, prefix: &[String], seen: &mut Vec<u32>, out: &mut Vec<(String, usize)>) {
            if idx as usize >= o.dir.len() || seen.contains(&idx) {
                return;
            }
            seen.push(idx);
            let e = &o.dir[idx as usize];
            walk(o, e.left, prefix, seen, out);
            let mut p = prefix.to_vec();
            if !e.name.is_empty() {
                p.push(e.name.clone());
            }
            if e.typ == 2 {
                out.push((p.join("/"), idx as usize));
            } else if e.typ == 1 {
                walk(o, e.child, &p, seen, out);
            }
            walk(o, e.right, prefix, seen, out);
        }
        let mut out = Vec::new();
        walk(self, self.dir[0].child, &[], &mut Vec::new(), &mut out);
        out
    }

    /// Paths of all the streams.
    pub fn paths_list(&self) -> impl Iterator<Item = &str> {
        self.streams.iter().map(|(p, _)| p.as_str())
    }

    pub fn has(&self, path: &str) -> bool {
        self.streams.iter().any(|(p, _)| p == path)
    }

    /// The bytes of a stream (`None` when the path is not one). A later stream of the same path wins, as in a Python dict.
    pub fn read(&self, path: &str) -> Option<Vec<u8>> {
        let e = &self.dir[self.streams.iter().rev().find(|(p, _)| p == path)?.1];
        let size = e.size as usize;
        if e.size < self.cutoff {
            let (mut out, mut sid, mut n) = (Vec::new(), e.start, 0);
            while sid < END && n < self.minifat.len() + 1 {
                let off = (sid as usize).saturating_mul(self.mss);
                let a = off.min(self.ministream.len());
                out.extend_from_slice(&self.ministream[a..off.saturating_add(self.mss).min(self.ministream.len())]);
                sid = self.minifat.get(sid as usize).copied().unwrap_or(END);
                n += 1;
            }
            out.truncate(size);
            return Some(out);
        }
        let mut b = self.chain_bytes(e.start);
        b.truncate(size);
        Some(b)
    }
}
