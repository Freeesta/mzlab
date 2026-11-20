//! `mzlab-cli file channels FILE [--out PATH]`: the channels (`channels/1`) of a file, the JSON of `api/channels`. An mzML also gives its SRM and SIC chromatograms;
//! any other file the app reads gives the channels of one sample (`--sample ID`, else the first): its scans and its chromatograms (the mzML shell the site
//! reads), what the site and the app list for that sample. The traces themselves are `trace chrom`.

use std::path::PathBuf;

use mzlab_core::error::Result;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli file channels: the channels of a file (TIC, BPC, one per scan type or precursor, one per SRM transition, UV, analog) as JSON (channels/1).

usage: mzlab-cli file channels FILE [--sample LIST] [--out PATH]

  FILE        an mzML, or any single file the app reads (.raw, mzXML, ...)
  --sample LIST the samples of a file with several (.wiff), by their ids: one id, or a list and ranges (1,3,5 or 2-4: one channels/1 line each, in the order
              of the file, only those samples are read); default the first, the others are named on the standard error; an id the file does not have: exit 5
  --out PATH  write there instead of the standard output
the trace of a channel is `mzlab-cli trace chrom FILE --id ID`";

/// What was asked: the file, its sample and where to write.
struct Request {
    file: PathBuf,
    sample: Option<Vec<u32>>,
    out: Option<PathBuf>,
}

fn parse(args: Vec<String>) -> std::result::Result<Option<Request>, String> {
    let (mut file, mut out, mut sample) = (None, None, None);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            "--sample" => {
                let v = it.next().ok_or("--sample needs a sample id or list")?;
                sample = Some(
                    crate::sample_select::parse(&v)
                        .map_err(|e| crate::error_text::human_error(&e))?,
                )
            }
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ if file.is_none() => file = Some(PathBuf::from(a)),
            _ => return Err("channels takes one file".into()),
        }
    }
    if sample.is_some() && file.as_ref().is_some_and(|f| crate::extract::is_mzml(f)) {
        return Err(
            "--sample is for a file of several samples (.wiff, ...), an mzML holds one".into(),
        );
    }
    Ok(Some(Request {
        file: file.ok_or("channels needs a file")?,
        sample,
        out,
    }))
}

/// The JSON of `api/channels` for `file`, indexes kept in `tmp`. The note names the other samples when `--sample` was not given.
fn read(c: &Request, sample: Option<u32>, tmp: &std::path::Path) -> Result<(String, String)> {
    let mut s = crate::extract::open_sample(&c.file, sample, tmp)?;
    Ok((s.routes.canali().json(), s.note))
}

pub fn run(args: Vec<String>) -> Outcome {
    let c = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let io = crate::extract::io;
    let r = crate::extract::with_tmp("canali", |tmp| {
        let mzml = crate::extract::is_mzml(&c.file);
        // the whole list is checked before the first sample is read: no partial answer for a list with a wrong id
        let ids = match &c.sample {
            Some(w) if !mzml => crate::sample_select::resolve(&c.file, w)?
                .into_iter()
                .map(Some)
                .collect(),
            _ => vec![None],
        };
        let (mut text, mut notes) = (Vec::new(), String::new());
        for (n, id) in ids.into_iter().enumerate() {
            let sub = tmp.join(n.to_string());
            std::fs::create_dir_all(&sub).map_err(io)?;
            let (t, note) = read(&c, id, &sub)?;
            text.push(t);
            notes += &note;
        }
        Ok((text.join("\n"), notes))
    });
    match r {
        Err(e) => crate::sample_select::fail(&e),
        Ok((t, note)) => {
            let mut o = crate::emit(t + "\n", &c.out);
            if o.code == 0 {
                o.stderr = note;
            }
            o
        }
    }
}

#[cfg(test)]
#[path = "channels_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "channels_schema_tests.rs"]
mod schema_tests;
