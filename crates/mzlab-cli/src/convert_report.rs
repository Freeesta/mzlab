//! What `mzlab-cli convert` says of each file it converted: spectra and `conv/1` read back from the head of the mzML (so a file taken from the cache is
//! described the same way) and the `--report` JSON, one object per input.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use mzlab_core::json::Out;

/// How much of the head of an mzML is looked at: the header, the `<run>` parameters and the opening of the spectrum list.
const HEAD: u64 = 1 << 20;

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

/// Number of spectra and the `conv/1` document of the mzML at `path`.
pub(crate) fn head_of(path: &Path) -> (usize, Option<String>) {
    let mut text = Vec::new();
    if let Ok(f) = File::open(path) {
        let _ = f.take(HEAD).read_to_end(&mut text);
    }
    let text = String::from_utf8_lossy(&text);
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
    let (z, n) = match kind {
        "standard" => ("2", "false"),
        "smallest" => ("2", "true"),
        "none" => ("0", "false"),
        _ => {
            return Err(format!(
                "--compression {kind}: expected standard, smallest or none"
            ))
        }
    };
    Ok([format!("conv.zlib_level={z}"), format!("conv.numpress={n}")])
}
