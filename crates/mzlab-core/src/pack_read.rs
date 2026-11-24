//! Checking and unpacking a `pack/1` (docs/agenti/file-pack.md): every part is hashed against the manifest, the files that lie in intact parts are
//! written and checked against their own SHA-256, the others are listed as lost; nothing is written over an existing file without `force`.

use std::io::{self, Read};

use sha2::{Digest as _, Sha256};

use crate::error::{Error, Result};
use crate::json::Out;
use crate::pack::{hex, FileRec, Manifest};

/// Where the parts are read from. `size` is `None` for a part that is not there; `open` starts reading at an offset of the part.
pub trait PartSource {
    fn size(&mut self, part: usize) -> Result<Option<u64>>;
    fn open(&mut self, part: usize, from: u64) -> Result<Box<dyn Read>>;
}

/// Where the files go. A file is `begin`, written in pieces, then `end(true)` to keep it or `end(false)` to drop it.
pub trait FileSink {
    fn exists(&mut self, path: &str) -> Result<bool>;
    fn begin(&mut self, path: &str) -> Result<()>;
    fn write(&mut self, bytes: &[u8]) -> Result<()>;
    fn end(&mut self, keep: bool) -> Result<()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartState {
    Ok,
    Missing,
    /// Wrong size, or the right size and a different SHA-256.
    Corrupt(&'static str),
}

impl PartState {
    pub fn key(self) -> &'static str {
        match self {
            PartState::Ok => "ok",
            PartState::Missing => "missing",
            PartState::Corrupt(_) => "corrupt",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileState {
    Written,
    /// A dry run: every part of the file is intact, so it would be written.
    Ready,
    /// A part it lies in is missing or corrupt.
    Lost,
    /// Its parts are intact but its bytes are not the ones the manifest names.
    Mismatch,
}

impl FileState {
    pub fn key(self) -> &'static str {
        match self {
            FileState::Written => "written",
            FileState::Ready => "ready",
            FileState::Lost => "lost",
            FileState::Mismatch => "mismatch",
        }
    }
}

fn input(e: impl ToString) -> Error {
    Error::new("err.run.input").with("detail", e)
}

/// SHA-256 of exactly `size` bytes of `r`, or `None` when it gives fewer.
fn hash_exact(r: &mut dyn Read, size: u64) -> io::Result<Option<String>> {
    let (mut sha, mut buf, mut left) = (Sha256::new(), vec![0u8; 1 << 20], size);
    while left > 0 {
        let want = usize::try_from(left).unwrap_or(usize::MAX).min(buf.len());
        let n = r.read(buf.get_mut(..want).unwrap_or_default())?;
        if n == 0 {
            return Ok(None);
        }
        sha.update(buf.get(..n).unwrap_or_default());
        left -= n as u64;
    }
    Ok(Some(hex(&sha.finalize())))
}

/// The state of every part. `seen` is called as each one is decided, so the command can say where it is.
pub fn verify_parts(
    m: &Manifest,
    src: &mut dyn PartSource,
    seen: &mut dyn FnMut(usize, PartState),
) -> Result<Vec<PartState>> {
    let mut out = Vec::with_capacity(m.parts.len());
    for (i, p) in m.parts.iter().enumerate() {
        let state = match src.size(i)? {
            None => PartState::Missing,
            Some(n) if n != p.size => PartState::Corrupt("size"),
            Some(_) => match hash_exact(&mut src.open(i, 0)?, p.size).map_err(input)? {
                Some(h) if h == p.sha256 => PartState::Ok,
                Some(_) => PartState::Corrupt("sha256"),
                None => PartState::Corrupt("size"),
            },
        };
        seen(i, state);
        out.push(state);
    }
    Ok(out)
}

/// The bytes `pos..end` of the stream, read across the parts.
struct Span<'a> {
    src: &'a mut dyn PartSource,
    starts: &'a [u64],
    sizes: &'a [u64],
    pos: u64,
    end: u64,
    cur: Option<Box<dyn Read>>,
}

impl Read for Span<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.end || buf.is_empty() {
            return Ok(0);
        }
        let i = self
            .starts
            .partition_point(|&s| s <= self.pos)
            .saturating_sub(1);
        let (start, size) = (
            self.starts.get(i).copied().unwrap_or(0),
            self.sizes.get(i).copied().unwrap_or(0),
        );
        let left = (start + size).min(self.end) - self.pos;
        if self.cur.is_none() {
            self.cur = Some(
                self.src
                    .open(i, self.pos - start)
                    .map_err(|e| io::Error::other(e.key))?,
            );
        }
        let want = usize::try_from(left).unwrap_or(usize::MAX).min(buf.len());
        let n = match self.cur.as_mut() {
            Some(r) => r.read(buf.get_mut(..want).unwrap_or_default())?,
            None => 0,
        };
        if n == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        self.pos += n as u64;
        if self.pos >= start + size {
            self.cur = None;
        }
        Ok(n)
    }
}

/// `err.pack.exists` naming the first file of the pack that is already there (and how many are), before anything is written.
fn refuse_existing(m: &Manifest, sink: &mut dyn FileSink) -> Result<()> {
    let mut taken = Vec::new();
    for f in &m.files {
        if sink.exists(&f.path)? {
            taken.push(f.path.as_str());
        }
    }
    match taken.first() {
        Some(first) => Err(Error::new("err.pack.exists")
            .with("path", first)
            .with("count", taken.len())),
        None => Ok(()),
    }
}

/// Writes one file from its span and keeps it only when its SHA-256 is the manifest's; a write or read failure drops it and is the error.
fn copy_file(span: &mut Span<'_>, sink: &mut dyn FileSink, f: &FileRec) -> Result<bool> {
    sink.begin(&f.path)?;
    let (mut sha, mut buf) = (Sha256::new(), vec![0u8; 1 << 20]);
    let ok = loop {
        match span.read(&mut buf) {
            Ok(0) => break true,
            Ok(n) => {
                let piece = buf.get(..n).unwrap_or_default();
                sha.update(piece);
                if let Err(e) = sink.write(piece) {
                    sink.end(false)?;
                    return Err(e);
                }
            }
            // a part that cannot be read now is an input problem, not a file that does not match
            Err(e) => {
                sink.end(false)?;
                return Err(input(e));
            }
        }
    } && hex(&sha.finalize()) == f.sha256;
    sink.end(ok)?;
    Ok(ok)
}

/// Writes every file whose parts are intact (`write` false: only says which would be), and returns the state of each part and each file.
/// Without `force` an existing target file is `err.pack.exists` before anything is written or read.
pub fn unpack(
    m: &Manifest,
    src: &mut dyn PartSource,
    sink: &mut dyn FileSink,
    force: bool,
    write: bool,
    seen: &mut dyn FnMut(usize, PartState),
) -> Result<(Vec<PartState>, Vec<FileState>)> {
    if !force {
        refuse_existing(m, sink)?;
    }
    let parts = verify_parts(m, src, seen)?;
    let (mut starts, mut at) = (Vec::new(), 0u64);
    for p in &m.parts {
        starts.push(at);
        at += p.size;
    }
    let sizes: Vec<u64> = m.parts.iter().map(|p| p.size).collect();
    let mut files = Vec::with_capacity(m.files.len());
    for f in &m.files {
        let (first, last) = (
            starts.partition_point(|&s| s <= f.offset).saturating_sub(1),
            starts
                .partition_point(|&s| s < f.offset + f.size.max(1))
                .saturating_sub(1),
        );
        if parts
            .get(first..=last)
            .is_none_or(|s| s.iter().any(|p| *p != PartState::Ok))
        {
            files.push(FileState::Lost);
            continue;
        }
        if !write {
            files.push(FileState::Ready);
            continue;
        }
        let mut span = Span {
            src: &mut *src,
            starts: &starts,
            sizes: &sizes,
            pos: f.offset,
            end: f.offset + f.size,
            cur: None,
        };
        let ok = copy_file(&mut span, sink, f)?;
        files.push(if ok {
            FileState::Written
        } else {
            FileState::Mismatch
        });
    }
    Ok((parts, files))
}

/// Whether nothing is wrong, and the code class: corrupt or mismatching bytes outrank missing parts.
pub fn damage(parts: &[PartState], files: &[FileState]) -> (bool, bool) {
    let corrupt = parts.iter().any(|p| matches!(p, PartState::Corrupt(_)))
        || files.contains(&FileState::Mismatch);
    let missing = parts.contains(&PartState::Missing) || files.contains(&FileState::Lost);
    (corrupt, missing)
}

fn parts_array(o: &mut Out, m: &Manifest, parts: &[PartState]) {
    o.raw("[");
    for (i, (p, s)) in m.parts.iter().zip(parts).enumerate() {
        o.raw(if i > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(&p.name)
            .raw(",\"size\":")
            .int(p.size as i64)
            .raw(",\"status\":")
            .str(s.key());
        if let PartState::Corrupt(why) = s {
            o.raw(",\"why\":").str(why);
        }
        o.raw("}");
    }
    o.raw("]");
}

fn names(o: &mut Out, m: &Manifest, parts: &[PartState], want: fn(&PartState) -> bool) {
    o.raw("[");
    let mut first = true;
    for (p, _) in m.parts.iter().zip(parts).filter(|(_, s)| want(s)) {
        o.raw(if first { "" } else { "," }).str(&p.name);
        first = false;
    }
    o.raw("]");
}

/// `verify-pack/1`: the state of every part and the names to send again (`resend`: the missing and the corrupt ones).
pub fn verify_json(m: &Manifest, parts: &[PartState]) -> String {
    let ok = parts.iter().all(|p| *p == PartState::Ok);
    let mut o = Out(String::new());
    o.raw("{\"schema\":\"verify-pack/1\",\"ok\":")
        .raw(if ok { "true" } else { "false" })
        .raw(",\"name\":")
        .str(&m.name)
        .raw(",\"parts\":");
    parts_array(&mut o, m, parts);
    o.raw(",\"missing\":");
    names(&mut o, m, parts, |s| *s == PartState::Missing);
    o.raw(",\"corrupt\":");
    names(&mut o, m, parts, |s| matches!(s, PartState::Corrupt(_)));
    o.raw(",\"resend\":");
    names(&mut o, m, parts, |s| *s != PartState::Ok);
    o.raw("}\n");
    o.0
}

/// `unpack-run/1`: the parts and the files with their states, and `lost` / `mismatch` as lists of paths.
pub fn unpack_json(
    m: &Manifest,
    parts: &[PartState],
    files: &[FileState],
    dry_run: bool,
) -> String {
    let ok = parts.iter().all(|p| *p == PartState::Ok)
        && files
            .iter()
            .all(|f| matches!(f, FileState::Written | FileState::Ready));
    let mut o = Out(String::new());
    o.raw("{\"schema\":\"unpack-run/1\",\"ok\":")
        .raw(if ok { "true" } else { "false" })
        .raw(",\"dry_run\":")
        .raw(if dry_run { "true" } else { "false" });
    o.raw(",\"name\":").str(&m.name).raw(",\"parts\":");
    parts_array(&mut o, m, parts);
    o.raw(",\"files\":[");
    for (i, (f, s)) in m.files.iter().zip(files).enumerate() {
        o.raw(if i > 0 { ",{\"path\":" } else { "{\"path\":" })
            .str(&f.path)
            .raw(",\"size\":")
            .int(f.size as i64)
            .raw(",\"status\":")
            .str(s.key())
            .raw("}");
    }
    o.raw("],\"lost\":[");
    let lost: Vec<&str> = m
        .files
        .iter()
        .zip(files)
        .filter(|(_, s)| matches!(s, FileState::Lost | FileState::Mismatch))
        .map(|(f, _)| f.path.as_str())
        .collect();
    for (i, p) in lost.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(p);
    }
    o.raw("]}\n");
    o.0
}
