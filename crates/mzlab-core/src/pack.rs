//! `pack/1`: many files as one stored-ZIP stream cut into parts of a fixed size, plus a manifest with the SHA-256 of every file and every part
//! (docs/agenti/file-pack.md). The bytes depend only on the names, the contents and the options; memory is one 1 MiB buffer. Reading and writing go
//! through small traits, so the command line (files) and the page (OPFS through JS callbacks) run this same code. Verification and unpacking live in `pack_read`.

use std::io::Read;

use sha2::{Digest as _, Sha256};

use crate::error::{Error, Result};
use crate::json::Out;
use crate::zip::Crc32;
use crate::zipstream::{self, Member};

pub const SCHEMA: &str = "pack/1";
const BLOCK: usize = 1 << 20;

/// One file of the pack: its relative path (`/` separators), size, SHA-256 and where its bytes start in the stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRec {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub offset: u64,
}

/// One part: its file name (in the folder of the manifest), size and SHA-256.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartRec {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

/// The `pack/1` document. A planned pack (`--dry-run`) has empty SHA-256 values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub part_size: u64,
    pub stream_bytes: u64,
    pub files: Vec<FileRec>,
    pub parts: Vec<PartRec>,
}

pub struct Options {
    pub name: String,
    pub part_size: u64,
    /// Size from which an entry is ZIP64 (`zipstream::ZIP64_AT`; tests lower it).
    pub zip64_at: u64,
}

/// Where the bytes of the parts go: `put` appends to part `part` (0-based); a new index starts a new part.
pub trait PartSink {
    fn put(&mut self, part: usize, bytes: &[u8]) -> Result<()>;
}

pub(crate) fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn bad(detail: impl ToString) -> Error {
    Error::new("err.pack.manifest").with("detail", detail)
}

/// The name of part `i` (0-based) of the pack `name`.
pub fn part_name(name: &str, i: usize) -> String {
    format!("{name}.part{:04}", i + 1)
}

/// Whether `p` is a relative path that stays inside the folder it is written under: no leading `/`, no `\`, no NUL, no `:` anywhere (a drive such as
/// `C:x`, or an NTFS stream), no `.` or `..` or empty component.
pub fn safe_path(p: &str) -> bool {
    !p.is_empty()
        && !p.contains(['\\', '\0', ':'])
        && !p.starts_with('/')
        && p.split('/').all(|c| !matches!(c, "" | "." | ".."))
}

/// Two paths that are one file on a case-insensitive file system (macOS, Windows), or the same path twice: `err.pack.duplicate` naming both.
pub fn refuse_duplicates<'a>(paths: impl Iterator<Item = &'a str>) -> Result<()> {
    let mut seen = std::collections::BTreeMap::new();
    for p in paths {
        if let Some(first) = seen.insert(p.to_lowercase(), p) {
            return Err(Error::new("err.pack.duplicate")
                .with("path", p)
                .with("other", first));
        }
    }
    Ok(())
}

fn plain_name(n: &str) -> bool {
    safe_path(n) && !n.contains('/')
}

/// Offsets of the data of each file and the length of the stream, from the names and sizes alone: what a dry run reports and what `pack` must reproduce.
pub fn plan(files: &[(String, u64)], o: &Options) -> Manifest {
    let (mut at, mut members, mut recs) = (0u64, Vec::new(), Vec::new());
    for (name, size) in files {
        let (head, tail) = zipstream::framing(name, *size, o.zip64_at);
        members.push(Member {
            name: name.clone(),
            size: *size,
            crc: 0,
            at,
        });
        recs.push(FileRec {
            path: name.clone(),
            size: *size,
            sha256: String::new(),
            offset: at + head,
        });
        at += head + size + tail;
    }
    let total = at + zipstream::central_directory(&members, at, o.zip64_at).len() as u64;
    let n = total.div_ceil(o.part_size.max(1)).max(1);
    let parts = (0..n)
        .map(|i| PartRec {
            name: part_name(&o.name, i as usize),
            size: o.part_size.min(total - i * o.part_size),
            sha256: String::new(),
        })
        .collect();
    Manifest {
        name: o.name.clone(),
        part_size: o.part_size,
        stream_bytes: total,
        files: recs,
        parts,
    }
}

/// Cuts the stream into parts and hashes each one as it goes.
struct Parts<'a> {
    sink: &'a mut dyn PartSink,
    size: u64,
    idx: usize,
    filled: u64,
    sha: Sha256,
    done: Vec<(u64, String)>,
    total: u64,
}

impl Parts<'_> {
    fn write(&mut self, mut b: &[u8]) -> Result<()> {
        while !b.is_empty() {
            let room = usize::try_from(self.size - self.filled).unwrap_or(usize::MAX);
            let (head, tail) = b.split_at(room.min(b.len()));
            self.sink.put(self.idx, head)?;
            self.sha.update(head);
            self.filled += head.len() as u64;
            self.total += head.len() as u64;
            b = tail;
            if self.filled == self.size {
                self.close();
            }
        }
        Ok(())
    }
    fn close(&mut self) {
        let sha = std::mem::take(&mut self.sha).finalize();
        self.done.push((self.filled, hex(&sha)));
        self.idx += 1;
        self.filled = 0;
    }
}

/// One file of the stream: header, bytes (hashed as they pass) and descriptor. More or fewer bytes than the size listed is `err.pack.growing`.
fn add_file(
    parts: &mut Parts<'_>,
    (index, name, size): (usize, &str, u64),
    open: &mut dyn FnMut(usize) -> Result<Box<dyn Read>>,
    buf: &mut [u8],
    zip64_at: u64,
) -> Result<(Member, FileRec)> {
    let at = parts.total;
    parts.write(&zipstream::local_header(name, size, zip64_at))?;
    let offset = parts.total;
    let (mut sha, mut crc, mut seen) = (Sha256::new(), Crc32::new(), 0u64);
    let mut r = open(index)?;
    loop {
        let n = r
            .read(buf)
            .map_err(|e| Error::new("err.run.input").with("detail", e))?;
        if n == 0 {
            break;
        }
        let piece = buf.get(..n).unwrap_or_default();
        sha.update(piece);
        crc.update(piece);
        seen += n as u64;
        if seen > size {
            return Err(Error::new("err.pack.growing").with("path", name));
        }
        parts.write(piece)?;
    }
    if seen != size {
        return Err(Error::new("err.pack.growing").with("path", name));
    }
    parts.write(&zipstream::descriptor(crc.value(), seen, zip64_at))?;
    let member = Member {
        name: name.to_string(),
        size: seen,
        crc: crc.value(),
        at,
    };
    Ok((
        member,
        FileRec {
            path: name.to_string(),
            size: seen,
            sha256: hex(&sha.finalize()),
            offset,
        },
    ))
}

/// Writes the stream of `files` (relative path, size at the time of listing) into parts, reading each through `open`. A file whose bytes are not the
/// size listed is still being written (or was changed): `err.pack.growing`, nothing is trusted.
pub fn pack(
    files: &[(String, u64)],
    open: &mut dyn FnMut(usize) -> Result<Box<dyn Read>>,
    sink: &mut dyn PartSink,
    o: &Options,
) -> Result<Manifest> {
    if o.part_size == 0 {
        return Err(
            Error::new("err.run.input").with("detail", "the part size must be at least 1 byte")
        );
    }
    for (name, _) in files {
        if !safe_path(name) || zipstream::check_name(name).is_err() {
            return Err(Error::new("err.pack.path").with("path", name));
        }
    }
    refuse_duplicates(files.iter().map(|f| f.0.as_str()))?;
    let planned = plan(files, o);
    let mut parts = Parts {
        sink,
        size: o.part_size,
        idx: 0,
        filled: 0,
        sha: Sha256::new(),
        done: Vec::new(),
        total: 0,
    };
    let (mut members, mut recs, mut buf) = (Vec::new(), Vec::new(), vec![0u8; BLOCK]);
    for (i, (name, size)) in files.iter().enumerate() {
        let (m, rec) = add_file(&mut parts, (i, name, *size), open, &mut buf, o.zip64_at)?;
        members.push(m);
        recs.push(rec);
    }
    let dir_at = parts.total;
    parts.write(&zipstream::central_directory(&members, dir_at, o.zip64_at))?;
    if parts.filled > 0 || parts.done.is_empty() {
        parts.close();
    }
    let done = std::mem::take(&mut parts.done);
    let total = parts.total;
    let recs_parts = planned
        .parts
        .iter()
        .zip(done)
        .map(|(p, (size, sha256))| PartRec {
            name: p.name.clone(),
            size,
            sha256,
        })
        .collect();
    Ok(Manifest {
        name: o.name.clone(),
        part_size: o.part_size,
        stream_bytes: total,
        files: recs,
        parts: recs_parts,
    })
}

fn files_json(o: &mut Out, m: &Manifest, with_offset: bool) {
    o.raw("[");
    for (i, f) in m.files.iter().enumerate() {
        o.raw(if i > 0 { ",{\"path\":" } else { "{\"path\":" })
            .str(&f.path)
            .raw(",\"size\":")
            .int(f.size as i64);
        if with_offset {
            o.raw(",\"offset\":").int(f.offset as i64);
        }
        o.raw(",\"sha256\":");
        sha_json(o, &f.sha256);
        o.raw("}");
    }
    o.raw("]");
}

fn sha_json(o: &mut Out, sha: &str) {
    if sha.is_empty() {
        o.raw("null");
    } else {
        o.str(sha);
    }
}

fn parts_json(o: &mut Out, m: &Manifest) {
    o.raw("[");
    for (i, p) in m.parts.iter().enumerate() {
        o.raw(if i > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(&p.name)
            .raw(",\"size\":")
            .int(p.size as i64)
            .raw(",\"sha256\":");
        sha_json(&mut *o, &p.sha256);
        o.raw("}");
    }
    o.raw("]");
}

/// The `pack/1` manifest text (the contract between the machines: file order, offsets, sizes and hashes of everything).
pub fn manifest_json(m: &Manifest) -> String {
    let mut o = Out(String::new());
    o.raw("{\"schema\":\"pack/1\",\"tool\":{\"name\":\"mzlab\",\"version\":")
        .str(env!("CARGO_PKG_VERSION"))
        .raw("},\"name\":")
        .str(&m.name)
        .raw(",\"part_size\":")
        .int(m.part_size as i64)
        .raw(",\"stream_bytes\":")
        .int(m.stream_bytes as i64)
        .raw(",\"files\":");
    files_json(&mut o, m, true);
    o.raw(",\"parts\":");
    parts_json(&mut o, m);
    o.raw("}\n");
    o.0
}

/// The `pack-run/1` answer of `file pack` (and of `--dry-run`, where the hashes are null): the manifest without the tool block and the offsets.
pub fn run_json(m: &Manifest, dry_run: bool) -> String {
    let mut o = Out(String::new());
    o.raw("{\"schema\":\"pack-run/1\",\"dry_run\":")
        .raw(if dry_run { "true" } else { "false" })
        .raw(",\"name\":")
        .str(&m.name)
        .raw(",\"manifest\":")
        .str(&format!("{}.pack.json", m.name))
        .raw(",\"part_size\":")
        .int(m.part_size as i64)
        .raw(",\"stream_bytes\":")
        .int(m.stream_bytes as i64)
        .raw(",\"files\":");
    files_json(&mut o, m, false);
    o.raw(",\"parts\":");
    parts_json(&mut o, m);
    o.raw("}\n");
    o.0
}

fn sha_ok(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn num(v: &crate::json::Value, key: &str) -> Result<u64> {
    let x = v
        .get(key)
        .and_then(crate::json::Value::f64)
        .ok_or_else(|| bad(format!("{key} is missing")))?;
    if x < 0.0 || x.fract() != 0.0 || x > 9_007_199_254_740_991.0 {
        return Err(bad(format!("{key} is not a size")));
    }
    Ok(x as u64)
}

fn text<'a>(v: &'a crate::json::Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(crate::json::Value::str)
        .ok_or_else(|| bad(format!("{key} is missing")))
}

/// Reads a `pack/1` manifest and checks it is consistent: a path that leaves the target folder is `err.pack.path`, anything else wrong `err.pack.manifest`.
pub fn parse_manifest(bytes: &[u8]) -> Result<Manifest> {
    let v = crate::json::parse(bytes).map_err(|_| bad("not JSON"))?;
    if text(&v, "schema")? != SCHEMA {
        return Err(bad("schema is not pack/1"));
    }
    let (name, part_size, stream_bytes) = (
        text(&v, "name")?.to_string(),
        num(&v, "part_size")?,
        num(&v, "stream_bytes")?,
    );
    if part_size == 0 || name.is_empty() {
        return Err(bad("part_size or name is empty"));
    }
    let mut files = Vec::new();
    let mut end = 0u64;
    for f in v.get("files").map_or(&[][..], crate::json::Value::arr) {
        let rec = FileRec {
            path: text(f, "path")?.to_string(),
            size: num(f, "size")?,
            sha256: text(f, "sha256")?.to_string(),
            offset: num(f, "offset")?,
        };
        if !safe_path(&rec.path) {
            return Err(Error::new("err.pack.path").with("path", &rec.path));
        }
        if !sha_ok(&rec.sha256)
            || rec.offset < end
            || rec
                .offset
                .checked_add(rec.size)
                .is_none_or(|e| e > stream_bytes)
        {
            return Err(bad(format!(
                "file {} is out of order or out of the stream",
                rec.path
            )));
        }
        end = rec.offset + rec.size;
        files.push(rec);
    }
    let mut parts = Vec::new();
    for p in v.get("parts").map_or(&[][..], crate::json::Value::arr) {
        let rec = PartRec {
            name: text(p, "name")?.to_string(),
            size: num(p, "size")?,
            sha256: text(p, "sha256")?.to_string(),
        };
        if !plain_name(&rec.name) || !sha_ok(&rec.sha256) {
            return Err(Error::new("err.pack.path").with("path", &rec.name));
        }
        parts.push(rec);
    }
    if parts.is_empty() || parts.iter().fold(0u64, |a, p| a.saturating_add(p.size)) != stream_bytes
    {
        return Err(bad("the parts do not add up to stream_bytes"));
    }
    refuse_duplicates(files.iter().map(|f| f.path.as_str()))?;
    Ok(Manifest {
        name,
        part_size,
        stream_bytes,
        files,
        parts,
    })
}
