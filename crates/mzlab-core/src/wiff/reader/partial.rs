//! The header of the mzML of an injection and the partial identity of a SCIEX pair opened for one injection: its sample alone is read, so its source is told by the sizes of both files, the header and
//! the directory of the container, and the bytes of the streams and of the `.wiff.scan` range that sample owns, never by a hash of the two whole files.
//! Same file and same sample give the same identity; another sample gives another one.

use super::*;
use crate::budget::sha256_hex;

/// The `sha1` a member of the pair carries to ask for the partial identity (no hash of the whole file is taken).
pub const MARK: &str = "partial";

/// Prefix of the identity: it is never a plain hash.
pub const PREFIX: &str = "partial(sample=";

/// Is `sha1` of a member the mark that asks for the partial identity, or an identity already taken (`partial(sample=ID):HEX`, handed on by the caller)?
pub fn is_mark(sha1: &str) -> bool {
    sha1 == MARK || sha1.starts_with(PREFIX)
}

/// The identity to give back for a member's `sha1`: empty to compute it, else the one handed on; `None` when the member does not ask for it.
pub(super) fn asked(sha1: Option<&str>) -> Option<String> {
    sha1.filter(|s| is_mark(s)).map(|s| {
        if s == MARK {
            String::new()
        } else {
            s.to_string()
        }
    })
}

/// `Reader::empty_sample` of a pair: blocks in the index mean chromatograms (MRM), so the sample is not empty and its DAD stream is not read to say so.
pub(super) fn empty_of(l: &mut WiffReader, c: usize) -> Result<bool> {
    let n = l.scan_len;
    if l.ready(c).is_ok() && l.part(c)?.w.records.iter().any(|r| !r.cut(n)) {
        return Ok(false);
    }
    let none = l.samples().get(c).is_some_and(|s| s.scansioni == 0);
    Ok(none && Reader::chromatograms(l, c)?.is_empty() && !l.losses_of(c)?.partial())
}

/// The listing entry of an injection `open_bare` leaves unread.
pub(super) fn unopened(sample: u32, scan_len: u64) -> Wiff {
    Wiff::refused(
        sample,
        scan_len,
        Error::new("err.file.wiff").with("detail", "not opened"),
    )
}

impl WiffReader {
    /// `partial(sample=ID):HEX`, the identity of the sample `c`'s ranges (`sha256` of what the module comment lists).
    pub(super) fn partial_identity(&mut self, c: usize) -> Result<String> {
        let id = self.part(c)?.w.sample;
        // an identity the caller took when it listed the sample (the same file, the same ranges) is not taken again
        if let Some(t) = self
            .partial
            .as_ref()
            .filter(|t| t.starts_with(&format!("{PREFIX}{id})")))
        {
            return Ok(t.clone());
        }
        let mut buf = format!(
            "mzlab-partial-identity/1\nwiff={}\nscan={}\nsample={id}\n",
            self.ole.size(),
            self.scan_len
        )
        .into_bytes();
        buf.extend(self.ole.head());
        buf.extend(self.ole.directory_digest().bytes());
        let own = format!("{SAMPLES}Sample{id}/");
        let paths: Vec<String> = self
            .ole
            .paths_list()
            .filter(|p| p.starts_with(&own))
            .map(str::to_string)
            .collect();
        for p in paths {
            let bytes = self.ole.read(&p).unwrap_or_default();
            buf.extend(format!("{p}\0{}\n", bytes.len()).bytes());
            buf.extend(bytes);
        }
        let recs = &self.part(c)?.w.records;
        let (from, to) = (
            recs.iter().map(|r| r.at).min().unwrap_or(0),
            recs.iter()
                .map(|r| r.data_end())
                .max()
                .unwrap_or(0)
                .min(self.scan_len),
        );
        buf.extend(format!("range={from}-{to}\n").bytes());
        let mut at = from;
        while at < to {
            let n = usize::try_from((to - at).min(1 << 20)).unwrap_or(1 << 20);
            let mut b = vec![0u8; n];
            self.scan.read_at(at, &mut b)?;
            buf.extend(b);
            at += n as u64;
        }
        Ok(format!("{PREFIX}{id}):{}", sha256_hex(&buf)))
    }
}

/// The header of the mzML with the `metodo/1` document in the run: the method (pumps, gradient, detectors) travels in the mzML, as for `.raw` and `.D`,
/// so the page reads it back from a converted file.
pub(super) fn with_method(l: &mut WiffReader, c: usize) -> Result<Header> {
    let mut h = file_header(l, c)?;
    let doc = l.method(c)?;
    if !doc.is_empty() {
        h.run_params
            .push((crate::method::USERPARAM.to_string(), doc));
    }
    Ok(h)
}

/// The header of the mzML of injection `c`, with the SHA-1 of the `.wiff.scan`.
pub(super) fn file_header(l: &mut WiffReader, c: usize) -> Result<Header> {
    l.ready(c)?;
    // the SHA-1 of the whole `.wiff` is taken here, for the header of one mzML, never when the pair is opened; an injection opened alone has its partial identity instead
    let ident = if l.partial.is_some() {
        Some(l.partial_identity(c)?)
    } else {
        None
    };
    if let Some(p) = l.parts.get_mut(c).filter(|_| l.partial.is_none())
        && p.w.wiff_sha1.is_none()
    {
        p.w.wiff_sha1 = Some(l.ole.sha1_hex());
    }
    if l.scan_sha1.is_none() && l.partial.is_none() {
        let win = &mut Window::new(&mut l.scan, l.scan_len);
        l.scan_sha1 = Some(sha1_hex(win, l.scan_len).ok());
    }
    let sha = l.scan_sha1.clone().flatten();
    let w = &l.part(c)?.w;
    #[cfg(feature = "estesi")]
    let mut h = w.header_estesi(&l.nome, sha);
    #[cfg(not(feature = "estesi"))]
    let mut h = w.header(&l.nome, sha, false);
    if let Some(id) = ident {
        h.source_sha1.clear();
        h.run_params.push(("mzlab:source-identity".into(), id));
    }
    // a section the index does not list: the run says that its name, method and times were inferred from an indexed sample
    if let Some(d) = w.inferred_from {
        h.run_params.push((
            "mzlab:wiff-unindexed".into(),
            format!("name, method and times inferred from sample {d}"),
        ));
    }
    Ok(h)
}
