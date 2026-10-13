//! Files of the feature tables: `.mzfeatf` (the table of one file, named `<sha256 of the input>.<fingerprint of the recipe>.mzfeatf`) and `.mzfeat`
//! (the aligned table of a series, `<series>.mzfeat`). Both have the same container, little-endian, versioned by the magic:
//!
//! `MZFEAT01` | meta length (u64) | meta (JSON text) | padding to 64 bytes | arrays, each starting at a multiple of 64 bytes from the end of the padding.
//!
//! The meta carries the recipe and its fingerprints, the version of the crate, the SHA-256 of every input file (from its index), the times (empty when
//! the bytes must be reproducible) and the directory of the arrays (`name`, `dtype`, `count`, `offset`). `evid` holds the graph of the proofs (`evid/1`, JSON): empty in
//! the file of one sample and when no graph was built. The bytes depend only on the table and on the meta given: no clock, no path.

use mzlab_core::budget::WriteAt;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Out;

use crate::facts::FileFacts;
use crate::recipe::Recipe;
use crate::rowident::adduct_names;
use crate::table::{Aligned, FileFeatures};
use crate::tp::TpBlock;

pub(crate) const MAGIC: &[u8; 8] = b"MZFEAT01";
pub(crate) const ALIGN: usize = 64;
/// Version of the container of the aligned table (`"version"` of its meta): 2 adds the `id`, the summaries of the rows and, optional, the identity columns (`identity` = 1 in the meta); the files of version 1 are still read.
pub const TABLE_FORMAT: i64 = 2;

pub(crate) fn bad() -> Error {
    Error::new("error.badMzfeat")
}

/// What the meta says about the origin of a table.
#[derive(Debug, Clone, Default)]
pub struct Origin {
    /// `(name, SHA-256)` of every input file, in the order of the files of the table.
    pub inputs: Vec<(String, String)>,
    /// How each input was acquired (meta `columns`, same order as `inputs`); empty when not read.
    pub facts: Vec<FileFacts>,
    /// SHA-256 of the `campioni/1` document of the series (meta `samples`), when there is one.
    pub samples: Option<String>,
    /// `(step, seconds)`; leave empty to get the same bytes in two runs.
    pub times: Vec<(String, f64)>,
}

/// A column of a container: a slice of one of the types of the format.
#[derive(Clone, Copy)]
enum Col<'a> {
    F64(&'a [f64]),
    F32(&'a [f32]),
    U32(&'a [u32]),
    U16(&'a [u16]),
    U8(&'a [u8]),
    I8(&'a [i8]),
}

/// Elements converted and written at a time (the buffer of a write is at most `PIECE * 8` bytes).
pub(crate) const PIECE: usize = 8192;

macro_rules! put_le {
    ($seq:expr, $v:expr) => {
        for part in $v.chunks(PIECE) {
            $seq.buf.clear();
            $seq.buf.extend(part.iter().flat_map(|x| x.to_le_bytes()));
            $seq.flush()?;
        }
    };
}

impl Col<'_> {
    /// `(dtype, count, bytes per element)`.
    fn shape(&self) -> (&'static str, usize, usize) {
        match self {
            Col::F64(v) => ("f64", v.len(), 8),
            Col::F32(v) => ("f32", v.len(), 4),
            Col::U32(v) => ("u32", v.len(), 4),
            Col::U16(v) => ("u16", v.len(), 2),
            Col::U8(v) => ("u8", v.len(), 1),
            Col::I8(v) => ("i8", v.len(), 1),
        }
    }

    fn put<W: WriteAt>(&self, seq: &mut Seq<W>) -> Result<()> {
        match self {
            Col::F64(v) => put_le!(seq, v),
            Col::F32(v) => put_le!(seq, v),
            Col::U32(v) => put_le!(seq, v),
            Col::U16(v) => put_le!(seq, v),
            Col::U8(v) => seq.bytes(v)?,
            Col::I8(v) => {
                for part in v.chunks(PIECE) {
                    seq.buf.clear();
                    seq.buf.extend(part.iter().map(|&x| x as u8));
                    seq.flush()?;
                }
            }
        }
        Ok(())
    }
}

/// A section that is written in pieces and whose length is known beforehand (the `evid` graph: tens of MB that never exist as one buffer).
pub trait Section {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Gives the bytes, in order, to `put`; they must add up to `len()`.
    fn emit(&mut self, put: &mut dyn FnMut(&[u8]) -> Result<()>) -> Result<()>;
}

impl Section for &[u8] {
    fn len(&self) -> usize {
        <[u8]>::len(self)
    }
    fn emit(&mut self, put: &mut dyn FnMut(&[u8]) -> Result<()>) -> Result<()> {
        put(self)
    }
}

/// Sequential writer over a `WriteAt` with a reusable conversion buffer.
struct Seq<'a, W: WriteAt> {
    sink: &'a mut W,
    pos: u64,
    buf: Vec<u8>,
}

impl<W: WriteAt> Seq<'_, W> {
    fn bytes(&mut self, b: &[u8]) -> Result<()> {
        self.sink.write_at(self.pos, b)?;
        self.pos += b.len() as u64;
        Ok(())
    }

    /// Writes the conversion buffer.
    fn flush(&mut self) -> Result<()> {
        self.sink.write_at(self.pos, &self.buf)?;
        self.pos += self.buf.len() as u64;
        Ok(())
    }

    fn pad_to(&mut self, n: usize) -> Result<()> {
        let k = (n - (self.pos as usize) % n) % n;
        self.bytes(&[0u8; ALIGN][..k])
    }
}

/// Directory of the arrays: `(name, dtype, count, offset from the start of the data)` and the length of the data.
fn plan(cols: &[(&'static str, Col)], evid: usize) -> (Vec<(&'static str, &'static str, usize, usize)>, usize) {
    let (mut dir, mut at) = (Vec::with_capacity(cols.len() + 1), 0usize);
    let items = cols.iter().map(|(n, c)| (*n, c.shape()));
    for (name, (dtype, count, size)) in items.chain([("evid", ("u8", evid, 1))]) {
        at = at.div_ceil(ALIGN) * ALIGN;
        dir.push((name, dtype, count, at));
        at += count * size;
    }
    (dir, at)
}

/// The names of the adducts that the codes of `ion` stand for, as the meta `adducts`.
fn adducts_json(j: &mut Out) {
    j.raw(",\"adducts\":[");
    for (i, a) in adduct_names().iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).str(a);
    }
    j.raw("]");
}

/// The JSON meta: format, versions, recipe, fingerprints, inputs, times, shape and directory of the arrays.
fn meta_json(
    kind: &str,
    r: &Recipe,
    o: &Origin,
    shape: &[(&str, usize)],
    extra: &str,
    dir: &[(&'static str, &'static str, usize, usize)],
) -> String {
    let mut j = Out::default();
    let text = if kind == "file" {
        r.file_text()
    } else {
        r.table_text()
    };
    j.raw("{\"format\":")
        .str(kind)
        .raw(",\"version\":")
        .int(if kind == "table" { TABLE_FORMAT } else { 1 })
        .raw(",\"crate\":")
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
    j.raw("]");
    if !o.facts.is_empty() {
        j.raw(",\"columns\":[");
        for (i, f) in o.facts.iter().enumerate() {
            j.raw(if i > 0 { "," } else { "" });
            f.write(&mut j);
        }
        j.raw("]");
    }
    if let Some(s) = &o.samples {
        j.raw(",\"samples\":").str(s);
    }
    j.raw(",\"times\":{");
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
    if shape.contains(&("identity", 1)) {
        adducts_json(&mut j);
    }
    j.raw(extra);
    j.raw(",\"arrays\":[");
    for (i, (name, dtype, count, off)) in dir.iter().enumerate() {
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

/// Writes a whole container to `sink`, in order: header, meta, the columns and the `evid` section (never held as one buffer).
fn write_container<W: WriteAt>(
    sink: &mut W,
    kind: &str,
    r: &Recipe,
    o: &Origin,
    shape: &[(&str, usize)],
    extra: &str,
    cols: &[(&'static str, Col)],
    evid: &mut dyn Section,
) -> Result<()> {
    let (dir, _) = plan(cols, evid.len());
    let meta = meta_json(kind, r, o, shape, extra, &dir);
    let mut seq = Seq {
        sink,
        pos: 0,
        buf: Vec::with_capacity(PIECE * 8),
    };
    seq.bytes(MAGIC)?;
    seq.bytes(&(meta.len() as u64).to_le_bytes())?;
    seq.bytes(meta.as_bytes())?;
    seq.pad_to(ALIGN)?;
    let start = seq.pos;
    for ((_, col), (_, _, _, off)) in cols.iter().zip(&dir) {
        seq.bytes(&vec![0u8; (start as usize + off) - seq.pos as usize])?;
        col.put(&mut seq)?;
    }
    let off = dir[cols.len()].3;
    seq.bytes(&vec![0u8; (start as usize + off) - seq.pos as usize])?;
    let seq = std::cell::RefCell::new(seq);
    evid.emit(&mut |b| seq.borrow_mut().bytes(b))
}

fn file_cols(f: &FileFeatures) -> Vec<(&'static str, Col<'_>)> {
    vec![
        ("mz", Col::F64(&f.mz)),
        ("rt", Col::F32(&f.rt)),
        ("left", Col::F32(&f.left)),
        ("right", Col::F32(&f.right)),
        ("apex", Col::U32(&f.apex)),
        ("scan_lo", Col::U32(&f.scan_lo)),
        ("scan_hi", Col::U32(&f.scan_hi)),
        ("area", Col::F64(&f.area)),
        ("height", Col::F32(&f.height)),
        ("npts", Col::U16(&f.npts)),
        ("flags", Col::U8(&f.flags)),
        ("iso_group", Col::U32(&f.iso_group)),
        ("charge", Col::I8(&f.charge)),
        ("shape", Col::F32(&f.shape)),
        ("snr", Col::F32(&f.snr)),
        ("fwhm", Col::F32(&f.fwhm)),
        ("asym", Col::F32(&f.asym)),
        ("gaps", Col::U16(&f.gaps)),
        ("qreason", Col::U8(&f.qreason)),
        ("ms2_n", Col::U16(&f.ms2_n)),
        ("purity", Col::F32(&f.purity)),
        ("area_win", Col::F64(&f.area_win)),
        ("family_id", Col::U32(&f.family_id)),
        ("role", Col::U8(&f.role)),
        ("alt", Col::U8(&f.alt)),
    ]
}

/// Writes the `.mzfeatf` of one file to `sink`.
pub fn write_file_to<W: WriteAt>(sink: &mut W, f: &FileFeatures, r: &Recipe, o: &Origin) -> Result<()> {
    let mut cols = file_cols(f);
    if !f.q3.is_empty() {
        cols.push(("q3", Col::F64(&f.q3)));
    }
    let dt = [f.dt];
    cols.push(("dt", Col::F64(&dt)));
    write_container(sink, "file", r, o, &[("rows", f.len())], "", &cols, &mut (&[][..]))
}

/// Bytes of the `.mzfeatf` of one file.
pub fn write_file(f: &FileFeatures, r: &Recipe, o: &Origin) -> Vec<u8> {
    let mut v = Vec::new();
    write_file_to(&mut v, f, r, o).expect("a Vec grows");
    v
}

/// Adds the columns of the block `tp/1` to `cols` when the table has one; says whether it did.
fn tp_cols<'a>(tp: &'a TpBlock, cols: &mut Vec<(&'static str, Col<'a>)>) -> bool {
    if tp.is_empty() {
        return false;
    }
    cols.push(("tp_status", Col::U8(&tp.status)));
    cols.push(("tp_notes", Col::U8(&tp.notes)));
    cols.push(("tp_via", Col::U8(&tp.via)));
    cols.push(("tp_kinship", Col::U8(&tp.kinship)));
    true
}

/// Bytes of the `.mzfeat` of a series, with an empty `evid` section.
pub fn write_table(t: &Aligned, r: &Recipe, o: &Origin) -> Vec<u8> {
    write_table_with_evid(t, r, o, &[])
}

/// Writes the `.mzfeat` of a series to `sink` with `evid` (the graph of the proofs, `evid/1` JSON) as its last section.
pub fn write_table_to<W: WriteAt>(
    sink: &mut W,
    t: &Aligned,
    r: &Recipe,
    o: &Origin,
    evid: &mut dyn Section,
) -> Result<()> {
    write_table_tp(sink, t, &t.tp, r, o, evid)
}

/// `write_table_to` with the block `tp/1` given apart from the table (the view of the products is computed from the proofs, which borrow the table).
pub fn write_table_tp<W: WriteAt>(
    sink: &mut W,
    t: &Aligned,
    tp: &TpBlock,
    r: &Recipe,
    o: &Origin,
    evid: &mut dyn Section,
) -> Result<()> {
    let ids: Vec<u32> = (0..t.len()).map(|r| t.id(r)).collect();
    let m = t.meta.or_none(t.len());
    let mut cols = vec![
        ("id", Col::U32(&ids)),
        ("mz", Col::F64(&t.mz)),
        ("rt", Col::F64(&t.rt)),
        ("area", Col::F64(&t.area)),
        ("height", Col::F32(&t.height)),
        ("apex_rt", Col::F32(&t.apex_rt)),
        ("width", Col::F32(&t.width)),
        ("state", Col::U8(&t.state)),
        ("member_start", Col::U32(&t.member_start)),
        ("member_mz", Col::F64(&t.member_mz)),
        ("member_rt", Col::F32(&t.member_rt)),
        ("member_role", Col::U8(&t.member_role)),
        ("rt_lo", Col::F32(&m.rt_lo)),
        ("rt_hi", Col::F32(&m.rt_hi)),
        ("charge", Col::I8(&m.charge)),
        ("q_shape", Col::F32(&m.q_shape)),
        ("q_snr", Col::F32(&m.q_snr)),
        ("q_fwhm", Col::F32(&m.q_fwhm)),
        ("qreason", Col::U8(&m.qreason)),
        ("iso_m1", Col::F32(&m.iso_m1)),
        ("ms2_file", Col::U32(&m.ms2_file)),
        ("ms2_purity", Col::F32(&m.ms2_purity)),
    ];
    if !t.q3.is_empty() {
        cols.push(("q3", Col::F64(&t.q3)));
    }
    let ident = !t.ident.is_empty();
    if ident {
        cols.push(("neutral", Col::F64(&t.ident.neutral)));
        cols.push(("ion", Col::U8(&t.ident.ion)));
        cols.push(("src", Col::U8(&t.ident.src)));
    }
    let shape = [
        ("rows", t.len()),
        ("files", t.n_files),
        ("summaries", usize::from(!t.meta.is_empty())),
        ("identity", usize::from(ident)),
    ];
    let tp = if tp_cols(tp, &mut cols) { format!(",\"tp\":{}", tp.meta_json()) } else { String::new() };
    write_container(sink, "table", r, o, &shape, &tp, &cols, evid)
}

/// Bytes of the `.mzfeat` of a series with the graph of the proofs (`evid/1`, JSON bytes of `evid::graph_json`) in the `evid` section.
pub fn write_table_with_evid(t: &Aligned, r: &Recipe, o: &Origin, evid: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    write_table_to(&mut v, t, r, o, &mut { evid }).expect("a Vec grows");
    v
}

pub use crate::store_read::*;
