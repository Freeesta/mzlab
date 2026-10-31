//! A gzip-compressed mzML (`x.mzML.gz`, the form public repositories serve) as a `Lettore`. The file is inflated into memory (at most `Limiti::byte`,
//! `error.memoryBudget` beyond) and read by the reader of the open mzML (`mzml_numpress`, which also reads plain arrays). Not in the engine of the page: the module
//! `aperti` and the desktop app and CLI (registry) carry it.

use std::io::Read;

use flate2::read::MultiGzDecoder;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{Fiducia, FileSet, Lettore, Limiti, Membro, VoceLettore, CONTRATTO};

const SUFFISSO: &str = ".mzml.gz";
/// The bytes of the start of the file that `riconosci` inflates to see the root element.
const TESTA: usize = 4096;

pub const VOCE: VoceLettore = VoceLettore {
    id: "mzml-gz",
    contratto: CONTRATTO,
    estensioni: &[".mzml.gz"],
    riconosci,
    apri,
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
    let Some(i) = fs.trova(SUFFISSO) else {
        return Fiducia::No;
    };
    let head = fs.testa(i, 64 << 10);
    if head.len() < 2 || head[..2] != [0x1f, 0x8b] {
        return Fiducia::No;
    }
    let mut text = Vec::new();
    // a short or cut stream still gives its first bytes; the error is the caller's at `apri`
    let _ = MultiGzDecoder::new(&head[..])
        .take(TESTA as u64)
        .read_to_end(&mut text);
    let text = String::from_utf8_lossy(&text);
    if text.contains("<mzML") || text.contains("<indexedmzML") {
        Fiducia::Si
    } else {
        Fiducia::No
    }
}

fn apri(mut fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    let i = fs
        .trova(SUFFISSO)
        .ok_or_else(|| Error::new("err.file.notAccepted"))?;
    let m = fs.membri.swap_remove(i);
    let nome = m.nome[..m.nome.len() - ".gz".len()].to_string();
    let mut fuori = Vec::new();
    let lette = MultiGzDecoder::new(Flusso {
        dati: m.dati,
        pos: 0,
        dim: m.dim,
    })
    .take(limiti.byte + 1)
    .read_to_end(&mut fuori)
    .map_err(|e| Error::new("err.file.zlib").with("detail", e))?;
    if lette as u64 > limiti.byte {
        return Err(Error::memory_budget());
    }
    let dim = fuori.len() as u64;
    super::mzml_numpress::apri(
        FileSet {
            membri: vec![Membro {
                nome,
                dim,
                dati: Box::new(fuori),
                sha1: None,
            }],
        },
        limiti,
    )
}
