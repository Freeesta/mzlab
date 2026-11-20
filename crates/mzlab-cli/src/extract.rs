//! What `trace`, `spectrum` and `file info` share: opening one sample of a file as the site does (index, chromatogram headers, channels), a small
//! argument reader, and the helpers that turn a route's `{"rt":[..],"y":[..]}` text into a `trace/1`-style document or a 2-column CSV without
//! touching the number tokens (the bytes of the route are the bytes of the CLI).

use std::fs::File;
use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::iteminfo::Facts;
use mzlab_core::json::Out;
use mzlab_core::mzmlhead::{Chrom, Mzml};
use mzlab_core::routes::Routes;

/// One sample of a file, open the way the page opens it: its routes, and the mzML (the file itself, or the shell the reader builds for a vendor sample).
pub(crate) struct Sample {
    pub name: String,
    /// The id of the sample read (`None` for an mzML, which holds one).
    pub sample: Option<u32>,
    /// `Format::id` of the reader (`mzml`, `thermo-raw`, ...), and (id, name, scans) of every sample the reader lists (none for an mzML).
    pub container: String,
    pub listing: Vec<(u32, String, usize)>,
    /// A line for stderr when the file has several samples and none was asked for.
    pub note: String,
    pub routes: Routes<File>,
    pub mzml: Mzml<Box<dyn ReadAt>>,
    pub chroms: Vec<Chrom>,
}

pub(crate) fn is_mzml(file: &Path) -> bool {
    file.to_string_lossy().to_lowercase().ends_with(".mzml")
}

pub(crate) fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// `f` with a scratch folder for the index, removed afterwards.
pub(crate) fn with_tmp<T>(tag: &str, f: impl FnOnce(&Path) -> Result<T>) -> Result<T> {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-{tag}{k}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp)
        .map_err(io)
        .and_then(|_| f(&tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    r
}

/// The mzML of the sample read (the file itself, or the shell the reader builds for a vendor sample) and the position of that sample in `ids`.
fn open_mzml(
    file: &Path,
    l: Option<&mut Box<dyn mzlab_core::readers::Reader>>,
    label: &str,
    ids: &[u32],
) -> Result<(Mzml<Box<dyn ReadAt>>, usize)> {
    let Some(l) = l else {
        let f = File::open(file).map_err(io)?;
        let size = f.metadata().map_err(io)?.len();
        return Ok((Mzml::new(Box::new(f), size), 0));
    };
    let k = crate::flow_set::sample_index(label, ids)?;
    // the chromatograms (SIM, SRM, UV) come from the mzML shell the site reads (`readers_mzml::guscio`)
    let shell = mzlab_core::readers_mzml::guscio(l.as_mut(), k)?;
    let size = shell.len() as u64;
    Ok((Mzml::new(Box::new(shell), size), k))
}

/// The stderr line of a file with several samples when none was asked for.
fn several_note(name: &str, ids: &[u32], k: usize) -> String {
    if ids.len() < 2 {
        return String::new();
    }
    let list = ids
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "note: {name} has {} samples (ids {list}): this is sample {}; --sample ID for another\n",
        ids.len(),
        ids[k]
    )
}

/// Sample `sample` of `file` (the first when `None`), its index built in `tmp`. An id the file does not have is `err.sample.unknown` (exit 5).
pub(crate) fn open_sample(file: &Path, sample: Option<u32>, tmp: &Path) -> Result<Sample> {
    let name = file
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let mzml = is_mzml(file);
    let label = sample.map_or(String::new(), |id| format!("{name}@{id}"));
    // the sample is checked before its index is built: the ids of the file name the others
    let mut l = match mzml {
        true => None,
        false => Some(mzlab_lettori::apri(
            crate::flow_set::file_set(file)?,
            &mzlab_core::readers::Limits::desktop(),
        )?),
    };
    let ids: Vec<u32> = l
        .iter()
        .flat_map(|l| l.samples().iter().map(|s| s.id))
        .collect();
    let container = l.as_ref().map_or("mzml", |l| l.format().id()).to_string();
    let listing = l
        .iter()
        .flat_map(|l| {
            l.samples()
                .iter()
                .map(|s| (s.id, s.nome.clone(), s.scansioni))
        })
        .collect();
    if let Some(id) = sample {
        mzlab_core::readers::selection::resolve(&ids, &[id])?;
    }
    let idx = tmp.join("0.mzidx");
    crate::flow_pace::build_index_of(
        file,
        &label,
        &idx,
        mzlab_core::budget_auto::native_index_budget(),
    )?;
    let mut routes = Routes::new(crate::open(&idx)?);
    let end = routes.idx.scans.iter().map(|s| s.end).max().unwrap_or(0);
    let (mut m, k) = open_mzml(file, l.as_mut(), &label, &ids)?;
    let chroms = m.chromatograms(end)?;
    let facts: Facts = mzlab_core::iteminfo::facts_from(&mut m, end, chroms.len() as i64, true)?;
    routes.pon_cromatogrammi(&name, &chroms, &facts);
    let note = if !mzml && sample.is_none() {
        several_note(&name, &ids, k)
    } else {
        String::new()
    };
    Ok(Sample {
        name,
        sample: if mzml { None } else { Some(ids[k]) },
        container,
        listing,
        note,
        routes,
        mzml: m,
        chroms,
    })
}

/// The arguments of a command: its positional words, the options that take a value and the switches.
pub(crate) struct Parsed {
    pub pos: Vec<String>,
    vals: Vec<(String, String)>,
    flags: Vec<String>,
}

impl Parsed {
    pub fn get(&self, k: &str) -> Option<&str> {
        self.vals
            .iter()
            .rev()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
    }
    pub fn num(&self, k: &str) -> std::result::Result<Option<f64>, String> {
        self.get(k)
            .map(|v| {
                v.parse::<f64>()
                    .ok()
                    .filter(|x| x.is_finite())
                    .ok_or(format!("{k} needs a number"))
            })
            .transpose()
    }
    /// `--ms-level N`: 1 when absent.
    pub fn level(&self) -> std::result::Result<u8, String> {
        match self.get("--ms-level") {
            None => Ok(1),
            Some(v) => v
                .parse::<u8>()
                .ok()
                .filter(|l| *l >= 1)
                .ok_or("--ms-level needs 1, 2, ...".into()),
        }
    }
    /// `--rt A[,B]` in minutes.
    pub fn window(&self) -> std::result::Result<Option<(f64, f64)>, String> {
        let Some(v) = self.get("--rt") else {
            return Ok(None);
        };
        let num = |x: &str| {
            x.parse::<f64>()
                .ok()
                .filter(|x| x.is_finite())
                .ok_or("--rt needs minutes: A or A,B".to_string())
        };
        let (a, b) = v.split_once(',').unwrap_or((v, v));
        let (a, b) = (num(a)?, num(b)?);
        Ok(Some((a.min(b), a.max(b))))
    }
    /// `--sample ID`: one id, never for an mzML.
    pub fn sample(&self, file: &Path) -> std::result::Result<Option<u32>, String> {
        self.sample_of("--sample", file)
    }
    /// The sample id given by `key` (`--sample`, `--minus-sample`) for `file`.
    pub fn sample_of(&self, key: &str, file: &Path) -> std::result::Result<Option<u32>, String> {
        let Some(v) = self.get(key) else {
            return Ok(None);
        };
        if is_mzml(file) {
            return Err(format!(
                "{key} is for a file of several samples (.wiff, ...), an mzML holds one"
            ));
        }
        match crate::sample_select::parse(v)
            .map_err(|e| crate::error_text::human_error(&e))?
            .as_slice()
        {
            [one] => Ok(Some(*one)),
            _ => Err(format!("{key} takes one sample id here")),
        }
    }
    pub fn out(&self) -> Option<PathBuf> {
        self.get("--out").map(PathBuf::from)
    }
    /// `--format json|csv`: whether the answer is CSV.
    pub fn csv(&self) -> std::result::Result<bool, String> {
        match self.get("--format") {
            None | Some("json") => Ok(false),
            Some("csv") => Ok(true),
            Some(_) => Err("--format takes json or csv".into()),
        }
    }
}

/// `args` split by the options that take a value (`valued`) and the switches (`flags`); `Ok(None)` for `--help`. `--json` is always accepted.
pub(crate) fn parse(
    args: Vec<String>,
    valued: &[&str],
    flags: &[&str],
) -> std::result::Result<Option<Parsed>, String> {
    let mut p = Parsed {
        pos: Vec::new(),
        vals: Vec::new(),
        flags: Vec::new(),
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--json" => {}
            o if valued.contains(&o) => {
                let v = it.next().ok_or(format!("{o} needs a value"))?;
                p.vals.push((a, v));
            }
            o if flags.contains(&o) => p.flags.push(a),
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ => p.pos.push(a),
        }
    }
    Ok(Some(p))
}

/// The tokens of the array `"key":[...]` of a flat JSON text, exactly as written.
pub(crate) fn tokens<'a>(text: &'a str, key: &str) -> Option<Vec<&'a str>> {
    let at = text.find(&format!("\"{key}\":["))? + key.len() + 4;
    let end = at + text[at..].find(']')?;
    let body = &text[at..end];
    Some(if body.is_empty() {
        Vec::new()
    } else {
        body.split(',').collect()
    })
}

/// The `x`/`y` token pairs of `text` kept inside `window` (the whole when `None`), by the value of `x`.
pub(crate) fn pairs<'a>(
    x: &[&'a str],
    y: &[&'a str],
    window: Option<(f64, f64)>,
) -> Vec<(&'a str, &'a str)> {
    x.iter()
        .zip(y)
        .filter(|(a, _)| {
            window.is_none_or(|(lo, hi)| a.parse::<f64>().is_ok_and(|v| v >= lo && v <= hi))
        })
        .map(|(a, b)| (*a, *b))
        .collect()
}

/// `{"schema":..,<head>,"points":[[x,y],...],<tail>}`: the head and the tail are JSON members already written (no leading comma).
pub(crate) fn points_doc(schema: &str, head: &str, pts: &[(&str, &str)], tail: &str) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":").str(schema).raw(",").raw(head);
    o.raw(",\"n_points\":").int(pts.len() as i64);
    o.raw(",\"points\":[");
    for (i, (a, b)) in pts.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("[")
            .raw(a)
            .raw(",")
            .raw(b)
            .raw("]");
    }
    o.raw("]");
    if !tail.is_empty() {
        o.raw(",").raw(tail);
    }
    o.raw("}");
    o.0
}

/// The 2-column CSV of `pts` under the header `hx,hy`.
pub(crate) fn points_csv(hx: &str, hy: &str, pts: &[(&str, &str)]) -> String {
    let mut s = format!("{hx},{hy}\n");
    for (a, b) in pts {
        s += &format!("{a},{b}\n");
    }
    s
}

/// A JSON string literal.
pub(crate) fn jstr(s: &str) -> String {
    let mut o = Out::default();
    o.str(s);
    o.0
}

/// A number as the engine writes one.
pub(crate) fn jnum(v: f64) -> String {
    let mut o = Out::default();
    o.num(v);
    o.0
}

#[cfg(test)]
#[path = "extract_tests.rs"]
pub(crate) mod tests;
