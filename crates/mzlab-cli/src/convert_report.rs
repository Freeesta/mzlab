//! What `mzlab-cli convert` says of each file it converted: spectra and `conv/1` read back from the head of the mzML (so a file taken from the cache is
//! described the same way) and the `--report` JSON, one object per input.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use mzlab_core::json::Out;

/// One input of the conversion as `--report` writes it.
pub(crate) enum Item {
    Done {
        input: String,
        output: String,
        sha256: String,
        spectra: usize,
        seconds: f64,
        skipped: bool,
        conv: Option<String>,
    },
    Failed {
        input: String,
        cause: String,
    },
}

/// The read step through the head of an mzML.
const CHUNK: u64 = 1 << 20;

/// Number of spectra and the `conv/1` document of the mzML at `path`. Both sit in the header before the first spectrum, after the `metodo/1`
/// document, which can be many MiB: the head is read until the `spectrumList` opening (or the `chromatogramList`, or the end), not a fixed size.
pub(crate) fn head_of(path: &Path) -> (usize, Option<String>) {
    let mut bytes = Vec::new();
    if let Ok(mut f) = File::open(path) {
        loop {
            let from = bytes.len().saturating_sub(64);
            match (&mut f).take(CHUNK).read_to_end(&mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let new = &bytes[from..];
            let end = |m: &[u8]| new.windows(m.len()).position(|w| w == m);
            if let Some(i) = end(b"<spectrumList count=\"") {
                // the count closes within a few bytes of its opening
                if new[i..].iter().skip(21).any(|&b| b == b'"') {
                    break;
                }
            } else if end(b"<chromatogramList").is_some() {
                break;
            }
        }
    }
    let text = String::from_utf8_lossy(&bytes);
    let spectra = text
        .split_once("<spectrumList count=\"")
        .and_then(|(_, r)| r.split_once('"'))
        .and_then(|(n, _)| n.parse().ok())
        .unwrap_or(0);
    let conv = text
        .split_once("name=\"mzLab conv/1\" value=\"")
        .and_then(|(_, r)| r.split_once('"'))
        .map(|(v, _)| mzlab_core::metodo::xml_unescape(v));
    (spectra, conv)
}

/// The `--report` document: a JSON array with `{ingresso, uscita, sha256, spettri, secondi, saltato, conv/1}` or `{ingresso, errore, causa}` per input.
pub(crate) fn report(items: &[Item]) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, it) in items.iter().enumerate() {
        o.raw(if i > 0 { ",\n" } else { "\n" });
        match it {
            Item::Done {
                input,
                output,
                sha256,
                spectra,
                seconds,
                skipped,
                conv,
            } => {
                o.raw("{\"ingresso\":").str(input);
                o.raw(",\"uscita\":").str(output);
                o.raw(",\"sha256\":").str(sha256);
                o.raw(",\"spettri\":").int(*spectra as i64);
                o.raw(",\"secondi\":").num((seconds * 1e3).round() / 1e3);
                o.raw(if *skipped {
                    ",\"saltato\":true"
                } else {
                    ",\"saltato\":false"
                });
                if let Some(c) = conv {
                    o.raw(",\"conv/1\":").raw(c);
                }
                o.raw("}");
            }
            Item::Failed { input, cause } => {
                o.raw("{\"ingresso\":").str(input);
                let key = cause.split(' ').next().unwrap_or_default();
                o.raw(",\"errore\":").str(key);
                o.raw(",\"causa\":").str(cause).raw("}");
            }
        }
    }
    o.raw("\n]\n");
    o.0
}

/// The `--set` pairs of a compression kind (the three choices of the download dialog): standard = zlib, smallest = zlib and MS-Numpress, none = raw arrays.
pub(crate) fn compression_sets(kind: &str) -> Result<[String; 2], String> {
    let o = mzlab_core::mzmlwrite::opzioni_compressione(kind)
        .ok_or_else(|| format!("--compression {kind}: expected standard, smallest or none"))?;
    let (z, n) = (o.livello, o.numpress);
    Ok([format!("conv.zlib_level={z}"), format!("conv.numpress={n}")])
}

/// The name of a gzip-compressed mzML without the `.gz` (`x.mzML.gz` -> `x.mzML`), any other name as it is.
pub(crate) fn sans_gz(name: &str) -> &str {
    name.strip_suffix(".gz")
        .filter(|n| n.to_lowercase().ends_with(".mzml"))
        .unwrap_or(name)
}
