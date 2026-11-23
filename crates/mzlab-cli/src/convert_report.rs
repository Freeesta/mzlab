//! What `mzlab-cli file convert` says of each file it converted: spectra and `conversion/1` read back from the head of the mzML (so a file taken from the cache is
//! described the same way) and the `--report` JSON, one object per input.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::Outcome;
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
        /// `method/1.warnings`: key and place of what the reader could not read.
        notes: Vec<(String, String)>,
        /// With `--peaks mzlab`: the centroids flagged (merged, saturated).
        flagged: Option<(usize, usize)>,
        /// A partial read as the record of the run has it, once the scans were read (declared, read, cause).
        lost: Option<(usize, usize, String)>,
        /// The spectrum-type decision (always, for a file written by this version).
        spectrum_type: Option<String>,
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

/// The spectrum-type decision (`spectrum=..; written=..; basis=..`) of the input `label`, from the note of its file in the record of the run.
pub(crate) fn spectrum_type_of(run: &mzlab_core::flow::Run, label: &str) -> Option<String> {
    let note = run
        .activities
        .iter()
        .flat_map(|a| &a.files)
        .find(|(n, _)| n == label)?;
    note.1.get("spectrum_type")?.str().map(str::to_string)
}

/// The partial read (declared, read, cause) of the input `label`, from the note of its file in the record of the run: counted once the scans were read,
/// so it holds the scans left out because their data do not decode, which the `conversion/1` of the header, written before them, cannot.
pub(crate) fn lost_of(run: &mzlab_core::flow::Run, label: &str) -> Option<(usize, usize, String)> {
    let note = run
        .activities
        .iter()
        .flat_map(|a| &a.files)
        .find(|(n, _)| n == label)?;
    let n = |k: &str| note.1.get(k).and_then(|v| v.f64()).map(|x| x as usize);
    let cause = note.1.get("cause").and_then(|v| v.str())?.to_string();
    Some((n("declared")?, n("read")?, cause))
}

/// The read step through the head of an mzML.
const CHUNK: u64 = 1 << 20;

/// Number of spectra, the `conversion/1` document and the `method/1.warnings` entries of the mzML at `path`. Both sit in the header before the first spectrum, after the `method/1`
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
        .split_once("name=\"mzLab conversion/1\" value=\"")
        .and_then(|(_, r)| r.split_once('"'))
        .map(|(v, _)| mzlab_core::method::xml_unescape(v));
    (spectra, conv, notes_of(&text))
}

/// The `warnings` of the `method/1` document in the head `text` of an mzML, as (key, place).
fn notes_of(text: &str) -> Vec<(String, String)> {
    let Some(doc) = text
        .split_once("name=\"mzLab method/1\" value=\"")
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
    doc.get("warnings")
        .map(|a| a.arr())
        .unwrap_or_default()
        .iter()
        .map(|a| (s(a, "key"), s(a, "place")))
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

/// A partial read is written and said, never passed for a whole one: one warning line per file with its counts.
pub(crate) fn partial_lines(items: &[Item]) -> String {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Done {
                input, conv, lost, ..
            } => lost
                .clone()
                .or_else(|| conv.as_deref().and_then(partial_of))
                .map(|(declared, read, cause)| {
                    // an MRM sample counts its cycles (`unit` of its `conversion/1`), every other its scans
                    let unit = match conv
                        .as_deref()
                        .and_then(|c| mzlab_core::json::parse(c.as_bytes()).ok())
                        .and_then(|d| d.get("unit").and_then(|u| u.str().map(str::to_string)))
                    {
                        Some(u) if u == "cycles" => "MRM cycles",
                        _ => "scans",
                    };
                    format!(
                        "warning: {input}: partial read, {read} of {declared} {unit} ({cause})\n"
                    )
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

/// One stderr line per input with the spectrum-type decision (from the note of its file, so a file taken from the cache says it too).
pub(crate) fn spectrum_lines(items: &[Item]) -> String {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Done {
                input,
                spectrum_type: Some(t),
                ..
            } => Some(format!("spectrum type: {input}: {t}\n")),
            _ => None,
        })
        .collect()
}

/// The scans declared and read and the cause key when the `conversion/1` document `conv` says the read was partial.
pub(crate) fn partial_of(conv: &str) -> Option<(usize, usize, String)> {
    let doc = mzlab_core::json::parse(conv.as_bytes()).ok()?;
    if doc.get("partial").and_then(|p| p.bool()) != Some(true) {
        return None;
    }
    let n = |k: &str| doc.get(k).and_then(|v| v.f64()).unwrap_or(0.0) as usize;
    let cause = doc.get("loss").and_then(|s| s.str()).unwrap_or_default();
    Some((n("declared"), n("read"), cause.to_string()))
}

/// The `convert/1` document (`--report FILE`, and the stdout of `--json`): `{"schema","dry_run":false,"out","items":[...]}` with, per input,
/// `{input, output, sha256, spectra, seconds, skipped, conversion, warnings}` (`centroids` {merged, saturated} with `--peaks mzlab`, `spectrum_type`) or `{input, error, cause}`.
pub(crate) fn report(items: &[Item], out: &Path) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"convert/1\",\"dry_run\":false,\"out\":")
        .str(&out.display().to_string())
        .raw(",\"items\":[");
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
                spectrum_type,
                ..
            } => {
                o.raw("{\"input\":").str(input);
                o.raw(",\"output\":").str(output);
                o.raw(",\"sha256\":").str(sha256);
                o.raw(",\"spectra\":").int(*spectra as i64);
                o.raw(",\"seconds\":").num((seconds * 1e3).round() / 1e3);
                o.raw(if *skipped {
                    ",\"skipped\":true"
                } else {
                    ",\"skipped\":false"
                });
                if let Some(c) = conv {
                    o.raw(",\"conversion\":").raw(c);
                }
                o.raw(",\"warnings\":[");
                for (i, (k, d)) in notes.iter().enumerate() {
                    o.raw(if i > 0 { ",{\"key\":" } else { "{\"key\":" });
                    o.str(k).raw(",\"place\":").str(d).raw("}");
                }
                o.raw("]");
                if let Some((m, s)) = flagged {
                    o.raw(",\"centroids\":{\"merged\":").int(*m as i64);
                    o.raw(",\"saturated\":").int(*s as i64).raw("}");
                }
                if let Some(t) = spectrum_type {
                    o.raw(",\"spectrum_type\":").str(t);
                }
                o.raw("}");
            }
            Item::Failed { input, cause } => {
                o.raw("{\"input\":").str(input);
                let key = cause.split(' ').next().unwrap_or_default();
                o.raw(",\"error\":").str(key);
                o.raw(",\"cause\":").str(cause).raw("}");
            }
        }
    }
    o.raw("\n]}\n");
    o.0
}

/// One mzML that `--dry-run` would write.
pub(crate) struct Planned {
    pub(crate) input: String,
    pub(crate) output: String,
    pub(crate) sample_id: Option<u32>,
    /// the id of the reader that takes the dataset (`sciex-wiff`, `thermo-raw`, ...), `None` when no reader recognises it
    pub(crate) reader: Option<&'static str>,
    pub(crate) input_bytes: u64,
    pub(crate) empty: bool,
}

/// The `convert/1` document of `--dry-run`: what would be converted, nothing written.
pub(crate) fn dry_run(plan: &[Planned], out: &Path) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"convert/1\",\"dry_run\":true,\"out\":")
        .str(&out.display().to_string())
        .raw(",\"items\":[");
    for (i, p) in plan.iter().enumerate() {
        o.raw(if i > 0 { ",\n" } else { "\n" });
        o.raw("{\"input\":").str(&p.input);
        o.raw(",\"output\":").str(&p.output);
        o.raw(",\"sample_id\":");
        match p.sample_id {
            Some(id) => o.int(i64::from(id)),
            None => o.raw("null"),
        };
        o.raw(",\"reader\":");
        match p.reader {
            Some(r) => o.str(r),
            None => o.raw("null"),
        };
        o.raw(",\"input_bytes\":").int(p.input_bytes as i64);
        o.raw(if p.empty {
            ",\"empty\":true}"
        } else {
            ",\"empty\":false}"
        });
    }
    o.raw("\n]}\n");
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

/// A failure of `convert` with its own code and a hint (its `error/1` key is the generic one of the code).
pub(crate) fn fail(code: u8, msg: &str, hint: &str) -> Outcome {
    Outcome {
        code,
        stdout: Vec::new(),
        stderr: format!("error: {msg}\nhint: {hint}\n"),
        error: None,
    }
}

/// The last lines on stderr and the code: every file written, or the number that failed.
pub(crate) fn summary(items: &[Item], out: &Path, json: bool) -> Outcome {
    let skipped = items
        .iter()
        .filter(|i| matches!(i, Item::Done { skipped: true, .. }))
        .count();
    let failed = items
        .iter()
        .filter(|i| matches!(i, Item::Failed { .. }))
        .count();
    let converted = items.len() - skipped - failed;
    let stdout: String = items
        .iter()
        .filter_map(|i| match i {
            Item::Done { output, .. } => Some(format!("{output}\n")),
            Item::Failed { .. } => None,
        })
        .collect();
    // what the readers could not read (`method/1.warnings`), one note per entry, as the Method window lists it
    let notes: String = items
        .iter()
        .flat_map(|i| match i {
            Item::Done { input, notes, .. } => notes
                .iter()
                .map(|(k, d)| format!("note: {input}: {}\n", crate::error_text::method_note(k, d)))
                .collect(),
            Item::Failed { .. } => Vec::new(),
        })
        .collect();
    let partial = partial_lines(items);
    let notes = notes + &spectrum_lines(items) + &crate::convert_report::merged_lines(items);
    let (code, stderr) = if failed == 0 {
        (
            if partial.is_empty() { 0 } else { 3 },
            format!(
                "{notes}{partial}done: {converted} converted, {skipped} skipped; outputs in {}\n",
                out.display()
            ),
        )
    } else {
        let causes: String = items
            .iter()
            .filter_map(|i| match i {
                Item::Failed { input, cause } => Some(format!("error: {input}: {cause}\n")),
                Item::Done { .. } => None,
            })
            .collect();
        let line = format!(
            "error: {failed} of {} failed ({converted} converted, {skipped} skipped)\n",
            items.len()
        );
        (1, notes + &partial + &causes + &line + "hint: the other files are written; run again to retry the failed ones, the converted ones are skipped\n")
    };
    Outcome {
        code,
        stdout: if json { report(items, out) } else { stdout }.into_bytes(),
        stderr,
        error: None,
    }
}

/// `--report PATH`: the `convert/1` document written there; `Some(outcome)` (exit 2) when the file cannot be written.
pub(crate) fn write_report(path: Option<&Path>, items: &[Item], out: &Path) -> Option<Outcome> {
    let path = path?;
    let e = std::fs::write(path, report(items, out)).err()?;
    Some(fail(
        2,
        &format!("{}: {e}", path.display()),
        "--report must be a file that can be written",
    ))
}
