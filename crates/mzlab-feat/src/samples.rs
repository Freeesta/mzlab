//! Metadata of the samples of a series, `campioni/1` (`docs/schemi/campioni-1.json`): a type, a time, a replicate, a group and a concentration for every file.
//! It is proposed from the file names with the rules of `mzlab_core::project` and edited by the user; the SHA-256 of its canonical JSON goes in the `.mzflow`.
//! Changing a metadatum does not change the detection (the `.mzfeat` is the same): only the views and the identities read it.

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::project::{guess_conc, guess_sample};

pub const SCHEMA: &str = "campioni/1";
/// Types of a sample, in the order of the schema.
pub const KINDS: [&str; 7] = ["sample", "blank", "dark", "t0", "qc", "std", "calibration"];

/// One file of the series.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub file: String,
    pub sha256: String,
    /// One of `KINDS`.
    pub kind: String,
    pub time_min: Option<f64>,
    /// 1-based among the files with the same type, time and concentration.
    pub replicate: u32,
    pub group: String,
    pub conc: Option<f64>,
    pub unit: Option<String>,
    /// 1-based position of the injection.
    pub order: u32,
    /// The file is an internal standard (normalisation by the standard).
    pub istd: bool,
    pub note: String,
}

fn has_word(low: &str, word: &str) -> bool {
    low.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| w == word || (w.starts_with(word) && w[word.len()..].chars().all(|c| c.is_ascii_digit())))
}

/// The type of a file from its name: the rules of `guess_sample` plus `dark`, `qc`, `calibration` and the time zero.
fn kind_of(name: &str, guess: &'static str, time: Option<f64>) -> &'static str {
    let low = name.to_lowercase();
    match guess {
        "blank" => "blank",
        "standard" => ["cal", "calib", "calibr", "calibrazione", "taratura", "curva"]
            .iter()
            .find(|w| has_word(&low, w))
            .map_or("std", |_| "calibration"),
        _ if has_word(&low, "dark") => "dark",
        _ if has_word(&low, "qc") => "qc",
        _ if time == Some(0.0) => "t0",
        _ => "sample",
    }
}

/// The metadata proposed for `files` (`(name, SHA-256)` in the order of injection, as in the table).
pub fn propose(files: &[(String, String)]) -> Vec<Sample> {
    let mut out: Vec<Sample> = Vec::with_capacity(files.len());
    for (i, (name, sha)) in files.iter().enumerate() {
        let (_, time, guess) = guess_sample(name);
        let kind = kind_of(name, guess, time);
        let (conc, unit) = match guess_conc(name) {
            Some((v, u)) if guess == "standard" => (Some(v), u),
            _ => (None, None),
        };
        let replicate = 1 + out
            .iter()
            .filter(|s| s.kind == kind && s.time_min == time && s.conc == conc)
            .count() as u32;
        let group = if matches!(kind, "sample" | "std" | "calibration") { "" } else { kind };
        out.push(Sample {
            file: name.clone(),
            sha256: sha.clone(),
            kind: kind.to_string(),
            time_min: time,
            replicate,
            group: group.to_string(),
            conc,
            unit,
            order: i as u32 + 1,
            istd: false,
            note: String::new(),
        });
    }
    out
}

fn opt_str(j: &mut Out, v: Option<&str>) {
    match v {
        Some(s) => j.str(s),
        None => j.raw("null"),
    };
}

/// The canonical JSON of `campioni/1`: fixed order of the keys, so the same metadata give the same bytes (and the same SHA-256).
pub fn to_json(samples: &[Sample]) -> String {
    let mut j = Out::default();
    j.raw("{\"schema\":").str(SCHEMA).raw(",\"campioni\":[");
    for (i, s) in samples.iter().enumerate() {
        j.raw(if i > 0 { ",{\"file\":" } else { "{\"file\":" }).str(&s.file);
        j.raw(",\"sha256\":").str(&s.sha256).raw(",\"tipo\":").str(&s.kind);
        j.raw(",\"tempo_min\":").opt(s.time_min);
        j.raw(",\"replica\":").int(s.replicate as i64).raw(",\"gruppo\":").str(&s.group);
        j.raw(",\"concentrazione\":").opt(s.conc).raw(",\"unita\":");
        opt_str(&mut j, s.unit.as_deref());
        j.raw(",\"ordine_iniezione\":").int(s.order as i64);
        j.raw(",\"istd\":").raw(if s.istd { "true" } else { "false" });
        j.raw(",\"note\":").str(&s.note).raw("}");
    }
    j.raw("]}");
    j.0
}

/// SHA-256 of the canonical JSON: what the `campioni` operation of the `.mzflow` records and what the identities (L8) are tied to.
pub fn digest(samples: &[Sample]) -> String {
    sha256_hex(to_json(samples).as_bytes())
}

/// The TSV with the columns of the metadata table of mzmine (`filename`, then the same fields as `campioni/1`): one line per file, tab-separated.
pub fn to_tsv(samples: &[Sample]) -> String {
    let mut t = String::from("file\tsha256\ttipo\ttempo_min\treplica\tgruppo\tconcentrazione\tunita\tordine_iniezione\tistd\tnote\n");
    let num = |v: Option<f64>| v.map_or(String::new(), |x| format!("{x}"));
    for s in samples {
        let clean = |x: &str| x.replace(['\t', '\n', '\r'], " ");
        t.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            clean(&s.file),
            s.sha256,
            s.kind,
            num(s.time_min),
            s.replicate,
            clean(&s.group),
            num(s.conc),
            s.unit.as_deref().unwrap_or(""),
            s.order,
            if s.istd { "si" } else { "no" },
            clean(&s.note)
        ));
    }
    t
}

fn bad() -> Error {
    Error::new("error.badCampioni")
}

fn text(v: &Value, k: &str) -> Result<String> {
    Ok(v.get(k).and_then(|x| x.str()).ok_or_else(bad)?.to_string())
}

fn number(v: &Value, k: &str) -> Result<Option<f64>> {
    match v.get(k) {
        None => Ok(None),
        Some(x) if x.is_null() => Ok(None),
        Some(x) => x.f64().map(Some).ok_or_else(bad),
    }
}

/// Reads a `campioni/1` document; the type must be one of `KINDS`, the order and the replicate whole numbers from 1.
pub fn from_json(bytes: &[u8]) -> Result<Vec<Sample>> {
    let doc = parse(bytes)?;
    if doc.get("schema").and_then(|v| v.str()) != Some(SCHEMA) {
        return Err(bad());
    }
    let mut out = Vec::new();
    for v in doc.get("campioni").map(|a| a.arr()).unwrap_or(&[]) {
        let kind = text(v, "tipo")?;
        let whole = |k: &str| -> Result<u32> {
            let x = number(v, k)?.ok_or_else(bad)?;
            (x >= 1.0 && x.fract() == 0.0 && x < 4e9).then_some(x as u32).ok_or_else(bad)
        };
        if !KINDS.contains(&kind.as_str()) {
            return Err(bad());
        }
        out.push(Sample {
            file: text(v, "file")?,
            sha256: text(v, "sha256")?,
            kind,
            time_min: number(v, "tempo_min")?,
            replicate: whole("replica")?,
            group: text(v, "gruppo")?,
            conc: number(v, "concentrazione")?,
            unit: v.get("unita").and_then(|x| x.str()).map(str::to_string),
            order: whole("ordine_iniezione")?,
            istd: v.get("istd").and_then(|x| x.bool()).ok_or_else(bad)?,
            note: text(v, "note")?,
        });
    }
    Ok(out)
}
