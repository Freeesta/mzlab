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
        /// `metodo/1.avvisi`: key and place of what the reader could not read.
        notes: Vec<(String, String)>,
        /// With `--peaks mzlab`: the centroids flagged (merged, saturated).
        flagged: Option<(usize, usize)>,
    },
    Failed {
        input: String,
        cause: String,
    },
}

/// The centroids flagged (merged, saturated) of our own peak picking on the input `label`, from the note of its file in the record of the run (a file
/// taken from the cache keeps the note of the run that wrote it).
pub(crate) fn flagged_of(run: &mzlab_core::flow::Run, label: &str) -> Option<(usize, usize)> {
    let note = run
        .activities
        .iter()
        .flat_map(|a| &a.files)
        .find(|(n, _)| n == label)?;
    let n = |k: &str| note.1.get(k).and_then(|v| v.f64()).map(|x| x as usize);
    Some((n("merged")?, n("saturated")?))
}

/// The read step through the head of an mzML.
const CHUNK: u64 = 1 << 20;

/// Number of spectra, the `conv/1` document and the `metodo/1.avvisi` entries of the mzML at `path`. Both sit in the header before the first spectrum, after the `metodo/1`
/// document, which can be many MiB: the head is read until the `spectrumList` opening (or the `chromatogramList`, or the end), not a fixed size.
pub(crate) fn head_of(path: &Path) -> (usize, Option<String>, Vec<(String, String)>) {
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
        .map(|(v, _)| mzlab_core::method::xml_unescape(v));
    (spectra, conv, notes_of(&text))
}

/// The `avvisi` of the `metodo/1` document in the head `text` of an mzML, as (key, place).
fn notes_of(text: &str) -> Vec<(String, String)> {
    let Some(doc) = text
        .split_once("name=\"mzLab metodo/1\" value=\"")
        .and_then(|(_, r)| r.split_once('"'))
        .map(|(v, _)| mzlab_core::method::xml_unescape(v))
        .and_then(|v| mzlab_core::json::parse(v.as_bytes()).ok())
    else {
        return Vec::new();
    };
    let s = |a: &mzlab_core::json::Value, k: &str| {
        a.get(k)
            .and_then(|v| v.str())
            .unwrap_or_default()
            .to_string()
    };
    doc.get("avvisi")
        .map(|a| a.arr())
        .unwrap_or_default()
        .iter()
        .map(|a| (s(a, "chiave"), s(a, "dove")))
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

/// A partial read is written and said, never passed for a whole one: one warning line per file with its counts.
pub(crate) fn partial_lines(items: &[Item]) -> String {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Done {
                input,
                conv: Some(c),
                ..
            } => partial_of(c).map(|(declared, read, cause)| {
                format!("warning: {input}: partial read, {read} of {declared} scans ({cause})\n")
            }),
            _ => None,
        })
        .collect()
}

/// One stderr line per input with centroids flagged merged: a notice (`warn.centroid.merged`), not an answer, that leaves the exit code alone.
pub(crate) fn merged_lines(items: &[Item]) -> String {
    let text = crate::error_text::catalog_text("warn.centroid.merged").unwrap_or_default();
    items
        .iter()
        .filter_map(|i| match i {
            Item::Done {
                input,
                flagged: Some((m, _)),
                ..
            } if *m > 0 => Some(format!(
                "warning: {input}: {m} centroids merged: {text} (warn.centroid.merged)\n"
            )),
            _ => None,
        })
        .collect()
}

/// The scans declared and read and the cause key when the `conv/1` document `conv` says the read was partial.
pub(crate) fn partial_of(conv: &str) -> Option<(usize, usize, String)> {
    let doc = mzlab_core::json::parse(conv.as_bytes()).ok()?;
    if doc.get("parziale").and_then(|p| p.bool()) != Some(true) {
        return None;
    }
    let n = |k: &str| doc.get(k).and_then(|v| v.f64()).unwrap_or(0.0) as usize;
    let cause = doc.get("perdita").and_then(|s| s.str()).unwrap_or_default();
    Some((n("dichiarate"), n("lette"), cause.to_string()))
}

/// The `--report` document: a JSON array with `{ingresso, uscita, sha256, spettri, secondi, saltato, conv/1, avvisi}` (and `centroidi` {merged, saturated}
/// with `--peaks mzlab`) or `{ingresso, errore, causa}` per input.
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
                notes,
                flagged,
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
                o.raw(",\"avvisi\":[");
                for (i, (k, d)) in notes.iter().enumerate() {
                    o.raw(if i > 0 {
                        ",{\"chiave\":"
                    } else {
                        "{\"chiave\":"
                    });
                    o.str(k).raw(",\"dove\":").str(d).raw("}");
                }
                o.raw("]");
                if let Some((m, s)) = flagged {
                    o.raw(",\"centroidi\":{\"merged\":").int(*m as i64);
                    o.raw(",\"saturated\":").int(*s as i64).raw("}");
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
    let o = mzlab_core::mzmlwrite::compression_options(kind)
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
