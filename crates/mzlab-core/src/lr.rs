//! The open files of a session as the page lists them, without Python: the entry of the loading table (`App.files`) and the items of
//! `Session` (`explore.Session._open`), built from the index of the file and its mzML (`iteminfo`, `sniff`, `project`, `mzmlhead`).

use crate::error::Result;
use crate::idxread::{Index, ReadAt};
use crate::iteminfo::{facts_from, file_parts, item_info, Meta};
use crate::json::{self, Out, Value};
use crate::mzmlhead::Mzml;
use crate::project::{guess_conc, guess_sample};
use crate::sniff::sniff;

fn base_name(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

/// One entry of the file list: `{name, size, sha256, time, type, conc, cunit}` and what the content says (`sniff`, with the hint of the name).
pub fn file_entry<R: ReadAt, M: ReadAt>(
    idx: &mut Index<R>,
    m: &mut Mzml<M>,
    name: &str,
    size: u64,
) -> Result<String> {
    let (_, t, typ) = guess_sample(name);
    let c = if typ == "standard" {
        guess_conc(name)
    } else {
        None
    };
    let n_chrom = idx
        .run
        .get("n_chromatograms")
        .and_then(Value::f64)
        .unwrap_or(0.0) as i64;
    let scans_end = idx.scans.last().map_or(0, |s| s.end);
    let facts = facts_from(m, scans_end, n_chrom, idx.scans.is_empty())?;
    let sniffed = sniff(idx, m, &facts, name)?;
    let sha = idx
        .source
        .get("sha256")
        .and_then(Value::str)
        .unwrap_or("")
        .to_string();
    let mut o = Out::default();
    o.raw("{\"name\":")
        .str(name)
        .raw(",\"size\":")
        .int(size as i64)
        .raw(",\"sha256\":")
        .str(&sha)
        .raw(",\"time\":")
        .opt(t)
        .raw(",\"type\":")
        .str(typ);
    o.raw(",\"conc\":")
        .opt(c.as_ref().map(|c| c.0))
        .raw(",\"cunit\":");
    match c.and_then(|c| c.1) {
        Some(u) => o.str(&u),
        None => o.raw("null"),
    };
    o.raw(",").raw(sniffed.trim_start_matches('{'));
    Ok(o.0)
}

/// What the sample table (or a saved notebook) says about one file of a session; absent = guessed from the name.
#[derive(Debug, Clone, Default)]
pub struct Sample {
    pub label: Option<String>,
    pub typ: Option<String>,
    pub time: Option<f64>,
    pub conc: Option<f64>,
    pub cunit: Option<String>,
    pub rep: Option<String>,
}

/// An item of the session: the JSON of `Item.info()` and whether its file is high resolution.
pub struct OpenItem {
    pub info: String,
    pub hr: bool,
}

/// The items of one file: one for a plain file, one per part for a mixed file (`Session._open`).
pub fn open_items<R: ReadAt, M: ReadAt>(
    idx: &mut Index<R>,
    m: &mut Mzml<M>,
    name: &str,
    s: &Sample,
) -> Result<Vec<OpenItem>> {
    let (label0, t0, typ0) = guess_sample(base_name(name));
    let label = s.label.clone().filter(|l| !l.is_empty()).unwrap_or(label0);
    let time = s.time.or(t0);
    let typ = s
        .typ
        .clone()
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| typ0.to_string());
    let n_chrom = idx
        .run
        .get("n_chromatograms")
        .and_then(Value::f64)
        .unwrap_or(0.0) as i64;
    let scans_end = idx.scans.last().map_or(0, |x| x.end);
    let facts = facts_from(m, scans_end, n_chrom, true)?;
    let sha = idx
        .source
        .get("sha256")
        .and_then(Value::str)
        .unwrap_or("")
        .to_string();
    let parts = file_parts(&idx.scans, facts.srm > 0);
    let one = |idx: &mut Index<R>,
               file: String,
               part: Option<&crate::iteminfo::Part>|
     -> Result<OpenItem> {
        let shown = match part {
            Some(p) => {
                let n = crate::iteminfo::part_label(p, &idx.scans);
                let old = format!(" \u{b7} {}", p.tag);
                let base = label.strip_suffix(old.as_str()).unwrap_or(&label);
                if base.ends_with(&n) {
                    base.to_string()
                } else {
                    format!("{base} \u{b7} {n}")
                }
            }
            _ => label.clone(),
        };
        let meta = Meta {
            file,
            label: shown,
            sha256: sha.clone(),
            time,
            typ: typ.clone(),
            conc: s.conc,
            cunit: s.cunit.clone().filter(|u| !u.is_empty()),
            rep: s.rep.clone().filter(|r| !r.is_empty()),
        };
        let info = item_info(idx, &facts, &meta, part)?;
        let v = json::parse(info.as_bytes())?;
        let hr = ["prof1", "prof2"].iter().any(|k| {
            v.get(k)
                .and_then(|p| p.get("hr"))
                .and_then(Value::bool)
                .unwrap_or(false)
        });
        Ok(OpenItem { info, hr })
    };
    if parts.is_empty() {
        return Ok(vec![one(idx, name.to_string(), None)?]);
    }
    parts
        .iter()
        .map(|p| one(idx, format!("{name}#{}", p.tag), Some(p)))
        .collect()
}
