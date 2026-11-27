//! One entry of the list of feature tables (`tables/1`, `docs/schemi/tables-1.json`): the facts of one `.mzfeat` that the Flow column and
//! `mzlab-cli flow tables` show, read from its meta alone (no column is read) plus, when known, the step that made it (from the `prov/1` of the run
//! folder, or given by the page for a table in OPFS). The same function answers the site, the app and the CLI, so the bytes are the same.

use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::json::{Out, Value};

use crate::store_read::open;

/// The step that made a table, as `prov/1` or the live scheduler knows it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Made {
    pub instance: String,
    /// `id@version`.
    pub step: String,
    /// `ok` or `cached`.
    pub status: String,
    pub ms: f64,
    pub warnings: usize,
}

impl Made {
    /// From `{instance, step, status?, ms?, warnings?}` (what the page gives for a table of OPFS); `None` without an instance.
    pub fn from_json(v: &Value) -> Option<Made> {
        let s = |k: &str| v.get(k).and_then(|x| x.str()).map(str::to_string);
        Some(Made {
            instance: s("instance")?,
            step: s("step").unwrap_or_default(),
            status: s("status").unwrap_or_else(|| "ok".into()),
            ms: v.get("ms").and_then(|x| x.f64()).unwrap_or(0.0),
            warnings: v.get("warnings").and_then(|x| x.f64()).unwrap_or(0.0) as usize,
        })
    }

    /// The activity of `instance` in a parsed `prov/1`.
    pub fn from_prov(prov: &Value, instance: &str) -> Option<Made> {
        let a = prov.get("activity")?.get(instance)?;
        Some(Made {
            instance: instance.to_string(),
            step: a.get("step")?.str()?.to_string(),
            status: if a.get("cached").and_then(|x| x.bool()) == Some(true) {
                "cached"
            } else {
                "ok"
            }
            .to_string(),
            ms: a.get("ms").and_then(|x| x.f64()).unwrap_or(0.0),
            warnings: a.get("warnings").map_or(0, |w| w.arr().len()),
        })
    }
}

/// The instance a run-folder file belongs to: its name without the extension, or without `-K` for one file of a per-file step; the longest
/// instance of the `prov/1` that fits.
pub fn instance_of(prov: &Value, file: &str) -> Option<String> {
    let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
    let ids = prov.get("activity")?.entries();
    let fits = |id: &str| {
        stem == id
            || stem
                .strip_prefix(id)
                .and_then(|r| r.strip_prefix('-'))
                .is_some_and(|k| !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()))
    };
    ids.iter()
        .map(|(id, _)| id.as_str())
        .filter(|id| fits(id))
        .max_by_key(|id| id.len())
        .map(str::to_string)
}

/// `-1`, `1`, or `0` when the recipe of a table does not say (`polarity: Some(..)`; `Some(0)` is "both").
pub fn recipe_polarity(recipe: &str) -> i32 {
    if recipe.contains("polarity: Some(") && !recipe.contains("polarity: Some(0)") {
        if recipe.contains("polarity: Some(-1)") {
            -1
        } else {
            1
        }
    } else {
        0
    }
}

/// The `tables/1` entry of the `.mzfeat` `name` of `size` bytes (JSON text, no newline): name, bytes, rows, files, polarity (when the recipe says),
/// whether it carries identities and a product view, the recipe fingerprint and crate version of its meta, then the step that made it when known.
pub fn entry<R: ReadAt + ?Sized>(
    name: &str,
    r: &mut R,
    size: u64,
    made: Option<&Made>,
) -> Result<String> {
    let meta = open(r, size, "table")?.meta;
    let int = |k: &str| meta.get(k).and_then(|v| v.f64()).unwrap_or(0.0) as i64;
    let text = |k: &str| meta.get(k).and_then(|v| v.str()).unwrap_or("");
    let mut o = Out::default();
    o.raw("{\"name\":").str(name);
    o.raw(",\"bytes\":").int(size as i64);
    o.raw(",\"rows\":").int(int("rows"));
    o.raw(",\"files\":").int(int("files"));
    let pol = recipe_polarity(text("recipe"));
    if pol != 0 {
        o.raw(",\"polarity\":").int(i64::from(pol));
    }
    o.raw(",\"identity\":").raw(bool_text(int("identity") == 1));
    o.raw(",\"tp\":").raw(bool_text(meta.get("tp").is_some()));
    o.raw(",\"recipe_hash\":").str(text("recipe_hash"));
    o.raw(",\"crate\":").str(text("crate"));
    if let Some(m) = made {
        o.raw(",\"instance\":").str(&m.instance);
        o.raw(",\"step\":").str(&m.step);
        o.raw(",\"status\":").str(&m.status);
        o.raw(",\"ms\":").num((m.ms * 10.0).round() / 10.0);
        o.raw(",\"warnings\":").int(m.warnings as i64);
    }
    o.raw("}");
    Ok(o.0)
}

fn bool_text(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

/// The `tables/1` document: `entries` (texts of `entry`) in the order given, then the files that could not be read as `{name, error_key}`.
pub fn document(entries: &[String], skipped: &[(String, String)]) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"tables/1\",\"tables\":[");
    o.raw(&entries.join(","));
    o.raw("],\"skipped\":[");
    for (i, (name, key)) in skipped.iter().enumerate() {
        o.raw(if i > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(name)
            .raw(",\"error_key\":")
            .str(key)
            .raw("}");
    }
    o.raw("]}");
    o.0
}

#[cfg(test)]
#[path = "tables_tests.rs"]
mod tests;
