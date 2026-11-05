//! A gzip-compressed mzML or mzXML (`x.mzML.gz`, `x.mzXML.gz`, the forms public repositories serve) as a `Lettore`. The file is inflated into memory (at most `Limiti::byte`,
//! `error.memoryBudget` beyond) and read by the reader of the open format (`mzml_numpress`, which also reads plain arrays, or `mzxml`). The desktop app and the CLI (registry)
//! read them so; the page streams the gzip into OPFS instead (`mzlab_core::gunzip`), and its module `aperti` carries only the mzML voice (the fallback without OPFS).

use std::io::Read;

use flate2::read::MultiGzDecoder;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{Fiducia, FileSet, Lettore, Limiti, Membro, VoceLettore, CONTRATTO};

/// The bytes of the start of the file that `riconosci` inflates to see the root element.
const TESTA: usize = 4096;

pub const VOCE: VoceLettore = VoceLettore {
    id: "mzml-gz",
    contratto: CONTRATTO,
    estensioni: &[".mzml.gz"],
    riconosci,
    apri,
};

/// `x.mzXML.gz`: the same inflate, then the reader of `mzxml.rs` (registry of the app and the CLI only).
#[cfg_attr(not(feature = "tutte"), allow(dead_code))]
pub const VOCE_MZXML: VoceLettore = VoceLettore {
    id: "mzxml-gz",
    contratto: CONTRATTO,
    estensioni: &[".mzxml.gz"],
    riconosci: riconosci_mzxml,
    apri: apri_mzxml,
};

/// A `ReadAt` seen as a `Read`, from the start.
struct Flusso {
    dati: Box<dyn ReadAt>,
    pos: u64,
    dim: u64,
}

impl Read for Flusso {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf.len().min((self.dim - self.pos).min(1 << 20) as usize);
        if n == 0 {
            return Ok(0);
        }
        self.dati
            .read_at(self.pos, &mut buf[..n])
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        self.pos += n as u64;
        Ok(n)
    }
}

/// Yes when the member is gzip (magic bytes) and its first bytes inflate to an `<mzML` or `<indexedmzML` root.
fn riconosci(fs: &mut FileSet) -> Fiducia {
    vede(fs, ".mzml.gz", &["<mzML", "<indexedmzML"])
}

#[cfg_attr(not(feature = "tutte"), allow(dead_code))]
fn riconosci_mzxml(fs: &mut FileSet) -> Fiducia {
    vede(fs, ".mzxml.gz", &["<mzXML"])
}

fn vede(fs: &mut FileSet, suffisso: &str, radici: &[&str]) -> Fiducia {
    let Some(i) = fs.trova(suffisso) else {
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
    let _ = MultiGzDecoder::new(&head[..])
        .take(TESTA as u64)
        .read_to_end(&mut text);
    let text = String::from_utf8_lossy(&text);
    if radici.iter().any(|r| text.contains(r)) {
        Fiducia::Si
    } else if text.is_empty() {
        // gzip whose first bytes do not inflate (header only, cut): damaged, not another type
        Fiducia::Forse
    } else {
        Fiducia::No
    }
}

fn apri(fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    super::mzml_numpress::apri(gonfia(fs, ".mzml.gz", limiti)?, limiti)
}

#[cfg_attr(not(feature = "tutte"), allow(dead_code))]
fn apri_mzxml(fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    (super::mzxml::VOCE.apri)(gonfia(fs, ".mzxml.gz", limiti)?, limiti)
}

/// The set with its gzip member inflated into memory, named without `.gz`.
fn gonfia(mut fs: FileSet, suffisso: &str, limiti: &Limiti) -> Result<FileSet> {
    let i = fs
        .trova(suffisso)
        .ok_or_else(|| Error::new("err.file.notAccepted"))?;
    let m = fs.membri.swap_remove(i);
    let nome = m.nome[..m.nome.len() - ".gz".len()].to_string();
    let mut fuori = Vec::new();
    let letto = MultiGzDecoder::new(Flusso {
        dati: m.dati,
        pos: 0,
        dim: m.dim,
    })
    .take(limiti.byte + 1)
    .read_to_end(&mut fuori);
    // a cut stream (download stopped) keeps what it inflated; the spectra it lacks are counted by `mancanti`
    let lette = match letto {
        Ok(n) => n,
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof && !fuori.is_empty() => fuori.len(),
        Err(e) => return Err(Error::new("err.file.zlib").with("detail", e)),
    };
    // nothing inflated (an empty file, the magic bytes or the header alone): the page's inflate refuses it so (`rust-carica.js` `inflate`)
    if fuori.is_empty() {
        return Err(Error::new("err.file.zlib").with("detail", "truncated"));
    }
    if lette as u64 > limiti.byte {
        return Err(Error::memory_budget());
    }
    let dim = fuori.len() as u64;
    Ok(FileSet {
        membri: vec![Membro {
            nome,
            dim,
            dati: Box::new(fuori),
            sha1: None,
        }],
    })
}
