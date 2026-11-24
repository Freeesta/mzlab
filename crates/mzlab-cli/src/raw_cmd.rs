//! `mzlab-cli raw info FILE...` (`raw-info/1`, `docs/studies/RAW_ACCESS_STUDY_2.md` §1): what kind of data a file holds and whether its ions can be
//! counted, from its index (`mzlab_core::rawinfo`, the `/api/rawinfo` route the app and the site answer). A file that cannot be read is an entry
//! with its `error/1` (exit 1, the others answered); `--dry-run` is `raw-plan/1`. The `provenance` block and the warnings are shared with `raw cache`.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

use crate::extract::{self, jstr, Parsed};
use crate::Outcome;

pub const INFO_HELP: &str = "mzlab-cli raw info: what kind of data a file holds and whether its ions can be counted, as JSON (raw-info/1), from its index:
per sample polarity, RT, scans per MS level, scan types, mass range, and on probed scans the m/z grid, the ion quantum and the spectrum-type label
against the data (warn.raw.labelMismatch when they differ); `provenance` names the program, files, parameters and automatic choices.

usage: mzlab-cli raw info FILE... [--sample ID] [--probe N] [--offset-ion MZ[@RT]] [--dry-run] [--out PATH]

  FILE        an mzML, or any file the app reads (raw scans of any format, not only Thermo .raw); several files give one entry each, in order
  --sample ID one sample of a file with several (.wiff); default the first; an id the file does not have: exit 5
  --probe N   scans looked at for the grid, the quantum and the spectrum type, equally spaced over the lowest MS level (default 64, at most 4096)
  --offset-ion MZ[@RT] measure the m/z offset on this known ion (near RT min): `mass_offset` carries the evidence; default not measured
  --dry-run   the resolved query and each file with its cache status (raw-plan/1); reads no peak
  --out PATH  write there instead of the standard output
exit 0 answered, 1 a file could not be read (its entry has `error`, the others are answered), 2 usage, 4 a file is missing, 5 unknown sample";

/// The parsed question of `raw info`.
pub(crate) struct Ask {
    pub files: Vec<PathBuf>,
    pub probe: usize,
    pub offset: Option<(f64, Option<f64>)>,
}

/// `MZ[@RT]` of `--offset-ion`.
fn offset_of(v: &str) -> std::result::Result<(f64, Option<f64>), String> {
    let bad = || format!("--offset-ion needs MZ or MZ@RT (an m/z above 0, minutes): {v}");
    let (m, t) = v.split_once('@').map_or((v, None), |(m, t)| (m, Some(t)));
    let mz = m
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite() && *x > 0.0)
        .ok_or_else(bad)?;
    let rt = t
        .map(|t| {
            t.parse::<f64>()
                .ok()
                .filter(|x| x.is_finite())
                .ok_or_else(bad)
        })
        .transpose()?;
    Ok((mz, rt))
}

fn ask_of(p: &Parsed) -> std::result::Result<Ask, String> {
    let files: Vec<PathBuf> = p.pos.iter().map(PathBuf::from).collect();
    if files.is_empty() {
        return Err("raw info needs a file".into());
    }
    let max = mzlab_core::rawinfo::PROBE_MAX;
    let probe = match p.get("--probe") {
        None => mzlab_core::rawinfo::PROBE,
        Some(v) => v
            .parse::<usize>()
            .ok()
            .filter(|n| (1..=max).contains(n))
            .ok_or(format!(
                "--probe needs a whole number of scans from 1 to {max}"
            ))?,
    };
    let offset = p.get("--offset-ion").map(offset_of).transpose()?;
    Ok(Ask {
        files,
        probe,
        offset,
    })
}

/// The file name the answers use.
pub(crate) fn name_of(f: &Path) -> String {
    f.file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned())
}

/// The first file of `files` that is not there, as the input error of the whole run (exit 4).
pub(crate) fn missing(files: &[PathBuf]) -> Option<Error> {
    files
        .iter()
        .find(|f| std::fs::metadata(f).is_err())
        .map(|f| Error::new("err.flow.io").with("detail", format!("{}: no such file", f.display())))
}

/// A file entry that could not be answered: `{"file":NAME,"error":<error/1>}`.
pub(crate) fn error_entry(f: &Path, e: &Error, command: &str) -> String {
    let line = crate::contract::error_line(crate::exit::code_of(e), e, "", command);
    format!(
        "{{\"file\":{},\"error\":{}}}",
        jstr(&name_of(f)),
        line.trim_end()
    )
}

/// Whether an error is the whole run's (a usage or selector error: exit 2 or 5), not one file's.
pub(crate) fn whole_run(e: &Error) -> bool {
    matches!(
        crate::exit::code_of(e),
        crate::exit::USAGE | crate::exit::SELECTOR
    )
}

/// The warnings one sample's members call for: a label the data contradict, intensities with no quantum.
pub(crate) fn warnings_of(file: &str, sample: Option<u32>, members: &str, out: &mut Vec<String>) {
    let Ok(v) = parse(format!("{{{members}}}").as_bytes()) else {
        return;
    };
    let at = |a: &str, b: &str| {
        v.get(a)
            .and_then(|x| x.get(b))
            .cloned()
            .unwrap_or(Value::Null)
    };
    let head = |key: &str| {
        let mut o = Out::default();
        o.raw("{\"key\":")
            .str(key)
            .raw(",\"params\":{\"file\":")
            .str(file);
        o.raw(",\"sample\":")
            .raw(&sample.map_or("null".into(), |s| s.to_string()));
        o.0
    };
    if at("spectrum_type", "agree").bool() == Some(false) {
        let mut o = Out::default();
        o.raw(&head("warn.raw.labelMismatch"));
        o.raw(",\"label\":")
            .str(at("spectrum_type", "label").str().unwrap_or(""));
        o.raw(",\"data\":")
            .str(at("spectrum_type", "data").str().unwrap_or(""));
        o.raw(",\"runs_share\":")
            .opt(at("spectrum_type", "runs_share").f64())
            .raw("}}");
        out.push(o.0);
    }
    if at("ion_quantum", "status").str() == Some("continuous") {
        out.push(head("warn.raw.noQuantum") + "}}");
    }
}

/// The `provenance` member of a `raw` answer: program, version, engine, the command as given, the files, the parameters with their defaults
/// resolved, the automatic choices and the citation. No cache status: stdout is the same with and without `--cache` (stderr says it).
pub(crate) fn provenance(
    command: &[String],
    files: &[String],
    params: &str,
    choices: &[String],
) -> String {
    let mut o = Out::default();
    o.raw("\"provenance\":{\"program\":\"mzLab\",\"version\":")
        .str(crate::citation::VERSION);
    o.raw(",\"commit\":");
    match crate::citation::commit() {
        Some(c) => o.str(c),
        None => o.raw("null"),
    };
    o.raw(",\"engine\":{\"index_format\":")
        .int(i64::from(mzlab_core::idx_newer::VERSION_MAX));
    o.raw(",\"quantum\":\"ionstat.quantum\"},\"command\":[");
    for (i, a) in command.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(a);
    }
    o.raw("],\"files\":[")
        .raw(&files.join(","))
        .raw("],\"params\":")
        .raw(params);
    o.raw(",\"choices\":[")
        .raw(&choices.join(","))
        .raw("],\"cite\":")
        .str(&crate::citation::text());
    o.raw("}");
    o.0
}

/// One `provenance.files` member.
pub(crate) fn prov_file(
    name: &str,
    sha256: &str,
    size: Option<u64>,
    sample: Option<u32>,
) -> String {
    let mut o = Out::default();
    o.raw("{\"name\":")
        .str(name)
        .raw(",\"sha256\":")
        .str(sha256);
    o.raw(",\"size\":")
        .raw(&size.map_or("null".into(), |s| s.to_string()));
    o.raw(",\"sample\":")
        .raw(&sample.map_or("null".into(), |s| s.to_string()))
        .raw("}");
    o.0
}

/// One `choices` member: what was chosen, its value (JSON) and why.
pub(crate) fn choice(what: &str, value: &str, why: &str) -> String {
    let mut o = Out::default();
    o.raw("{\"what\":")
        .str(what)
        .raw(",\"value\":")
        .raw(value)
        .raw(",\"why\":")
        .str(why)
        .raw("}");
    o.0
}

/// What one file answered: its entry, the `provenance.files` member, the warnings and the automatic choices.
struct Answer {
    entry: String,
    prov: String,
    warnings: Vec<String>,
    choices: Vec<String>,
}

fn one(f: &Path, sample: Option<u32>, a: &Ask) -> Result<(Answer, String)> {
    extract::with_tmp("raw", |tmp| {
        let mut s = extract::open_sample(f, sample, tmp)?;
        let members = s.routes.raw_info(a.probe, a.offset)?;
        let sha = s
            .routes
            .idx
            .source
            .get("sha256")
            .and_then(|v| v.str())
            .unwrap_or("")
            .to_string();
        let size = std::fs::metadata(f)
            .ok()
            .filter(|m| m.is_file())
            .map(|m| m.len());
        let mut o = Out::default();
        o.raw("{\"file\":")
            .str(&s.name)
            .raw(",\"container\":")
            .str(&s.container);
        o.raw(",\"sha256\":").str(&sha).raw(",\"size_bytes\":");
        o.raw(&size.map_or("null".into(), |n| n.to_string()));
        o.raw(",\"samples\":[{\"id\":")
            .raw(&s.sample.map_or("null".into(), |i| i.to_string()));
        let sname = s
            .listing
            .iter()
            .find(|l| Some(l.0) == s.sample)
            .map_or(s.name.clone(), |l| l.1.clone());
        o.raw(",\"name\":")
            .str(&sname)
            .raw(",")
            .raw(&members)
            .raw("}]}");
        let mut warnings = Vec::new();
        warnings_of(&s.name, s.sample, &members, &mut warnings);
        let mut choices = Vec::new();
        if sample.is_none() && s.sample.is_some() {
            choices.push(choice(
                "sample",
                &s.sample.map_or("null".into(), |i| i.to_string()),
                "the first sample of the file (--sample picks another)",
            ));
        }
        let lv = parse(format!("{{{members}}}").as_bytes())
            .ok()
            .and_then(|v| {
                v.get("probe")
                    .and_then(|p| p.get("ms_level"))
                    .and_then(|l| l.f64())
            });
        if let Some(l) = lv {
            choices.push(choice(
                "probe_level",
                &(l as i64).to_string(),
                "the lowest MS level with scans",
            ));
        }
        let prov = prov_file(&s.name, &sha, size, s.sample);
        Ok((
            Answer {
                entry: o.0,
                prov,
                warnings,
                choices,
            },
            s.note,
        ))
    })
}

/// `raw-plan/1`: the resolved query and each file with its cache status; no peak is read.
fn plan(p: &Parsed, a: &Ask) -> Outcome {
    let dir = crate::idx_cache::dir();
    let mut rows = Vec::new();
    for f in &a.files {
        let label = match p.sample(f) {
            Ok(s) => s.map_or(String::new(), |id| format!("{}@{id}", name_of(f))),
            Err(m) => return Outcome::fail(&m),
        };
        let cache = match &dir {
            None => "none".to_string(),
            Some(d) => match crate::idx_cache::peek(d, f, &label) {
                Ok((st, _, _)) => st.to_string(),
                Err(e) => return crate::sample_select::fail(&e),
            },
        };
        let size = std::fs::metadata(f).ok().map(|m| m.len());
        rows.push(format!(
            "{{\"file\":{},\"size_bytes\":{},\"cache\":\"{cache}\"}}",
            jstr(&name_of(f)),
            size.map_or("null".into(), |n| n.to_string())
        ));
    }
    let mut o = Out::default();
    o.raw("{\"schema\":\"raw-plan/1\",\"command\":\"raw info\",\"query\":{\"probe\":")
        .int(a.probe as i64);
    o.raw(",\"offset_ion\":").raw(&offset_json(a.offset));
    o.raw(",\"sample\":")
        .raw(&p.get("--sample").map_or("null".into(), jstr));
    o.raw("},\"files\":[").raw(&rows.join(",")).raw("]}\n");
    crate::emit(o.0, &p.out())
}

fn offset_json(o: Option<(f64, Option<f64>)>) -> String {
    let mut out = Out::default();
    match o {
        None => out.raw("null"),
        Some((mz, rt)) => out.raw("{\"mz\":").num(mz).raw(",\"rt\":").opt(rt).raw("}"),
    };
    out.0
}

/// `provenance.parameters`: the query with its defaults resolved.
fn params_json(p: &Parsed, a: &Ask) -> String {
    let mut params = Out::default();
    params
        .raw("{\"probe\":")
        .int(a.probe as i64)
        .raw(",\"offset_ion\":")
        .raw(&offset_json(a.offset));
    params
        .raw(",\"sample\":")
        .raw(&p.get("--sample").map_or("null".into(), jstr))
        .raw("}");
    params.0
}

pub fn info_run(args: Vec<String>) -> Outcome {
    let command: Vec<String> = ["mzlab-cli", "raw", "info"]
        .iter()
        .map(|s| s.to_string())
        .chain(args.iter().cloned())
        .collect();
    let p = match extract::parse(
        args,
        &["--sample", "--probe", "--offset-ion", "--out"],
        &["--dry-run"],
    ) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{INFO_HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let a = match ask_of(&p) {
        Ok(a) => a,
        Err(e) => return Outcome::fail(&e),
    };
    if let Some(e) = missing(&a.files) {
        return crate::sample_select::fail(&e);
    }
    if p.flag("--dry-run") {
        return plan(&p, &a);
    }
    let (mut entries, mut provs, mut warnings, mut choices, mut notes, mut bad) = (
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        String::new(),
        false,
    );
    for f in &a.files {
        let sample = match p.sample(f) {
            Ok(s) => s,
            Err(m) => return Outcome::fail(&m),
        };
        match one(f, sample, &a) {
            Ok((ans, note)) => {
                entries.push(ans.entry);
                provs.push(ans.prov);
                warnings.extend(ans.warnings);
                choices.extend(ans.choices);
                notes += &note;
            }
            Err(e) if whole_run(&e) => return crate::sample_select::fail(&e),
            Err(e) => {
                bad = true;
                entries.push(error_entry(f, &e, "raw info"));
            }
        }
    }
    let params = params_json(&p, &a);
    let text = format!(
        "{{\"schema\":\"raw-info/1\",\"files\":[{}],\"warnings\":[{}],{}}}\n",
        entries.join(","),
        warnings.join(","),
        provenance(&command, &provs, &params, &choices)
    );
    let mut o = crate::emit(text, &p.out());
    if o.code == 0 {
        o.stderr = notes;
        if bad {
            o.code = crate::exit::DATA;
        }
    }
    o
}

#[cfg(test)]
#[path = "raw_cmd_tests.rs"]
pub(crate) mod tests;
