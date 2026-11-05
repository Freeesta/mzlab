//! Streaming gunzip for the page: a `.mzML.gz` is inflated piece by piece (pieces of the File in, pieces of the mzML out) so the worker of the engine writes the mzML
//! into OPFS without ever holding it whole (docs/agenti/lettori-aperti.md, «mzML gzip»). Several gzip members one after the other (bgzip, `cat a.gz b.gz`) are one stream, as
//! `Membri` reads them in `aperti/mzml_gz.rs` for the desktop app and the CLI: the bytes out are the same. Zero bytes after a whole member (a file padded to a block size)
//! end the stream, as GNU gzip reads them; a byte other than zero after that padding is `err.file.zlib`.

use std::io::{self, BufRead, Read, Write};

use flate2::write::GzDecoder;

use crate::error::{Error, Result};

/// The input given to the decoder at a time: a piece of highly repetitive XML inflates about 1000 times, so one push never makes more than ~64 MB.
const PASSO: usize = 64 << 10;

pub struct Gunzip {
    /// the decoder of the current member; its output is taken by `prendi`
    dec: GzDecoder<Vec<u8>>,
    /// the current member ended: the next bytes start another member or are padding
    tra: bool,
    /// zero bytes of padding seen after a member: only zeros may follow
    zeri: bool,
    /// bytes of the mzML given out so far
    fuori: u64,
    /// the most bytes the mzML may have (`err.memory.over4gb` beyond)
    tetto: u64,
}

impl Gunzip {
    pub fn new(tetto: u64) -> Self {
        Gunzip {
            dec: GzDecoder::new(Vec::new()),
            tra: false,
            zeri: false,
            fuori: 0,
            tetto,
        }
    }

    /// The mzML bytes that `input` (the next piece of the gzip file) completes; empty while a block is still open.
    pub fn push(&mut self, input: &[u8]) -> Result<Vec<u8>> {
        let zlib = |e: io::Error| Error::new("err.file.zlib").with("detail", e);
        let mut out = Vec::new();
        for p in input.chunks(PASSO) {
            let mut b = p;
            while !b.is_empty() {
                if self.tra {
                    let z = b.iter().take_while(|x| **x == 0).count();
                    self.zeri |= z > 0;
                    b = &b[z..];
                    if b.is_empty() {
                        break;
                    }
                    if self.zeri {
                        return Err(Error::new("err.file.zlib")
                            .with("detail", "data after the zero padding"));
                    }
                    out.extend(self.prendi()?);
                    self.dec = GzDecoder::new(Vec::new());
                    self.tra = false;
                }
                match self.dec.write(b).map_err(zlib)? {
                    // the member ended (header, data and CRC read): what follows is the next member or padding
                    0 => {
                        self.dec.try_finish().map_err(zlib)?;
                        self.tra = true;
                    }
                    n => b = &b[n..],
                }
            }
            out.extend(self.prendi()?);
        }
        Ok(out)
    }

    /// The last bytes once the whole file was pushed, and whether the stream ended well (false: the file is cut short, as a stopped download; what was inflated stays
    /// and the reader counts the spectra it lacks, as `mzml_gz` does).
    pub fn finish(&mut self) -> Result<(Vec<u8>, bool)> {
        let intero = self.tra || self.dec.try_finish().is_ok();
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

/// The members of a gzip file one after the other as one stream, for the app and the CLI (`aperti/mzml_gz.rs`): the bytes `Gunzip` gives the page, zero padding
/// after a member included.
pub struct Membri<R: BufRead> {
    dec: Option<flate2::bufread::GzDecoder<R>>,
}

impl<R: BufRead> Membri<R> {
    pub fn new(r: R) -> Self {
        Membri {
            dec: Some(flate2::bufread::GzDecoder::new(r)),
        }
    }
}

impl<R: BufRead> Read for Membri<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let Some(d) = self.dec.as_mut() else {
                return Ok(0);
            };
            let n = d.read(buf)?;
            if n > 0 || buf.is_empty() {
                return Ok(n);
            }
            let mut r = self.dec.take().map(|d| d.into_inner()).expect("decoder");
            if !fine(&mut r)? {
                self.dec = Some(flate2::bufread::GzDecoder::new(r));
            }
        }
    }
}

/// After a whole member: true when the stream ends here (end of the input, or zero bytes up to it), false when another member starts.
fn fine(r: &mut impl BufRead) -> io::Result<bool> {
    let mut zeri = false;
    loop {
        let b = r.fill_buf()?;
        if b.is_empty() {
            return Ok(true);
        }
        let z = b.iter().take_while(|x| **x == 0).count();
        if z == 0 {
            return if zeri {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "data after the zero padding",
                ))
            } else {
                Ok(false)
            };
        }
        zeri = true;
        r.consume(z);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        flate2::read::MultiGzDecoder::new(&file[..])
            .read_to_end(&mut atteso)
            .unwrap();
        assert_eq!(app(&file).unwrap(), atteso);
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

    fn app(file: &[u8]) -> std::io::Result<Vec<u8>> {
        let mut out = Vec::new();
        Membri::new(file).read_to_end(&mut out).map(|_| out)
    }

    #[test]
    fn zero_padding_after_a_member_ends_the_stream_on_both_sides() {
        let a = testo(5_000);
        let mut file = gz(&a);
        file.extend(gz(&testo(7)));
        let atteso = app(&file).unwrap();
        file.extend([0u8; 1000]);
        assert_eq!(app(&file).unwrap(), atteso);
        for pezzo in [1, 7, 4096, file.len()] {
            assert_eq!(
                a_pezzi(&file, pezzo),
                (atteso.clone(), true),
                "piece {pezzo}"
            );
        }
    }

    #[test]
    fn data_after_the_padding_is_an_error_on_both_sides() {
        let mut file = gz(&testo(100));
        file.extend([0u8; 10]);
        file.extend(gz(&testo(3)));
        assert!(app(&file).is_err());
        for pezzo in [1, 7, file.len()] {
            let mut g = Gunzip::new(u64::MAX);
            let e = file.chunks(pezzo).map(|p| g.push(p)).find_map(|r| r.err());
            assert_eq!(e.map(|e| e.key), Some("err.file.zlib"), "piece {pezzo}");
        }
    }
}
