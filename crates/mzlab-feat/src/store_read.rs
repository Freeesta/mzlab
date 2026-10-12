//! Reading of the containers of `store`: by offsets, one column at a time, with the rows and columns a reader asks for.

use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::json::{parse, Value};

use crate::store::{bad, ALIGN, MAGIC, PIECE};
use crate::table::{Aligned, FileFeatures};

/// Bytes in memory as a `ReadAt`.
pub struct Slice<'a>(pub &'a [u8]);

impl ReadAt for Slice<'_> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        let a = off as usize;
        let src = a
            .checked_add(buf.len())
            .and_then(|b| self.0.get(a..b))
            .ok_or_else(bad)?;
        buf.copy_from_slice(src);
        Ok(())
    }
}

/// A container opened: its meta and where its data start. The arrays are read by offset, one column at a time, never the whole file.
struct Opened {
    meta: Value,
    start: u64,
}

fn open<R: ReadAt>(r: &mut R, size: u64, kind: &str) -> Result<Opened> {
    let mut h = [0u8; 16];
    if size < 16 {
        return Err(bad());
    }
    r.read_at(0, &mut h)?;
    if &h[..8] != MAGIC {
        return Err(bad());
    }
    let n = u64::from_le_bytes(h[8..].try_into().unwrap());
    let end = 16u64.checked_add(n).filter(|&e| e <= size).ok_or_else(bad)?;
    let mut text = mzlab_core::budget::try_vec::<u8>(n as usize)?;
    text.resize(n as usize, 0);
    r.read_at(16, &mut text)?;
    let meta = parse(&text)?;
    if meta.get("format").and_then(|v| v.str()) != Some(kind) {
        return Err(bad());
    }
    Ok(Opened {
        meta,
        start: end.div_ceil(ALIGN as u64) * ALIGN as u64,
    })
}

impl Opened {
    /// `(count, offset)` of the array `name` when it has `dtype`.
    fn entry(&self, name: &str, dtype: &str) -> Result<(usize, u64)> {
        let e = self.meta.get("arrays").map(|a| a.arr()).unwrap_or(&[]);
        let e = e
            .iter()
            .find(|x| x.get("name").and_then(|v| v.str()) == Some(name))
            .ok_or_else(bad)?;
        let get = |k: &str| e.get(k).and_then(|v| v.f64()).map(|v| v as usize).ok_or_else(bad);
        if e.get("dtype").and_then(|v| v.str()) != Some(dtype) {
            return Err(bad());
        }
        Ok((get("count")?, self.start + get("offset")? as u64))
    }

    /// Rows `rows` (all when `None`) of the array `name`, decoded `N` bytes at a time and read in pieces.
    fn col<R: ReadAt, T, const N: usize>(
        &self,
        r: &mut R,
        size: u64,
        name: &str,
        dtype: &str,
        rows: Option<(usize, usize)>,
        dec: fn([u8; N]) -> T,
    ) -> Result<Vec<T>> {
        let (count, off) = self.entry(name, dtype)?;
        let (a, b) = rows.map_or((0, count), |(a, b)| (a.min(count), b.min(count)));
        let end = off.checked_add((count * N) as u64).ok_or_else(bad)?;
        if end > size || b < a {
            return Err(bad());
        }
        let mut out = mzlab_core::budget::try_vec::<T>(b - a)?;
        let mut buf = vec![0u8; PIECE * N];
        let mut at = a;
        while at < b {
            let k = (b - at).min(PIECE);
            r.read_at(off + (at * N) as u64, &mut buf[..k * N])?;
            out.extend(buf[..k * N].chunks_exact(N).map(|c| dec(c.try_into().unwrap())));
            at += k;
        }
        Ok(out)
    }
}

/// Which columns of a `.mzfeatf` a reader wants and which rows (a half-open range of m/z; the file is sorted by m/z).
#[derive(Clone, Copy)]
pub struct Want {
    /// Only what the alignment reads (m/z, RT, limits, area, height, flags, charge, role, family): 39 bytes per row instead of 91.
    pub lean: bool,
    pub mz: Option<(f64, f64)>,
}

impl Want {
    pub const ALL: Want = Want {
        lean: false,
        mz: None,
    };
}

macro_rules! cols {
    ($o:ident, $r:ident, $n:ident, $rows:ident; $($f:ident: $ty:ident),*) => {
        FileFeatures { $( $f: cols!(@one $o, $r, $n, $rows, $f, $ty), )* ..FileFeatures::default() }
    };
    (@one $o:ident, $r:ident, $n:ident, $rows:ident, $f:ident, f64) => { $o.col($r, $n, stringify!($f), "f64", $rows, f64::from_le_bytes)? };
    (@one $o:ident, $r:ident, $n:ident, $rows:ident, $f:ident, f32) => { $o.col($r, $n, stringify!($f), "f32", $rows, f32::from_le_bytes)? };
    (@one $o:ident, $r:ident, $n:ident, $rows:ident, $f:ident, u32) => { $o.col($r, $n, stringify!($f), "u32", $rows, u32::from_le_bytes)? };
    (@one $o:ident, $r:ident, $n:ident, $rows:ident, $f:ident, u16) => { $o.col($r, $n, stringify!($f), "u16", $rows, u16::from_le_bytes)? };
    (@one $o:ident, $r:ident, $n:ident, $rows:ident, $f:ident, u8) => { $o.col($r, $n, stringify!($f), "u8", $rows, |b: [u8; 1]| b[0])? };
    (@one $o:ident, $r:ident, $n:ident, $rows:ident, $f:ident, i8) => { $o.col($r, $n, stringify!($f), "i8", $rows, |b: [u8; 1]| b[0] as i8)? };
}

/// Reads a `.mzfeatf` by offsets: the table and its meta. With `want.mz` only the rows with m/z in the range; with `want.lean` only the columns of the
/// alignment (the others stay empty, so such a table is for `align` and nothing else).
pub fn read_file_from<R: ReadAt>(r: &mut R, size: u64, want: Want) -> Result<(FileFeatures, Value)> {
    let o = open(r, size, "file")?;
    let rows = match want.mz {
        None => None,
        Some((lo, hi)) => {
            let mz = o.col(r, size, "mz", "f64", None, f64::from_le_bytes)?;
            Some((mz.partition_point(|&m| m < lo), mz.partition_point(|&m| m < hi)))
        }
    };
    let mut f = if want.lean {
        cols!(o, r, size, rows; mz: f64, rt: f32, left: f32, right: f32, area: f64, height: f32, flags: u8, charge: i8, family_id: u32, role: u8)
    } else {
        cols!(o, r, size, rows; mz: f64, rt: f32, left: f32, right: f32, apex: u32, scan_lo: u32, scan_hi: u32, area: f64, height: f32, npts: u16,
            flags: u8, iso_group: u32, charge: i8, shape: f32, snr: f32, fwhm: f32, asym: f32, gaps: u16, qreason: u8, ms2_n: u16, purity: f32,
            area_win: f64, family_id: u32, role: u8, alt: u8)
    };
    f.dt = o
        .col(r, size, "dt", "f64", None, f64::from_le_bytes)?
        .first()
        .copied()
        .ok_or_else(bad)?;
    Ok((f, o.meta))
}

/// The meta of a `.mzfeatf` (name and SHA-256 of the inputs, recipe) and its m/z column, without anything else.
pub fn read_meta_mz(bytes: &[u8]) -> Result<(Value, Vec<f64>)> {
    let (mut r, size) = (Slice(bytes), bytes.len() as u64);
    let o = open(&mut r, size, "file")?;
    let mz = o.col(&mut r, size, "mz", "f64", None, f64::from_le_bytes)?;
    Ok((o.meta, mz))
}

/// Reads a `.mzfeatf`: the table and its meta.
pub fn read_file(bytes: &[u8]) -> Result<(FileFeatures, Value)> {
    read_file_from(&mut Slice(bytes), bytes.len() as u64, Want::ALL)
}

/// Reads a `.mzfeat` by offsets: the table and its meta.
pub fn read_table_from<R: ReadAt>(r: &mut R, size: u64) -> Result<(Aligned, Value)> {
    let o = open(r, size, "table")?;
    let n_files = o.meta.get("files").and_then(|v| v.f64()).ok_or_else(bad)? as usize;
    let rows = None;
    let t = Aligned {
        n_files,
        mz: o.col(r, size, "mz", "f64", rows, f64::from_le_bytes)?,
        rt: o.col(r, size, "rt", "f64", rows, f64::from_le_bytes)?,
        area: o.col(r, size, "area", "f64", rows, f64::from_le_bytes)?,
        height: o.col(r, size, "height", "f32", rows, f32::from_le_bytes)?,
        apex_rt: o.col(r, size, "apex_rt", "f32", rows, f32::from_le_bytes)?,
        width: o.col(r, size, "width", "f32", rows, f32::from_le_bytes)?,
        state: o.col(r, size, "state", "u8", rows, |b: [u8; 1]| b[0])?,
        member_start: o.col(r, size, "member_start", "u32", rows, u32::from_le_bytes)?,
        member_mz: o.col(r, size, "member_mz", "f64", rows, f64::from_le_bytes)?,
        member_rt: o.col(r, size, "member_rt", "f32", rows, f32::from_le_bytes)?,
        member_role: o.col(r, size, "member_role", "u8", rows, |b: [u8; 1]| b[0])?,
    };
    Ok((t, o.meta))
}

/// Reads a `.mzfeat`: the table and its meta.
pub fn read_table(bytes: &[u8]) -> Result<(Aligned, Value)> {
    read_table_from(&mut Slice(bytes), bytes.len() as u64)
}

/// The `evid` section of a `.mzfeat` of a series: the JSON bytes of the graph of the proofs (empty when none was written).
pub fn read_evid(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut r = Slice(bytes);
    let size = bytes.len() as u64;
    open(&mut r, size, "table")?.col(&mut r, size, "evid", "u8", None, |b: [u8; 1]| b[0])
}
