//! The samples of a series as the table of features reads them: a type, a time, a replicate, a group and a concentration for every file. It is a reduced
//! view of the `samples/1` document (`docs/schemi/samples-1.json`), the only format on disk and on the wire: `to_json` writes one, `from_json` reads one.
//! It is proposed from the file names by `mzlab_core::samples::propose` (the one parser of `samples/1`) and edited by the user; the SHA-256 of its canonical JSON goes in the `.mzflow`.
//! Changing a metadatum does not change the detection (the `.mzfeat` is the same): only the views and the identities read it.

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::project::guess_sample;
use mzlab_core::samples;

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

/// The metadata proposed for `files` (`(name, SHA-256)` in the order of injection, as in the table): the roles and times of
/// `mzlab_core::samples::propose` in the vocabulary of `samples/1` (an infusion takes the type and time of the old rules, a QC its time; no series; replicates by type, time and concentration).
pub fn propose(files: &[(String, String)]) -> Vec<Sample> {
    let named: Vec<(String, String, String)> = files
        .iter()
        .map(|(n, s)| (n.clone(), s.clone(), String::new()))
        .collect();
    let mut out: Vec<Sample> = Vec::with_capacity(files.len());
    for c in samples::propose(&named) {
        let (kind, time_min) = match c.kind.as_str() {
            "infusion" => (
                samples::table_kind_of_name(&c.file),
                guess_sample(&c.file).1,
            ),
            "qc" => ("qc", guess_sample(&c.file).1),
            k => (k, c.time_min),
        };
        let replicate = 1 + out
            .iter()
            .filter(|s| s.kind == kind && s.time_min == time_min && s.conc == c.conc)
            .count() as u32;
        let group = if matches!(kind, "sample" | "std" | "calibration") {
            ""
        } else {
            kind
        };
        out.push(Sample {
            kind: kind.to_string(),
            time_min,
            replicate,
            group: group.to_string(),
            conc: c.conc,
            unit: c.unit,
            order: c.order,
            istd: false,
            note: String::new(),
            file: c.file,
            sha256: c.sha256,
        });
    }
    out
}

/// The `samples/1` entry of one sample (a time is written by the user, so confirmed).
fn core_of(s: &Sample) -> samples::Sample {
    let mut c = samples::Sample::new(&s.file, &s.sha256, s.order);
    c.kind = s.kind.clone();
    c.set_time(s.time_min, "min");
    c.time_confirmed = s.time_min.is_some();
    c.replicate = s.replicate;
    c.conc = s.conc;
    c.unit = s.unit.clone();
    c.istd = s.istd;
    c.note = s.note.clone();
    c
}

/// The canonical `samples/1` of the samples (the group is derived from the type when read back, so it is not written).
pub fn to_json(samples: &[Sample]) -> String {
    samples::to_json(&samples.iter().map(core_of).collect::<Vec<_>>())
}

/// SHA-256 of the canonical JSON: what the `samples` operation of the `.mzflow` records and what the identities (L8) are tied to.
pub fn digest(samples: &[Sample]) -> String {
    sha256_hex(to_json(samples).as_bytes())
}

/// The TSV with the columns of the metadata table of mzmine (`filename`, then the same fields as `samples/1`): one line per file, tab-separated.
pub fn to_tsv(samples: &[Sample]) -> String {
    let mut t = String::from("file\tsha256\tkind\ttime_min\treplicate\tgroup\tconcentration\tconcentration_unit\tinjection_order\tistd\tnote\n");
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
    Error::new("error.badSamples")
}

/// Reads a `samples/1` document as the table of features sees it: an infusion takes the type its name would give (the table has no infusion role) and the
/// group is the type, but for a sample, a standard or a calibration; a type outside `KINDS` is `error.badSamples`.
pub fn from_json(bytes: &[u8]) -> Result<Vec<Sample>> {
    let mut out = Vec::new();
    for c in samples::from_json(bytes)? {
        let kind = match c.kind.as_str() {
            "infusion" => samples::table_kind_of_name(&c.file).to_string(),
            k => k.to_string(),
        };
        if !KINDS.contains(&kind.as_str()) {
            return Err(bad());
        }
        let group = if matches!(kind.as_str(), "sample" | "std" | "calibration") {
            String::new()
        } else {
            kind.clone()
        };
        out.push(Sample {
            file: c.file,
            sha256: c.sha256,
            kind,
            time_min: c.time_min,
            replicate: c.replicate,
            group,
            conc: c.conc,
            unit: c.unit,
            order: c.order,
            istd: c.istd,
            note: c.note,
        });
    }
    Ok(out)
}

/// The samples of the file as the engine of the statistics reads them: the role in `kind`, the condition (`series`) in `group`.
pub fn samples_of(bytes: &[u8]) -> Result<Vec<Sample>> {
    Ok(samples::from_json(bytes)?
        .into_iter()
        .map(|c| Sample {
            kind: c.role().to_string(),
            group: c.series.clone(),
            file: c.file,
            sha256: c.sha256,
            time_min: c.time_min,
            replicate: c.replicate,
            conc: c.conc,
            unit: c.unit,
            order: c.order,
            istd: c.istd,
            note: c.note,
        })
        .collect())
}

/// The sample of each column of the table (`inputs` of its meta), by name else by SHA-256; a column the samples do not know is no condition and no blank.
pub fn columns(inputs: &[(String, String)], all: &[Sample]) -> Vec<Sample> {
    let unknown = |file: &str, sha256: &str| Sample {
        file: file.to_string(),
        sha256: sha256.to_string(),
        kind: "other".into(),
        time_min: None,
        replicate: 1,
        group: String::new(),
        conc: None,
        unit: None,
        order: 1,
        istd: false,
        note: String::new(),
    };
    inputs
        .iter()
        .map(|(name, sha)| {
            let found = all
                .iter()
                .find(|s| s.file == *name)
                .or_else(|| all.iter().find(|s| !sha.is_empty() && s.sha256 == *sha));
            found.cloned().unwrap_or_else(|| unknown(name, sha))
        })
        .collect()
}
