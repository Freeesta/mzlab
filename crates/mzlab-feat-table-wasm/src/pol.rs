//! What a table file is written with, one rule for `flow run` and the browser: the samples as `campioni/1` and the polarity, the recipe of the table when it names one, else the sign of the
//! polarities of the files of its lineage, positive when neither says.
use mzlab_core::campioni;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::parse;
use mzlab_feat::facts::file_facts;
use mzlab_feat::samples::{self, Sample};
use mzlab_feat::store::read_table_from;

/// `-1` or `1`; `indexes` are the `.mzidx` of the files the table was made from (none when they are not at hand).
pub fn of_table<R: ReadAt, I: ReadAt>(r: &mut R, size: u64, indexes: &[Index<I>]) -> Result<i32> {
    let (_, meta) = read_table_from(r, size)?;
    let recipe = meta.get("recipe").and_then(|v| v.str()).unwrap_or("");
    if recipe.contains("polarity: Some(") && !recipe.contains("polarity: Some(0)") {
        return Ok(if recipe.contains("polarity: Some(-1)") {
            -1
        } else {
            1
        });
    }
    let sum: i64 = indexes
        .iter()
        .map(|ix| i64::from(file_facts(ix).polarity))
        .sum();
    Ok(if sum < 0 { -1 } else { 1 })
}

/// The polarity the recipe of the table says (`-1` or `1`; positive when it names none): what `table.match` writes its MGF in.
pub fn recipe_polarity<R: ReadAt>(r: &mut R, size: u64) -> Result<i32> {
    let (_, meta) = read_table_from(r, size)?;
    let recipe = meta.get("recipe").and_then(|v| v.str()).unwrap_or("");
    Ok(if recipe.contains("polarity: Some(-1)") {
        -1
    } else {
        1
    })
}

/// The samples as `campioni/1` text, whichever of the schemas the file has (`campioni/3` too: its batch and matrix are read by `filterspec`).
pub fn samples_v1(bytes: &[u8]) -> Result<String> {
    let doc = parse(bytes)?;
    if !matches!(
        doc.get("schema").and_then(|v| v.str()),
        Some("campioni/2") | Some(campioni::SCHEMA3)
    ) {
        return Ok(String::from_utf8_lossy(bytes).into_owned());
    }
    let mut out: Vec<Sample> = Vec::new();
    for c in campioni::from_json(bytes)? {
        let kind = match c.kind.as_str() {
            "infusion" => campioni::tipo_v1(&c.file).to_string(),
            k => k.to_string(),
        };
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
    Ok(samples::to_json(&out))
}
