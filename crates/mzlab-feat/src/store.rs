//! Files of the feature tables: `.mzfeatf` (the table of one file, named `<sha256 of the input>.<fingerprint of the recipe>.mzfeatf`) and `.mzfeat`
//! (the aligned table of a series, `<series>.mzfeat`). Both have the same container, little-endian, versioned by the magic:
//!
//! `MZFEAT01` | meta length (u64) | meta (JSON text) | padding to 64 bytes | arrays, each starting at a multiple of 64 bytes from the end of the padding.
//!
//! The meta carries the recipe and its fingerprints, the version of the crate, the SHA-256 of every input file (from its index), the times (empty when
//! the bytes must be reproducible) and the directory of the arrays (`name`, `dtype`, `count`, `offset`). `evid` is reserved for the evidence of the
//! identification: it is written empty. The bytes depend only on the table and on the meta given: no clock, no path.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

use crate::recipe::Recipe;
use crate::table::{Aligned, FileFeatures};

const MAGIC: &[u8; 8] = b"MZFEAT01";
const ALIGN: usize = 64;

fn bad() -> Error {
    Error::new("error.badMzfeat")
}

fn pad_to(v: &mut Vec<u8>, n: usize) {
    while !v.len().is_multiple_of(n) {
        v.push(0);
    }
}

/// What the meta says about the origin of a table.
#[derive(Debug, Clone, Default)]
pub struct Origin {
    /// `(name, SHA-256)` of every input file, in the order of the files of the table.
    pub inputs: Vec<(String, String)>,
    /// `(step, seconds)`; leave empty to get the same bytes in two runs.
    pub times: Vec<(String, f64)>,
}

/// The arrays of a container, in order of writing.
#[derive(Default)]
struct Arrays {
    dir: Vec<(&'static str, &'static str, usize, usize)>,
    data: Vec<u8>,
}

impl Arrays {
    fn add(&mut self, name: &'static str, dtype: &'static str, count: usize, bytes: Vec<u8>) {
        pad_to(&mut self.data, ALIGN);
        self.dir.push((name, dtype, count, self.data.len()));
        self.data.extend_from_slice(&bytes);
    }
}

macro_rules! adders {
    ($($fn:ident, $t:ty, $name:literal);*) => { $(
        fn $fn(&mut self, name: &'static str, v: &[$t]) {
            self.add(name, $name, v.len(), v.iter().flat_map(|x| x.to_le_bytes()).collect());
        }
    )* };
}

impl Arrays {
    adders!(f64s, f64, "f64"; f32s, f32, "f32"; u32s, u32, "u32"; u16s, u16, "u16");
    fn u8s(&mut self, name: &'static str, v: &[u8]) {
        self.add(name, "u8", v.len(), v.to_vec());
    }
    fn i8s(&mut self, name: &'static str, v: &[i8]) {
        self.add(name, "i8", v.len(), v.iter().map(|&x| x as u8).collect());
    }
}

/// The JSON meta: format, versions, recipe, fingerprints, inputs, times, shape and directory of the arrays.
fn meta_json(kind: &str, r: &Recipe, o: &Origin, shape: &[(&str, usize)], a: &Arrays) -> String {
    let mut j = Out::default();
    let text = if kind == "file" {
        r.file_text()
    } else {
        r.table_text()
    };
    j.raw("{\"format\":")
        .str(kind)
        .raw(",\"version\":1,\"crate\":")
        .str(env!("CARGO_PKG_VERSION"));
    j.raw(",\"feat_version\":")
        .int(crate::FEAT_VERSION as i64)
        .raw(",\"recipe\":")
        .str(&text);
    j.raw(",\"recipe_hash\":")
        .str(&crate::recipe::fingerprint(&text))
        .raw(",\"inputs\":[");
    for (i, (name, sha)) in o.inputs.iter().enumerate() {
        j.raw(if i > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(name)
            .raw(",\"sha256\":")
            .str(sha)
            .raw("}");
    }
    j.raw("],\"times\":{");
    for (i, (step, secs)) in o.times.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" })
            .str(step)
            .raw(":")
            .num(*secs);
    }
    j.raw("}");
    for (k, n) in shape {
        j.raw(",").str(k).raw(":").int(*n as i64);
    }
    j.raw(",\"arrays\":[");
    for (i, (name, dtype, count, off)) in a.dir.iter().enumerate() {
        j.raw(if i > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(name)
            .raw(",\"dtype\":")
            .str(dtype);
        j.raw(",\"count\":")
            .int(*count as i64)
            .raw(",\"offset\":")
            .int(*off as i64)
            .raw("}");
    }
    j.raw("]}");
    j.0
}

fn container(meta: &str, a: &Arrays) -> Vec<u8> {
    let mut b = Vec::with_capacity(a.data.len() + meta.len() + 128);
    b.extend_from_slice(MAGIC);
    b.extend_from_slice(&(meta.len() as u64).to_le_bytes());
    b.extend_from_slice(meta.as_bytes());
    pad_to(&mut b, ALIGN);
    b.extend_from_slice(&a.data);
    b
}

/// Bytes of the `.mzfeatf` of one file.
pub fn write_file(f: &FileFeatures, r: &Recipe, o: &Origin) -> Vec<u8> {
    let mut a = Arrays::default();
    a.f64s("mz", &f.mz);
    a.f32s("rt", &f.rt);
    a.f32s("left", &f.left);
    a.f32s("right", &f.right);
    a.u32s("apex", &f.apex);
    a.u32s("scan_lo", &f.scan_lo);
    a.u32s("scan_hi", &f.scan_hi);
    a.f64s("area", &f.area);
    a.f32s("height", &f.height);
    a.u16s("npts", &f.npts);
    a.u8s("flags", &f.flags);
    a.u32s("iso_group", &f.iso_group);
    a.i8s("charge", &f.charge);
    a.f32s("shape", &f.shape);
    a.f32s("snr", &f.snr);
    a.f32s("fwhm", &f.fwhm);
    a.f32s("asym", &f.asym);
    a.u16s("gaps", &f.gaps);
    a.u8s("qreason", &f.qreason);
    a.u16s("ms2_n", &f.ms2_n);
    a.f32s("purity", &f.purity);
    a.f64s("area_win", &f.area_win);
    a.u32s("family_id", &f.family_id);
    a.u8s("role", &f.role);
    a.u8s("alt", &f.alt);
    a.f64s("dt", &[f.dt]);
    a.u8s("evid", &[]);
    container(&meta_json("file", r, o, &[("rows", f.len())], &a), &a)
}

/// Bytes of the `.mzfeat` of a series.
pub fn write_table(t: &Aligned, r: &Recipe, o: &Origin) -> Vec<u8> {
    let mut a = Arrays::default();
    a.f64s("mz", &t.mz);
    a.f64s("rt", &t.rt);
    a.f64s("area", &t.area);
    a.f32s("height", &t.height);
    a.f32s("apex_rt", &t.apex_rt);
    a.f32s("width", &t.width);
    a.u8s("state", &t.state);
    a.u32s("member_start", &t.member_start);
    a.f64s("member_mz", &t.member_mz);
    a.f32s("member_rt", &t.member_rt);
    a.u8s("member_role", &t.member_role);
    a.u8s("evid", &[]);
    container(
        &meta_json(
            "table",
            r,
            o,
            &[("rows", t.len()), ("files", t.n_files)],
            &a,
        ),
        &a,
    )
}

/// A container read back: the meta and the directory of the arrays.
struct Opened<'a> {
    meta: Value,
    data: &'a [u8],
}

fn open<'a>(bytes: &'a [u8], kind: &str) -> Result<Opened<'a>> {
    if bytes.len() < 16 || &bytes[..8] != MAGIC {
        return Err(bad());
    }
    let n = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
    let end = 16usize
        .checked_add(n)
        .filter(|&e| e <= bytes.len())
        .ok_or_else(bad)?;
    let meta = parse(&bytes[16..end])?;
    if meta.get("format").and_then(|v| v.str()) != Some(kind) {
        return Err(bad());
    }
    let start = end.div_ceil(ALIGN) * ALIGN;
    Ok(Opened {
        meta,
        data: bytes.get(start..).unwrap_or(&[]),
    })
}

impl Opened<'_> {
    /// The bytes of the array `name` when it has `dtype` and `size` bytes per element.
    fn array(&self, name: &str, dtype: &str, size: usize) -> Result<&[u8]> {
        let e = self.meta.get("arrays").map(|a| a.arr()).unwrap_or(&[]);
        let e = e
            .iter()
            .find(|x| x.get("name").and_then(|v| v.str()) == Some(name))
            .ok_or_else(bad)?;
        let get = |k: &str| {
            e.get(k)
                .and_then(|v| v.f64())
                .map(|v| v as usize)
                .ok_or_else(bad)
        };
        if e.get("dtype").and_then(|v| v.str()) != Some(dtype) {
            return Err(bad());
        }
        let (count, off) = (get("count")?, get("offset")?);
        let len = count.checked_mul(size).ok_or_else(bad)?;
        self.data
            .get(off..off.checked_add(len).ok_or_else(bad)?)
            .ok_or_else(bad)
    }
}

macro_rules! readers {
    ($($fn:ident, $t:ty, $name:literal);*) => { $(
        fn $fn(&self, name: &str) -> Result<Vec<$t>> {
            let b = self.array(name, $name, std::mem::size_of::<$t>())?;
            Ok(b.chunks_exact(std::mem::size_of::<$t>()).map(|c| <$t>::from_le_bytes(c.try_into().unwrap())).collect())
        }
    )* };
}

impl Opened<'_> {
    readers!(f64s, f64, "f64"; f32s, f32, "f32"; u32s, u32, "u32"; u16s, u16, "u16");
    fn u8s(&self, name: &str) -> Result<Vec<u8>> {
        Ok(self.array(name, "u8", 1)?.to_vec())
    }
    fn i8s(&self, name: &str) -> Result<Vec<i8>> {
        Ok(self
            .array(name, "i8", 1)?
            .iter()
            .map(|&x| x as i8)
            .collect())
    }
}

/// Reads a `.mzfeatf`: the table and its meta.
pub fn read_file(bytes: &[u8]) -> Result<(FileFeatures, Value)> {
    let o = open(bytes, "file")?;
    let f = FileFeatures {
        mz: o.f64s("mz")?,
        rt: o.f32s("rt")?,
        left: o.f32s("left")?,
        right: o.f32s("right")?,
        apex: o.u32s("apex")?,
        scan_lo: o.u32s("scan_lo")?,
        scan_hi: o.u32s("scan_hi")?,
        area: o.f64s("area")?,
        height: o.f32s("height")?,
        npts: o.u16s("npts")?,
        flags: o.u8s("flags")?,
        iso_group: o.u32s("iso_group")?,
        charge: o.i8s("charge")?,
        shape: o.f32s("shape")?,
        snr: o.f32s("snr")?,
        fwhm: o.f32s("fwhm")?,
        asym: o.f32s("asym")?,
        gaps: o.u16s("gaps")?,
        qreason: o.u8s("qreason")?,
        ms2_n: o.u16s("ms2_n")?,
        purity: o.f32s("purity")?,
        area_win: o.f64s("area_win")?,
        family_id: o.u32s("family_id")?,
        role: o.u8s("role")?,
        alt: o.u8s("alt")?,
        dt: o.f64s("dt")?.first().copied().ok_or_else(bad)?,
    };
    Ok((f, o.meta))
}

/// Reads a `.mzfeat`: the table and its meta.
pub fn read_table(bytes: &[u8]) -> Result<(Aligned, Value)> {
    let o = open(bytes, "table")?;
    let n_files = o.meta.get("files").and_then(|v| v.f64()).ok_or_else(bad)? as usize;
    let t = Aligned {
        n_files,
        mz: o.f64s("mz")?,
        rt: o.f64s("rt")?,
        area: o.f64s("area")?,
        height: o.f32s("height")?,
        apex_rt: o.f32s("apex_rt")?,
        width: o.f32s("width")?,
        state: o.u8s("state")?,
        member_start: o.u32s("member_start")?,
        member_mz: o.f64s("member_mz")?,
        member_rt: o.f32s("member_rt")?,
        member_role: o.u8s("member_role")?,
    };
    Ok((t, o.meta))
}
