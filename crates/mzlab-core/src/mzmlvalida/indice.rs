//! The index of an `indexedmzML`: a reader that hashes what the parser has consumed, and the comparison of offsets, `indexListOffset` and `fileChecksum`
//! with what the pass saw.

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read};

use sha1::{Digest as _, Sha1};

use super::{Problem, Severity};

/// A buffered reader whose SHA-1 covers exactly the bytes the parser has consumed, so the checksum can be read at the `<fileChecksum>` tag
/// (the file checksum covers every byte up to and including that opening tag).
pub(super) struct Tee<R: Read> {
    inner: BufReader<R>,
    sha: Sha1,
}

impl<R: Read> Tee<R> {
    pub fn new(r: R) -> Self {
        Tee {
            inner: BufReader::with_capacity(1 << 16, r),
            sha: Sha1::new(),
        }
    }

    /// The SHA-1 (hex) of the bytes consumed so far.
    pub fn hex(&self) -> String {
        self.sha
            .clone()
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

impl<R: Read> Read for Tee<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = self.fill_buf()?.len().min(out.len());
        out[..n].copy_from_slice(&self.inner.buffer()[..n]);
        self.consume(n);
        Ok(n)
    }
}

impl<R: Read> BufRead for Tee<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        let b = self.inner.buffer();
        self.sha.update(&b[..amt.min(b.len())]);
        self.inner.consume(amt);
    }
}

/// What the pass collects about the index.
#[derive(Default)]
pub(super) struct Idx {
    /// (id, byte position of the `<spectrum` tag) in file order, and the same for chromatograms.
    pub seen_spectra: Vec<(String, u64)>,
    pub seen_chroms: Vec<(String, u64)>,
    /// The `<offset>` entries of each `<index name=...>`.
    pub spectra: Vec<(String, u64)>,
    pub chroms: Vec<(String, u64)>,
    /// Name of the `<index>` being read and the `idRef` of the `<offset>` being read.
    pub list: String,
    pub id_ref: String,
    pub list_at: Option<u64>,
    pub list_offset: Option<u64>,
    pub checksum: Option<String>,
    pub computed: Option<String>,
    pub present: bool,
}

fn problem(path: &str, code: &'static str, message: String) -> Problem {
    Problem {
        severity: Severity::Error,
        path: path.to_string(),
        code,
        message,
    }
}

fn compare(
    seen: &[(String, u64)],
    listed: &[(String, u64)],
    what: &str,
    path: &str,
    out: &mut Vec<Problem>,
) {
    if seen.len() != listed.len() {
        out.push(problem(
            path,
            "index.count",
            format!(
                "the index lists {} {what} entries and the file holds {}",
                listed.len(),
                seen.len()
            ),
        ));
    }
    let at: HashMap<&str, u64> = seen.iter().map(|(i, p)| (i.as_str(), *p)).collect();
    for (id, off) in listed {
        match at.get(id.as_str()) {
            None => out.push(problem(
                path,
                "index.unknownId",
                format!("the index lists a {what} \"{id}\" that the file does not hold"),
            )),
            Some(p) if p != off => out.push(problem(
                path,
                "index.offset",
                format!("the offset of {what} \"{id}\" is {off} and it starts at {p}"),
            )),
            _ => {}
        }
    }
}

impl Idx {
    /// The findings of the index once the whole file has been read.
    pub fn check(&self, out: &mut Vec<Problem>) {
        if !self.present {
            return;
        }
        compare(
            &self.seen_spectra,
            &self.spectra,
            "spectrum",
            "/indexedmzML/indexList",
            out,
        );
        compare(
            &self.seen_chroms,
            &self.chroms,
            "chromatogram",
            "/indexedmzML/indexList",
            out,
        );
        match (self.list_at, self.list_offset) {
            (Some(at), Some(off)) if at != off => out.push(problem(
                "/indexedmzML/indexListOffset",
                "index.listOffset",
                format!("indexListOffset is {off} and the index list starts at {at}"),
            )),
            (None, _) => out.push(problem("/indexedmzML", "required", "no indexList".into())),
            (_, None) => out.push(problem(
                "/indexedmzML",
                "required",
                "no indexListOffset".into(),
            )),
            _ => {}
        }
        match (&self.checksum, &self.computed) {
            (Some(c), Some(h)) if !c.trim().eq_ignore_ascii_case(h) => out.push(problem(
                "/indexedmzML/fileChecksum",
                "index.checksum",
                format!(
                    "fileChecksum is {} and the SHA-1 of the file is {h}",
                    c.trim()
                ),
            )),
            (None, _) | (_, None) => out.push(problem(
                "/indexedmzML",
                "required",
                "no fileChecksum".into(),
            )),
            _ => {}
        }
    }
}
