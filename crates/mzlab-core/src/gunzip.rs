//! Streaming gunzip for the page: a `.mzML.gz` is inflated piece by piece (pieces of the File in, pieces of the mzML out) so the worker of the engine writes the mzML
//! into OPFS without ever holding it whole (docs/agenti/lettori-aperti.md, «mzML gzip»). Several gzip members one after the other (bgzip, `cat a.gz b.gz`) are one stream, as
//! `MultiGzDecoder` reads them in `aperti/mzml_gz.rs` for the desktop app and the CLI: the bytes out are the same.

use std::io::Write;

use flate2::write::MultiGzDecoder;

use crate::error::{Error, Result};

/// The input given to the decoder at a time: a piece of highly repetitive XML inflates about 1000 times, so one push never makes more than ~64 MB.
const PASSO: usize = 64 << 10;

pub struct Gunzip {
    dec: MultiGzDecoder<Vec<u8>>,
    /// bytes of the mzML given out so far
    fuori: u64,
    /// the most bytes the mzML may have (`err.memory.over4gb` beyond)
    tetto: u64,
}

impl Gunzip {
    pub fn new(tetto: u64) -> Self {
        Gunzip {
            dec: MultiGzDecoder::new(Vec::new()),
            fuori: 0,
            tetto,
        }
    }

    /// The mzML bytes that `input` (the next piece of the gzip file) completes; empty while a block is still open.
    pub fn push(&mut self, input: &[u8]) -> Result<Vec<u8>> {
        for p in input.chunks(PASSO) {
            self.dec
                .write_all(p)
                .map_err(|e| Error::new("err.file.zlib").with("detail", e))?;
        }
        self.prendi()
    }

    /// The last bytes once the whole file was pushed, and whether the stream ended well (false: the file is cut short, as a stopped download; what was inflated stays
    /// and the reader counts the spectra it lacks, as `mzml_gz` does).
    pub fn finish(&mut self) -> Result<(Vec<u8>, bool)> {
        let intero = self.dec.try_finish().is_ok();
        Ok((self.prendi()?, intero))
    }

    pub fn written(&self) -> u64 {
        self.fuori
    }

    fn prendi(&mut self) -> Result<Vec<u8>> {
        let out = std::mem::take(self.dec.get_mut());
        self.fuori += out.len() as u64;
        if self.fuori > self.tetto {
            return Err(Error::new("err.memory.over4gb"));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::MultiGzDecoder as Lettore;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Read;

    fn gz(dati: &[u8]) -> Vec<u8> {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(dati).unwrap();
        e.finish().unwrap()
    }

    fn testo(n: usize) -> Vec<u8> {
        (0..n)
            .map(|i| format!("<spectrum index=\"{i}\" id=\"scan={i}\"/>\n"))
            .collect::<String>()
            .into_bytes()
    }

    fn a_pezzi(file: &[u8], pezzo: usize) -> (Vec<u8>, bool) {
        let mut g = Gunzip::new(u64::MAX);
        let mut out = Vec::new();
        for p in file.chunks(pezzo) {
            out.extend(g.push(p).unwrap());
        }
        let (resto, intero) = g.finish().unwrap();
        out.extend(resto);
        (out, intero)
    }

    #[test]
    fn same_bytes_as_the_reader_of_the_app_for_any_piece_size() {
        let a = testo(20_000);
        let b = testo(3_000);
        let mut file = gz(&a);
        file.extend(gz(&b));
        let mut atteso = Vec::new();
        Lettore::new(&file[..]).read_to_end(&mut atteso).unwrap();
        assert_eq!(atteso.len(), a.len() + b.len());
        for pezzo in [1, 7, 4096, 1 << 20, file.len()] {
            let (out, intero) = a_pezzi(&file, pezzo);
            assert!(intero);
            assert_eq!(out, atteso, "piece {pezzo}");
        }
    }

    #[test]
    fn a_cut_file_keeps_what_it_inflated() {
        let file = gz(&testo(20_000));
        let (out, intero) = a_pezzi(&file[..file.len() / 2], 1000);
        assert!(!intero);
        assert!(!out.is_empty());
        assert!(testo(20_000).starts_with(&out));
    }

    #[test]
    fn not_gzip_and_over_the_cap_are_errors() {
        let mut g = Gunzip::new(u64::MAX);
        assert_eq!(
            g.push(b"<mzML>not gzip</mzML>").unwrap_err().key,
            "err.file.zlib"
        );
        let mut g = Gunzip::new(1000);
        let e = g.push(&gz(&testo(1000))).unwrap_err();
        assert_eq!(e.key, "err.memory.over4gb");
    }
}
