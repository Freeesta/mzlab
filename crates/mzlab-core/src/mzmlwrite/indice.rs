//! The `indexedmzML` wrapper of `MzmlWriter`: byte positions of every spectrum and chromatogram, the index list and the SHA-1 file checksum
//! (PSI mzML 1.1 indexed schema). Used only by `MzmlWriter::indicizzato`; the plain writer never builds it.

use std::io::Write;

use sha1::{Digest as _, Sha1};

use super::{io, Header, MzmlWriter, Opzioni, Result};

/// The positions and the running checksum of an indexed mzML being written.
pub(super) struct Indice {
    /// Bytes written so far.
    pub pos: u64,
    pub spectra: Vec<(String, u64)>,
    pub chroms: Vec<(String, u64)>,
    sha: Sha1,
}

impl Indice {
    pub fn new() -> Self {
        Indice {
            pos: 0,
            spectra: Vec::new(),
            chroms: Vec::new(),
            sha: Sha1::new(),
        }
    }

    /// Counts and hashes `b`, which the writer is about to write.
    pub fn feed(&mut self, b: &[u8]) {
        self.pos += b.len() as u64;
        self.sha.update(b);
    }

    /// The text after `</mzML>`: index list, its offset and the checksum of every byte up to and including the opening `<fileChecksum>` tag.
    pub fn tail(mut self) -> String {
        let mut lists = String::new();
        let mut n = 0;
        for (name, v) in [("spectrum", &self.spectra), ("chromatogram", &self.chroms)] {
            if v.is_empty() {
                continue;
            }
            n += 1;
            lists += &format!("<index name=\"{name}\">");
            for (id, off) in v {
                lists += &format!("<offset idRef=\"{}\">{off}</offset>", super::esc(id));
            }
            lists += "</index>";
        }
        if n == 0 {
            // the schema wants one index even when the run holds no spectrum and no chromatogram
            n = 1;
            lists = "<index name=\"spectrum\"/>".into();
        }
        let list_at = self.pos;
        let mut s = format!("<indexList count=\"{n}\">{lists}</indexList>\n<indexListOffset>{list_at}</indexListOffset>\n<fileChecksum>");
        self.sha.update(s.as_bytes());
        let hex: String = self
            .sha
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        s += &hex;
        s += "</fileChecksum>\n</indexedmzML>\n";
        s
    }
}

impl<W: Write> MzmlWriter<W> {
    /// As `con_opzioni`, wrapped in `indexedmzML`: PSI-MS CV 4.2.6, byte offsets of every spectrum and chromatogram, SHA-1 `fileChecksum`, and one
    /// `processingMethod` for each of `Header::steps` after the conversion.
    pub fn indicizzato(out: W, h: &Header, opz: Opzioni) -> Result<Self> {
        Self::avvia(out, h, opz, true)
    }

    /// Writes `b`, counting and hashing it when the file is indexed.
    pub(super) fn put(&mut self, b: &[u8]) -> Result<()> {
        if let Some(i) = &mut self.idx {
            i.feed(b);
        }
        self.out.write_all(b).map_err(io)
    }

    /// Notes that a spectrum (or chromatogram) called `id` starts at the current position.
    pub(super) fn mark(&mut self, spectrum: bool, id: &str) {
        if let Some(i) = &mut self.idx {
            let v = if spectrum {
                &mut i.spectra
            } else {
                &mut i.chroms
            };
            v.push((id.to_string(), i.pos));
        }
    }

    /// After `</mzML>`: the index list and the checksum, when the file is indexed.
    pub(super) fn close_index(&mut self) -> Result<()> {
        match self.idx.take() {
            Some(i) => self.out.write_all(i.tail().as_bytes()).map_err(io),
            None => Ok(()),
        }
    }
}
