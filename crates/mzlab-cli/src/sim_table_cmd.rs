//! `mzlab-cli sim-table FILE... [--set output=OUTPUT] [--set layout=samples|channels] [--out PATH]`: the SIM table of the Data view (`mzlab_core::simtab`):
//! the peaks of every SIM channel of every sample, or a pivot (one row per sample and one column per channel, or the reverse). Each file gives the `/api/mrm` transitions the engine gives the page (an mzML read as it is, a vendor file through the
//! chromatograms of `lettori_mzml::guscio`, as the site reads it), so the CSV is the bytes of the page's export. A `.wiff` gives one column per sample,
//! named as the site names its mzML (`conversione.js::wiffName`).

use std::fs::File;
use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::fileroutes::mrm_transitions;
use mzlab_core::json::Out;
use mzlab_core::lettori::Limiti;
use mzlab_core::mzmlhead::Mzml;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli sim-table: every SIM channel of the files integrated with the default settings, as CSV.

usage: mzlab-cli sim-table FILE... [--set output=OUTPUT] [--set layout=samples|channels] [--out PATH]

  FILE        an mzML, or a vendor file the app reads (a .wiff with its .wiff.scan gives one sample per sample of the file), in injection order
  --set output=area_sum    sum of the peak areas of each channel in each sample (default; 0.0 = no peak)
         output=area_max   area of the largest peak; height_max its height; apex_rt its apex time (min); n_peaks the number of peaks
         output=peaks      the peak table: one row per resolved peak (sample, q1, name, peak, apex_rt, area, height, snr, fwhm_s)
  --set layout=samples     one row per sample, one column per channel (its Q1 m/z; transition=N without one) (default); layout=channels the reverse
  --out PATH  write there instead of the standard output

Peaks: every resolved peak of the whole trace, up to 10 per channel (smoothing 1-2-1 over 3 points, at least 3 points across, S/N at least 3);
an empty cell = the sample has no such channel, or no peak for area_max, height_max and apex_rt.";

struct Chiesta {
    files: Vec<PathBuf>,
    output: String,
    layout: String,
    out: Option<PathBuf>,
}

fn parse(args: Vec<String>) -> std::result::Result<Option<Chiesta>, String> {
    let (mut files, mut output, mut layout, mut out) = (
        Vec::new(),
        "area_sum".to_string(),
        "samples".to_string(),
        None,
    );
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            "--set" => {
                let v = it.next().ok_or("--set needs NAME=VALUE")?;
                match v.split_once('=') {
                    Some(("output", o)) if mzlab_core::simtab::OUTPUTS.contains(&o) => {
                        output = o.to_string()
                    }
                    Some(("output", o)) => {
                        return Err(format!(
                            "output is one of {}, not {o}",
                            mzlab_core::simtab::OUTPUTS.join(", ")
                        ))
                    }
                    Some(("layout", l @ ("samples" | "channels"))) => layout = l.to_string(),
                    Some(("layout", l)) => {
                        return Err(format!("layout is samples or channels, not {l}"))
                    }
                    _ => return Err(format!("unknown setting {v}")),
                }
            }
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ => files.push(PathBuf::from(a)),
        }
    }
    if files.is_empty() {
        return Err("sim-table needs at least one file".into());
    }
    Ok(Some(Chiesta {
        files,
        output,
        layout,
        out,
    }))
}

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// The name of the mzML the site makes for sample `id` (named `given` by the reader) of the `.wiff` `stem`: `conversione.js::wiffName` with `freeMzml`.
fn wiff_name(stem: &str, id: u32, given: &str, taken: &[String]) -> String {
    let named = given != format!("{stem}.wiff #{id}");
    let clean: String = if named {
        given
            .trim()
            .chars()
            .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
            .take(120)
            .collect()
    } else {
        String::new()
    };
    let base = if clean.is_empty() {
        stem.to_string()
    } else {
        clean
    };
    let free = |n: &str| !taken.iter().any(|t| t.eq_ignore_ascii_case(n));
    if free(&format!("{base}.mzML")) {
        return format!("{base}.mzML");
    }
    (1..)
        .map(|i| {
            if i == 1 {
                format!("{base}.wiff.mzML")
            } else {
                format!("{base}_{i}.wiff.mzML")
            }
        })
        .find(|n| free(n))
        .unwrap_or_default()
}

/// (name, `/api/mrm` transitions) of every sample of the file `p`; a sample whose records lie past the end of its file (a copy cut short) adds a line
/// to `warn`, the table keeps what was read before the cut.
fn samples_of(
    p: &Path,
    taken: &mut Vec<String>,
    warn: &mut String,
) -> Result<Vec<(String, String)>> {
    let name = p
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let lower = name.to_lowercase();
    if lower.ends_with(".mzml") {
        let f = File::open(p).map_err(io)?;
        let size = f.metadata().map_err(io)?.len();
        let mut m = Mzml::new(f, size);
        let chroms = m.chromatograms(0)?;
        taken.push(name.clone());
        return Ok(vec![(name, mrm_transitions(&mut m, &chroms)?)]);
    }
    let mut l = mzlab_lettori::apri(crate::flow_set::file_set(p)?, &Limiti::desktop())?;
    let stem = name
        .rsplit_once('.')
        .map_or(name.as_str(), |s| s.0)
        .to_string();
    let wiff = lower.ends_with(".wiff");
    let list: Vec<(u32, String)> = l
        .campioni()
        .iter()
        .map(|c| (c.id, c.nome.clone()))
        .collect();
    let mut out = Vec::new();
    for (c, (id, given)) in list.iter().enumerate() {
        let shell = mzlab_core::lettori_mzml::guscio(l.as_mut(), c)?;
        let size = shell.len() as u64;
        let mut m = Mzml::new(shell, size);
        let chroms = m.chromatograms(0)?;
        let n = if wiff {
            wiff_name(&stem, *id, given, taken)
        } else {
            format!("{stem}.mzML")
        };
        let missing = l.mancanti(c);
        if missing > 0 {
            *warn += &format!("warning: {n}: {missing} records past the end of the file (cut short?): only the points before the cut are integrated\n");
        }
        taken.push(n.clone());
        out.push((n, mrm_transitions(&mut m, &chroms)?));
    }
    Ok(out)
}

fn table(c: &Chiesta, warn: &mut String) -> Result<String> {
    let mut taken = Vec::new();
    let mut o = Out::default();
    o.raw("[");
    let mut first = true;
    for p in &c.files {
        for (name, tr) in samples_of(p, &mut taken, warn)? {
            o.raw(if first { "{\"name\":" } else { ",{\"name\":" })
                .str(&name)
                .raw(",\"transitions\":")
                .raw(&tr)
                .raw("}");
            first = false;
        }
    }
    o.raw("]");
    mzlab_core::simtab::table_text(&o.0, &c.output, &c.layout)
}

pub fn run(args: Vec<String>) -> Outcome {
    let c = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let mut warn = String::new();
    match table(&c, &mut warn) {
        Err(e) => Outcome::fail(&format!("{warn}{} {:?}", e.key, e.params)),
        Ok(t) => {
            let mut o = crate::emit(t, &c.out);
            o.stderr.insert_str(0, &warn);
            o
        }
    }
}

#[cfg(test)]
#[path = "sim_table_cmd_tests.rs"]
mod tests;
