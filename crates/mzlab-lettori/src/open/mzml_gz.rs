//! A gzip-compressed mzML or mzXML (`x.mzML.gz`, `x.mzXML.gz`, the forms public repositories serve) as a `Lettore`. The file is inflated into memory (at most `Limiti::byte`,
//! `error.memoryBudget` beyond) and read by the reader of the open format (`mzml_numpress`, which also reads plain arrays, or `mzxml`). The desktop app and the CLI (registry)
//! read them so; the page streams the gzip into OPFS instead (`mzlab_core::gunzip`), and its module `aperti` carries only the mzML voice (the fallback without OPFS).

use std::io::Read;

use mzlab_core::error::{Error, Result};
use mzlab_core::gunzip::Members;
use mzlab_core::idxread::ReadAt;
use mzlab_core::readers::{Fiducia, FileSet, Limits, Member, Reader, ReaderEntry, CONTRATTO};

/// The bytes of the start of the file that `riconosci` inflates to see the root element.
const HEAD: usize = 4096;

pub const ENTRY: ReaderEntry = ReaderEntry {
    id: "mzml-gz",
    contratto: CONTRATTO,
    estensioni: &[".mzml.gz"],
    riconosci: recognize,
    apri: open,
};

/// `x.mzXML.gz`: the same inflate, then the reader of `mzxml.rs` (registry of the app and the CLI only).
#[cfg_attr(not(feature = "tutte"), allow(dead_code))]
pub const ENTRY_MZXML: ReaderEntry = ReaderEntry {
    id: "mzxml-gz",
    contratto: CONTRATTO,
    estensioni: &[".mzxml.gz"],
    riconosci: recognize_mzxml,
    apri: open_mzxml,
};

/// A `ReadAt` seen as a `Read`, from the start.
struct Stream {
    data: Box<dyn ReadAt>,
    pos: u64,
    dim: u64,
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf.len().min((self.dim - self.pos).min(1 << 20) as usize);
        if n == 0 {
            return Ok(0);
        }
        self.data
            .read_at(self.pos, &mut buf[..n])
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        self.pos += n as u64;
        Ok(n)
    }
}

/// Yes when the member is gzip (magic bytes) and its first bytes inflate to an `<mzML` or `<indexedmzML` root.
fn recognize(fs: &mut FileSet) -> Fiducia {
    sees(fs, ".mzml.gz", &["<mzML", "<indexedmzML"])
}

#[cfg_attr(not(feature = "tutte"), allow(dead_code))]
fn recognize_mzxml(fs: &mut FileSet) -> Fiducia {
    sees(fs, ".mzxml.gz", &["<mzXML"])
}

fn sees(fs: &mut FileSet, suffix: &str, roots: &[&str]) -> Fiducia {
    let Some(i) = fs.find(suffix) else {
        return Fiducia::No;
    };
    let head = fs.testa(i, 64 << 10);
    // too short for the magic bytes: a damaged gzip of this name, which `apri` refuses with `err.file.zlib` as the page's inflate does
    if head.len() < 2 {
        return Fiducia::Forse;
    }
    if head[..2] != [0x1f, 0x8b] {
        return Fiducia::No;
    }
    let mut text = Vec::new();
    // a short or cut stream still gives its first bytes; the error is the caller's at `apri`
    let _ = Members::new(&head[..])
        .take(HEAD as u64)
        .read_to_end(&mut text);
    let text = String::from_utf8_lossy(&text);
    if roots.iter().any(|r| text.contains(r)) {
        Fiducia::Si
    } else if text.is_empty() {
        // gzip whose first bytes do not inflate (header only, cut): damaged, not another type
        Fiducia::Forse
    } else {
        Fiducia::No
    }
}

fn open(fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    super::mzml_numpress::apri(inflate(fs, ".mzml.gz", limits)?, limits)
}

#[cfg_attr(not(feature = "tutte"), allow(dead_code))]
fn open_mzxml(fs: FileSet, limits: &Limits) -> Result<Box<dyn Reader>> {
    (super::mzxml::ENTRY.apri)(inflate(fs, ".mzxml.gz", limits)?, limits)
}

/// The set with its gzip member inflated into memory, named without `.gz`.
fn inflate(mut fs: FileSet, suffix: &str, limits: &Limits) -> Result<FileSet> {
    let i = fs
        .find(suffix)
        .ok_or_else(|| fs.not_accepted())?;
    let m = fs.membri.swap_remove(i);
    let name = m.nome[..m.nome.len() - ".gz".len()].to_string();
    let mut outside = Vec::new();
    let read_one = Members::new(std::io::BufReader::with_capacity(
        1 << 16,
        Stream {
            data: m.dati,
            pos: 0,
            dim: m.dim,
        },
    ))
    .take(limits.byte + 1)
    .read_to_end(&mut outside);
    // a cut stream (download stopped) keeps what it inflated; the spectra it lacks are counted by `mancanti`
    let read_ones = match read_one {
        Ok(n) => n,
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof && !outside.is_empty() => {
            outside.len()
        }
        Err(e) => return Err(Error::new("err.file.zlib").with("detail", e)),
    };
    // nothing inflated (an empty file, the magic bytes or the header alone): the page's inflate refuses it so (`rust-carica.js` `inflate`)
    if outside.is_empty() {
        return Err(Error::new("err.file.zlib").with("detail", "truncated"));
    }
    if read_ones as u64 > limits.byte {
        return Err(Error::memory_budget());
    }
    let dim = outside.len() as u64;
    Ok(FileSet {
        membri: vec![Member {
            nome: name,
            dim,
            dati: Box::new(outside),
            sha1: None,
        }],
    })
}
