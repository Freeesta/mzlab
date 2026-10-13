//! Reader of the `.mzidx` index (format in `docs/agenti/mzidx.md`): header, metadata, scan table and, on request, slices of the arrays.
//! The peaks are never loaded whole: a window of m/z reads only the blocks it needs. Bytes come from a `ReadAt` (a file, a slice, a JS callback).

use crate::error::{Error, Result};
use crate::json::{self, Value};
use crate::lettori::Posizione;

pub const BLOCK: u64 = 16384;
const HEADER: usize = 64;
/// Bit 0 of `flags` (version 2): the rows locate the scans in the members of a vendor file set, not in an mzML.
pub const FLAG_LETTORE: u32 = 1;

/// Bytes at an offset. `&mut self` so that a JS callback or a cache can keep state.
pub trait ReadAt {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()>;
    /// The reader and the sample behind the bytes when they are not an mzML (`sorgente::Scansioni` reads the scans from it): `None` for a file read as it is.
    fn lettore(&mut self) -> Option<(&mut dyn crate::lettori::Lettore, usize)> {
        None
    }
}

impl<T: ReadAt + ?Sized> ReadAt for Box<T> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        (**self).read_at(off, buf)
    }
    fn lettore(&mut self) -> Option<(&mut dyn crate::lettori::Lettore, usize)> {
        (**self).lettore()
    }
}

impl ReadAt for Vec<u8> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        let a = off as usize;
        let src = a
            .checked_add(buf.len())
            .and_then(|b| self.get(a..b))
            .ok_or_else(|| Error::new("error.badIndex"))?;
        buf.copy_from_slice(src);
        Ok(())
    }
}

impl ReadAt for std::fs::File {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        use std::io::{Read, Seek, SeekFrom};
        self.seek(SeekFrom::Start(off))
            .and_then(|_| self.read_exact(buf))
            .map_err(|_| Error::new("error.badIndex"))
    }
}

/// One row of the scan table (`SCAN_COLUMNS`).
#[derive(Debug, Clone, Default)]
pub struct Scan {
    /// Bytes of the spectrum in the mzML (version 1; 0 in the index of a vendor file).
    pub start: u64,
    pub end: u64,
    /// Index of a vendor file (version 2): where the scan lies in the members of the set (`None` when the reader does not say) and its number `k` in `Lettore::indice`.
    pub posizione: Option<Posizione>,
    pub k: Option<usize>,
    pub level: u8,
    pub rt: f64,
    pub polarity: i8,
    pub tic: f64,
    pub precursor: Option<f64>,
    pub collision_energy: Option<f64>,
    pub filter: String,
    pub profile: bool,
    pub parent: Option<i64>,
    pub iso: Option<(f64, f64)>,
    pub act: Option<String>,
    pub res: Option<f64>,
    pub an: Option<String>,
    pub pint: Option<f64>,
    /// ((m/z, activation, collision energy), ...) of the filter string.
    pub path: Option<Vec<(f64, String, f64)>>,
    pub charge: Option<i64>,
    pub native: String,
}

impl Scan {
    /// Centre of the isolation window, else the precursor, else `none` (the value the callers use for "no precursor").
    pub fn prec(&self, none: f64) -> f64 {
        match (self.iso, self.precursor) {
            (Some((a, b)), _) => (a + b) / 2.0,
            (None, Some(p)) if p != 0.0 => p,
            _ => none,
        }
    }

    /// Scan number as the instrument software shows it: the `scan=N` of the id, otherwise the position + 1.
    pub fn number(&self, index: usize) -> i64 {
        if let Some(p) = self.native.find("scan=") {
            let d: String = self.native[p + 5..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(n) = d.parse() {
                return n;
            }
        }
        index as i64 + 1
    }

    /// Name of the TYPE of a scan (`explore.scan_key`): the filter without the precursor mass and the range for the survey and the
    /// product ions, the whole path for MSn.
    pub fn key(&self) -> String {
        let f = self.filter.as_str();
        if self.level <= 1 {
            let t = strip_range(f);
            return if t.is_empty() { "ms1".into() } else { t.into() };
        }
        let pre = match key_prefix(f) {
            Some((a, b)) => format!("{a} {b} "),
            None => String::new(),
        };
        let last = self.path.as_ref().and_then(|p| p.last());
        let act = self
            .act
            .clone()
            .filter(|a| !a.is_empty())
            .or_else(|| last.map(|l| l.1.clone()))
            .unwrap_or_default()
            .to_lowercase();
        match &self.path {
            Some(p) if p.len() > 1 => {
                let ms: Vec<String> = p
                    .iter()
                    .map(|x| format!("{}", x.0.round_ties_even() as i64))
                    .collect();
                format!(
                    "{pre}ms{} {} @{act}{}",
                    self.level,
                    ms.join(" > "),
                    json::fmt_g(p[p.len() - 1].2)
                )
            }
            _ => format!("{pre}ms{} @{act}", self.level)
                .trim_end_matches([' ', '@'])
                .to_string(),
        }
    }
}

/// `re.sub(r"\s*\[[^\]]*\]\s*$", "", f)`: the scan range in square brackets at the end of a filter.
fn strip_range(f: &str) -> &str {
    let t = f.trim_end();
    if t.ends_with(']') {
        if let Some(p) = t.rfind('[') {
            if !t[p + 1..t.len() - 1].contains(']') {
                return t[..p].trim_end();
            }
        }
    }
    f
}

/// `^(\w+)\s+([+-])\s` of the filter: the analyzer and the polarity sign.
fn key_prefix(f: &str) -> Option<(&str, char)> {
    let w = f.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    if w == 0 {
        return None;
    }
    let rest = &f[w..];
    let t = rest.trim_start();
    if t.len() == rest.len() {
        return None;
    }
    let sign = t.chars().next().filter(|c| *c == '+' || *c == '-')?;
    let after = &t[1..];
    if after.chars().next().is_some_and(char::is_whitespace) {
        Some((&f[..w], sign))
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct Section {
    pub level: u8,
    pub f4: bool,
    pub n_scans: usize,
    pub n_peaks: u64,
    pub n_fence: usize,
    off: [u64; 10],
}

const NAMES: [&str; 10] = [
    "rt", "tic", "scan_ids", "sumint", "bp_mz", "bp_inten", "fence", "mz", "inten", "pos",
];

/// One section's arrays that are small enough to keep (per scan).
#[derive(Debug, Clone, Default)]
pub struct ScanArrays {
    pub rt: Vec<f64>,
    pub scan_ids: Vec<i64>,
    pub sumint: Vec<f64>,
    pub bp_mz: Vec<f64>,
    pub bp_inten: Vec<f64>,
}

pub struct Index<R: ReadAt> {
    r: R,
    pub scans: Vec<Scan>,
    pub keys: Vec<(String, usize)>,
    pub sections: Vec<Section>,
    pub source: Value,
    pub run: Value,
    pub(crate) cache: Vec<(usize, u8, u64, Vec<u8>, u64)>, // (section, array, block, bytes, tick of the last use)
    pub(crate) arrays: Vec<Option<ScanArrays>>,
    pub(crate) arrays_tick: Vec<u64>, // tick of the last use of each section's arrays (see idxcache.rs)
}

/// Whether the header is of an index of a vendor file (version 2 with `FLAG_LETTORE`) or of an mzML (version 1); `None` for any other.
fn nativo(h: &[u8]) -> Option<bool> {
    let w = |a: usize| {
        h.get(a..a + 4)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes)
    };
    match (w(8)?, w(12)?) {
        (1, 0) => Some(false),
        (2, FLAG_LETTORE) => Some(true),
        _ => None,
    }
}

/// The JSON of `len` bytes at `off`.
fn read_json<R: ReadAt>(r: &mut R, off: u64, len: u64) -> Result<Value> {
    let mut b = Vec::new();
    b.try_reserve_exact(len as usize)
        .map_err(|_| Error::memory_budget())?;
    b.resize(len as usize, 0);
    r.read_at(off, &mut b)?;
    json::parse(&b)
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

impl<R: ReadAt> Index<R> {
    /// `size` = size of the index file (the header says how long it must be).
    pub fn open(mut r: R, size: u64) -> Result<Self> {
        let bad = || Error::new("error.badIndex");
        let mut h = [0u8; HEADER];
        if size < HEADER as u64 {
            return Err(bad());
        }
        r.read_at(0, &mut h)?;
        if &h[..8] != b"MZIDX001" || u64_at(&h, 48) != size {
            return Err(bad());
        }
        let nativo = nativo(&h).ok_or_else(bad)?;
        let (moff, mlen, soff, slen) = (
            u64_at(&h, 16),
            u64_at(&h, 24),
            u64_at(&h, 32),
            u64_at(&h, 40),
        );
        if moff.saturating_add(mlen) > size || soff.saturating_add(slen) > size {
            return Err(bad());
        }
        let meta = read_json(&mut r, moff, mlen)?;
        let rows = read_json(&mut r, soff, slen)?;
        let scans: Vec<Scan> = rows
            .arr()
            .iter()
            .map(|r| crate::scanrow::scan_of_row(r, nativo))
            .collect::<Option<_>>()
            .ok_or_else(bad)?;
        let mut sections = Vec::new();
        for s in meta.get("sections").ok_or_else(bad)?.arr() {
            let n = |k: &str| s.get(k).and_then(Value::f64).ok_or_else(bad);
            let off = s.get("off").ok_or_else(bad)?;
            let mut o = [0u64; 10];
            for (i, name) in NAMES.iter().enumerate() {
                o[i] = off.get(name).and_then(Value::f64).ok_or_else(bad)? as u64;
            }
            sections.push(Section {
                level: n("level")? as u8,
                f4: s.get("dtype").and_then(Value::str) == Some("<f4"),
                n_scans: n("n_scans")? as usize,
                n_peaks: n("n_peaks")? as u64,
                n_fence: n("n_fence")? as usize,
                off: o,
            });
        }
        let keys = match meta.get("keys") {
            Some(Value::Obj(o)) => o
                .iter()
                .filter_map(|(k, v)| v.f64().map(|i| (k.clone(), i as usize)))
                .collect(),
            _ => return Err(bad()),
        };
        let n = sections.len();
        Ok(Index {
            r,
            scans,
            keys,
            sections,
            source: meta.get("source").cloned().unwrap_or(Value::Null),
            run: meta.get("run").cloned().unwrap_or(Value::Null),
            cache: Vec::new(),
            arrays: vec![None; n],
            arrays_tick: vec![0; n],
        })
    }

    /// Index of the section of a level and polarity (`None` = both), or `None` when the file has no such group.
    pub fn section_of(&self, level: u8, pol: Option<i8>) -> Option<usize> {
        let p = match pol {
            None => "all",
            Some(1) => "+",
            Some(-1) => "-",
            _ => "?",
        };
        let k = format!("{level}/{p}");
        self.keys.iter().find(|(n, _)| *n == k).map(|(_, i)| *i)
    }

    pub fn hr1(&self) -> bool {
        self.run.get("hr1").and_then(Value::bool).unwrap_or(false)
    }

    fn read_f64(&mut self, off: u64, n: usize) -> Result<Vec<f64>> {
        let mut b = Vec::new();
        b.try_reserve_exact(n * 8)
            .map_err(|_| Error::memory_budget())?;
        b.resize(n * 8, 0);
        self.r.read_at(off, &mut b)?;
        Ok(b.chunks_exact(8)
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }

    /// The per-scan arrays of a section, read once.
    pub fn arrays(&mut self, si: usize) -> Result<&ScanArrays> {
        if self.arrays[si].is_none() {
            let s = self.sections[si].clone();
            let n = s.n_scans;
            let rt = self.read_f64(s.off[0], n)?;
            let mut b = vec![0u8; n * 8];
            self.r.read_at(s.off[2], &mut b)?;
            let scan_ids = b
                .chunks_exact(8)
                .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
                .collect();
            let sumint = self.read_f64(s.off[3], n)?;
            let bp_mz = self.read_f64(s.off[4], n)?;
            let bp_inten = self.read_f64(s.off[5], n)?;
            self.arrays[si] = Some(ScanArrays {
                rt,
                scan_ids,
                sumint,
                bp_mz,
                bp_inten,
            });
        }
        Ok(self.touch_arrays(si))
    }

    /// Element `i` of the m/z array (as `f64`; the `f32` of a high-resolution survey is widened exactly), through the block cache.
    fn mz_at(&mut self, si: usize, i: u64) -> Result<f64> {
        let s = &self.sections[si];
        let (w, off) = (if s.f4 { 4 } else { 8 }, s.off[7]);
        let blk = i / BLOCK;
        let n = s.n_peaks;
        let at = self
            .cache
            .iter()
            .position(|c| c.0 == si && c.1 == 7 && c.2 == blk);
        let at = match at {
            Some(p) => p,
            None => {
                let first = blk * BLOCK;
                let cnt = (n - first).min(BLOCK) as usize;
                let mut b = vec![0u8; cnt * w];
                self.r.read_at(off + first * w as u64, &mut b)?;
                if self.cache.len() >= 8 {
                    self.cache.remove(0);
                }
                self.cache.push((si, 7, blk, b, 0));
                self.cache.len() - 1
            }
        };
        let b = self.touch_block(at);
        let k = (i - blk * BLOCK) as usize * w;
        Ok(if w == 4 {
            f32::from_le_bytes(b[k..k + 4].try_into().unwrap()) as f64
        } else {
            f64::from_le_bytes(b[k..k + 8].try_into().unwrap())
        })
    }

    /// Lowest and highest m/z of a section (`mz[0]`, `mz[-1]` of its peak table), None when it has no peaks.
    pub fn mz_range(&mut self, si: usize) -> Result<Option<(f64, f64)>> {
        let n = self.sections[si].n_peaks;
        if n == 0 {
            return Ok(None);
        }
        Ok(Some((self.mz_at(si, 0)?, self.mz_at(si, n - 1)?)))
    }

    /// `np.searchsorted(mz, v, side)` where `v` has the dtype of the array (a high-resolution survey compares in `f32`).
    pub fn search(&mut self, si: usize, v: f64, right: bool) -> Result<u64> {
        let v = if self.sections[si].f4 {
            v as f32 as f64
        } else {
            v
        };
        let (mut lo, mut hi) = (0u64, self.sections[si].n_peaks);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let x = self.mz_at(si, mid)?;
            if if right { x <= v } else { x < v } {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        Ok(lo)
    }

    /// `pos` and `inten` of the peaks `a..b`, handed to `f` in pieces of at most 64 K peaks, in table order.
    pub fn each_peak(
        &mut self,
        si: usize,
        a: u64,
        b: u64,
        mut f: impl FnMut(usize, f64, f64),
    ) -> Result<()> {
        const STEP: u64 = 1 << 16;
        let s = self.sections[si].clone();
        let w = if s.f4 { 4 } else { 8 } as u64;
        let mut i = a;
        while i < b {
            let c = (b - i).min(STEP) as usize;
            let mut ib = vec![0u8; c * w as usize];
            let mut mb = vec![0u8; c * w as usize];
            let mut pb = vec![0u8; c * 4];
            self.r.read_at(s.off[8] + i * w, &mut ib)?;
            self.r.read_at(s.off[7] + i * w, &mut mb)?;
            self.r.read_at(s.off[9] + i * 4, &mut pb)?;
            for j in 0..c {
                let (it, mz) = if w == 4 {
                    (
                        f32::from_le_bytes(ib[j * 4..j * 4 + 4].try_into().unwrap()) as f64,
                        f32::from_le_bytes(mb[j * 4..j * 4 + 4].try_into().unwrap()) as f64,
                    )
                } else {
                    (
                        f64::from_le_bytes(ib[j * 8..j * 8 + 8].try_into().unwrap()),
                        f64::from_le_bytes(mb[j * 8..j * 8 + 8].try_into().unwrap()),
                    )
                };
                let pos = i32::from_le_bytes(pb[j * 4..j * 4 + 4].try_into().unwrap()) as usize;
                f(pos, mz, it);
            }
            i += c as u64;
        }
        Ok(())
    }

    /// Summed intensity of the peaks `a..b` in every scan of the section (`np.bincount(pos, weights=inten, minlength=n)`).
    pub fn sum_window(&mut self, si: usize, a: u64, b: u64) -> Result<Vec<f64>> {
        let n = self.sections[si].n_scans;
        let mut out = vec![0.0; n];
        self.each_peak(si, a, b, |p, _mz, y| {
            if p < n {
                out[p] += y;
            }
        })?;
        Ok(out)
    }

    /// Highest intensity of the peaks `a..b` in every scan (0 where there are none).
    pub fn max_window(&mut self, si: usize, a: u64, b: u64) -> Result<Vec<f64>> {
        let n = self.sections[si].n_scans;
        let mut out = vec![0.0; n];
        self.each_peak(si, a, b, |p, _mz, y| {
            if p < n && y > out[p] {
                out[p] = y;
            }
        })?;
        Ok(out)
    }
}
