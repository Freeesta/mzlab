//! `mzlab-cli file info FILE...`: everything an agent asks first about a file, in one call (`file_info/1`): the container, its samples, the items summary
//! of `file items`, the channels of `file channels` and the experiment kind proposed for them. Each part is the very document its own command prints.

use std::path::{Path, PathBuf};

use mzlab_core::error::Result;
use mzlab_core::json::Out;
use mzlab_core::lr::Sample as LrSample;

use crate::extract::{self, Parsed};
use crate::Outcome;

pub const HELP: &str = "mzlab-cli file info: what a file holds, in one call, as JSON (file_info/1): container, samples, the items summary, the channels and the proposed experiment kind.

usage: mzlab-cli file info FILE... [--sample LIST] [--out PATH]

  FILE        an mzML, or any single file the app reads (.raw, mzXML, .wiff, ...); several files give one entry each, in order
  --sample LIST the samples of a file with several (.wiff) to read in full, by their ids (1,3,5 or 2-4); default the first. Every sample is listed with
              its id, name and scans; only the ones read carry `items` (items/1), `channels` (channels/1) and `kind`; an id the file does not have: exit 5
  --out PATH  write there instead of the standard output";

fn size_of(p: &Path) -> String {
    std::fs::metadata(p)
        .ok()
        .filter(|m| m.is_file())
        .map_or("null".into(), |m| m.len().to_string())
}

/// The `files[]` member of one file.
fn file_doc(file: &Path, want: &Option<Vec<u32>>) -> Result<(String, String)> {
    let mzml = extract::is_mzml(file);
    let ids: Vec<Option<u32>> = match want {
        Some(w) if !mzml => crate::sample_select::resolve(file, w)?
            .into_iter()
            .map(Some)
            .collect(),
        _ => vec![None],
    };
    let (mut read, mut notes, mut head) = (Vec::new(), String::new(), None);
    for id in ids {
        let doc = extract::with_tmp("info", |tmp| {
            let mut s = extract::open_sample(file, id, tmp)?;
            let items = mzlab_core::lr::open_items(
                &mut s.routes.idx,
                &mut s.mzml,
                &s.name,
                &LrSample::default(),
            )?;
            let kind = s.routes.canali().experiment.clone();
            let doc = format!(
                "\"items\":{},\"channels\":{},\"kind\":{{\"proposed\":{},\"reason\":{}}}",
                crate::items_cmd::document(&items),
                s.routes.canali().json(),
                extract::jstr(kind.proposed),
                extract::jstr(kind.reason),
            );
            Ok((
                s.sample,
                s.name.clone(),
                s.container.clone(),
                s.listing.clone(),
                s.note.clone(),
                doc,
            ))
        })?;
        notes += &doc.4;
        head.get_or_insert((doc.1, doc.2, doc.3));
        read.push((doc.0, doc.5));
    }
    let (name, container, listing) = head.unwrap_or_default();
    let mut o = Out::default();
    o.raw("{\"file\":")
        .str(&name)
        .raw(",\"container\":")
        .str(&container);
    o.raw(",\"size_bytes\":")
        .raw(&size_of(file))
        .raw(",\"samples\":[");
    if listing.is_empty() {
        // an mzML holds one sample
        o.raw(&format!(
            "{{\"id\":null,\"name\":{},{}}}",
            extract::jstr(&name),
            read[0].1
        ));
    }
    for (i, (id, sname, scans)) in listing.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        o.raw(&format!(
            "{{\"id\":{id},\"name\":{},\"scans\":{scans}",
            extract::jstr(sname)
        ));
        // the sample read when none was asked for is the first one of the file
        let detail = read.iter().find(|(r, _)| *r == Some(*id));
        if let Some((_, d)) = detail {
            o.raw(",").raw(d);
        }
        o.raw("}");
    }
    o.raw("]}");
    Ok((o.0, notes))
}

type Args = (Parsed, Vec<PathBuf>, Option<Vec<u32>>);

fn args_of(args: Vec<String>) -> std::result::Result<Option<Args>, String> {
    let Some(p) = extract::parse(args, &["--sample", "--out"], &[])? else {
        return Ok(None);
    };
    let files: Vec<PathBuf> = p.pos.iter().map(PathBuf::from).collect();
    if files.is_empty() {
        return Err("file info needs a file".into());
    }
    let want = p
        .get("--sample")
        .map(|v| crate::sample_select::parse(v).map_err(|e| crate::error_text::human_error(&e)))
        .transpose()?;
    if want.is_some() && files.iter().any(|f| extract::is_mzml(f)) {
        return Err(
            "--sample is for a file of several samples (.wiff, ...), an mzML holds one".into(),
        );
    }
    Ok(Some((p, files, want)))
}

pub fn run(args: Vec<String>) -> Outcome {
    let (p, files, want) = match args_of(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let r: Result<(Vec<String>, String)> =
        files
            .iter()
            .try_fold((Vec::new(), String::new()), |(mut docs, mut notes), f| {
                let (d, n) = file_doc(f, &want)?;
                docs.push(d);
                notes += &n;
                Ok((docs, notes))
            });
    match r {
        Err(e) => crate::sample_select::fail(&e),
        Ok((docs, notes)) => {
            let text = format!(
                "{{\"schema\":\"file_info/1\",\"files\":[{}]}}\n",
                docs.join(",")
            );
            let mut o = crate::emit(text, &p.out());
            if o.code == 0 {
                o.stderr = notes;
            }
            o
        }
    }
}

#[cfg(test)]
#[path = "file_info_tests.rs"]
mod tests;
