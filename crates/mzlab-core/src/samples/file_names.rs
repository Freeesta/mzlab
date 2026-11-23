//! The file names of the mzML files made after the sample names (`mzlab-cli file convert --name sample`): a sample name cleaned to a safe file name and the
//! clashes settled in the order of the files, each choice with the note that says it. One function for the CLI, the app and the site (route `mzml_names`).

use crate::error::{Error, Result};

/// The longest cleaned name in bytes: a file name holds 255 on the usual file systems, and `-ID.mzML` comes after it.
const MAX_BYTES: usize = 200;

/// A sample name as a safe file name: letters, digits and `. _ - + ( )` stay, anything else (spaces, path separators, `:`, quotes) becomes `_`, runs of `_`
/// collapse, the edges lose `. _ -` (no hidden file), the length is capped in bytes on a character, and a Windows device name (`CON`, `COM1`, with or without an
/// extension) gets a `_` right after it (`CON_.txt`). None when nothing is left.
pub fn clean_sample_name(name: &str) -> Option<String> {
    let mut out = String::new();
    for c in name.trim().chars() {
        let keep = c.is_alphanumeric() || matches!(c, '.' | '-' | '+' | '(' | ')');
        let c = if keep { c } else { '_' };
        if c != '_' || !out.ends_with('_') {
            out.push(c);
        }
    }
    let edge = |c: char| matches!(c, '.' | '_' | '-');
    let out = out.trim_matches(edge);
    let mut end = out.len().min(MAX_BYTES);
    while !out.is_char_boundary(end) {
        end -= 1;
    }
    let out = out[..end].trim_end_matches(edge);
    if out.is_empty() {
        return None;
    }
    let cut = out.find('.').unwrap_or(out.len());
    let dev = out[..cut].to_ascii_uppercase();
    let numbered = dev.len() == 4 && (dev.starts_with("COM") || dev.starts_with("LPT"));
    let reserved = matches!(dev.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (numbered && matches!(dev.as_bytes()[3], b'1'..=b'9'));
    Some(if reserved {
        format!("{}_{}", &out[..cut], &out[cut..])
    } else {
        out.to_string()
    })
}

/// One mzML to name: `who` (`NAME` or `NAME@ID`, for the notes), the `plain` name it would get otherwise (with its `.mzML`), the sample name and its id.
pub struct Item<'a> {
    pub who: &'a str,
    pub plain: &'a str,
    pub sample: Option<&'a str>,
    pub id: Option<u32>,
}

/// The name chosen for one file (with its `.mzML`) and the line that says why it is not the plain sample name, when it is not.
pub struct Named {
    pub out: String,
    pub note: Option<String>,
}

/// The names of `items`, in order: `SAMPLE.mzML` after the cleaned sample name; a name already taken (case aside, as on Windows and macOS: one file would replace the
/// other) gets `-ID`, then `-N` until it is free; a sample name that cleans to nothing keeps its plain name. The later file always yields to the earlier one.
pub fn mzml_names(items: &[Item]) -> Vec<Named> {
    let mut seen: Vec<String> = Vec::new();
    let mut made = Vec::new();
    for it in items {
        let clean = it.sample.and_then(clean_sample_name);
        let base = clean
            .clone()
            .unwrap_or_else(|| it.plain.trim_end_matches(".mzML").to_string());
        let free = |n: &str| !seen.contains(&n.to_lowercase());
        let mut name = base.clone();
        let mut n = 1;
        while !free(&name) {
            n += 1;
            name = match (it.id, n) {
                (Some(id), 2) => format!("{base}-{id}"),
                (Some(id), _) => format!("{base}-{id}-{}", n - 1),
                (None, _) => format!("{base}-{n}"),
            };
        }
        seen.push(name.to_lowercase());
        let out = format!("{name}.mzML");
        let who = it.who;
        let note = if it.sample.is_none() {
            None
        } else if clean.is_none() {
            Some(format!("note: {who}: the sample name is empty or has no usable characters: written as {out}"))
        } else if name != base {
            Some(format!(
                "note: {who}: the sample name {base} is already taken: written as {out}"
            ))
        } else {
            it.sample.filter(|raw| *raw != base).map(|raw| {
                format!(
                    "note: {who}: the sample name \"{raw}\" is written as {out} (a safe file name)"
                )
            })
        };
        made.push(Named { out, note });
    }
    made
}

/// The route `mzml_names`: `{"files":[{"who","plain","sample","id"}]}` → `{"files":[{"out","note"}]}`, the names of `mzml_names` (`note` null when there is none).
pub fn route(input: &str) -> Result<String> {
    let v = crate::json::parse(input.as_bytes())?;
    let rows = v
        .get("files")
        .ok_or_else(|| Error::new("error.badSamples"))?;
    fn text<'a>(r: &'a crate::json::Value, k: &str) -> Option<&'a str> {
        r.get(k).and_then(|x| x.str())
    }
    let items: Vec<Item> = rows
        .arr()
        .iter()
        .map(|r| Item {
            who: text(r, "who").unwrap_or(""),
            plain: text(r, "plain").unwrap_or(""),
            sample: text(r, "sample"),
            id: r.get("id").and_then(|x| x.f64()).map(|x| x as u32),
        })
        .collect();
    let mut j = crate::json::Out::default();
    j.raw("{\"files\":[");
    for (i, n) in mzml_names(&items).iter().enumerate() {
        j.raw(if i > 0 { ",{\"out\":" } else { "{\"out\":" })
            .str(&n.out)
            .raw(",\"note\":");
        match &n.note {
            Some(t) => j.str(t),
            None => j.raw("null"),
        };
        j.raw("}");
    }
    j.raw("]}");
    Ok(j.0)
}
