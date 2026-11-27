//! `mzlab-cli table row`: everything the `.mzfeat` knows about one row (`row/2`) and, with `--traces`, its XIC, MS1 and MS2 read from the `.mzidx` of the series
//! (`row_traces/2`). The bytes are those of the browser (`table_row`, `table_row_traces`) and of the app (`feat/table` `row`, `row_traces`): one engine, `mzlab_ops::run::table::core`.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::Index;
use mzlab_feat::store::{origin_from_meta, read_table_from};
use mzlab_ops::run::table::core;

use crate::extract;
use crate::Outcome;

pub const HELP: &str = "mzlab-cli table row: everything one row of a feature table knows, for the page of one feature (row/2).

usage: mzlab-cli table row TABLE.mzfeat --id N [--samples FILE.json] [--traces --index DIR [--ppm P]] [--json] [--out PATH]

  TABLE.mzfeat  a feature table (`features`, `flow run`, `quant`): the `id` of the first column of its CSV is --id
  --id N        the row; an id the table does not have is exit 5
  --samples F   a samples/1 document (`sample table`): types and times of the files; without it they are proposed from the file names (noted on stderr)
  --traces      also row_traces/2, from the `.mzidx` of every file: the XIC of each file, the MS1 at the apex of the file with the largest area and the MS2 of the row
  --index DIR   with --traces: the folder with the `.mzidx` of each file, named after it (`a.mzML` -> `a.mzidx`; build them with `file index`)
  --ppm P       with --traces: the XIC window around the m/z of the row (default 5)
  --json        accepted: the answer is always JSON
  --out PATH    write there instead of the standard output

stdout is row/2, then with --traces row_traces/2 on the next line (one document per line, each with its own bytes, the same as the site). A summary that was not computed is null.
Exit codes: 0 ok; 2 usage (no --id, --traces without --index); 4 table, samples or an index missing or unreadable; 5 the id is not in the table; 6 --out cannot be written.";

struct Job {
    table: std::path::PathBuf,
    id: u32,
    samples: Option<std::path::PathBuf>,
    index: Option<std::path::PathBuf>,
    ppm: f64,
    out: Option<std::path::PathBuf>,
}

fn job(args: Vec<String>) -> std::result::Result<std::result::Result<Job, &'static str>, String> {
    let valued = ["--id", "--samples", "--index", "--ppm", "--out"];
    let Some(p) = extract::parse(args, &valued, &["--traces"])? else {
        return Ok(Err(HELP));
    };
    let table = match p.pos.as_slice() {
        [t] => std::path::PathBuf::from(t),
        [] => return Err("table row needs a table".into()),
        _ => return Err("table row takes one table".into()),
    };
    let id = p
        .get("--id")
        .ok_or("table row needs --id")?
        .parse::<u32>()
        .map_err(|_| "--id needs a row id (a whole number)")?;
    let ppm = p.num("--ppm")?.unwrap_or(5.0);
    let traces = p.flag("--traces");
    if traces && p.get("--index").is_none() {
        return Err("--traces needs --index DIR".into());
    }
    if !traces && (p.get("--index").is_some() || p.get("--ppm").is_some()) {
        return Err("--index and --ppm are for --traces".into());
    }
    Ok(Ok(Job {
        table,
        id,
        samples: p.get("--samples").map(Into::into),
        index: traces.then(|| p.get("--index").map(Into::into)).flatten(),
        ppm,
        out: p.out(),
    }))
}

pub(super) fn read(path: &std::path::Path) -> Result<Vec<u8>> {
    std::fs::read(path)
        .map_err(|e| Error::new("err.cli.file").with("path", format!("{}: {e}", path.display())))
}

/// The `samples/1` text and whether it was proposed: the given file, else `propose` over the names and SHA-256 of the files the table records.
pub(super) fn samples_of(given: Option<&std::path::Path>, table: &[u8]) -> Result<(String, bool)> {
    if let Some(p) = given {
        let text = String::from_utf8(read(p)?)
            .map_err(|_| Error::new("err.cli.file").with("path", p.display().to_string()))?;
        return Ok((text, false));
    }
    let (_, meta) = read_table_from(&mut table.to_vec(), table.len() as u64)?;
    let files = origin_from_meta(&meta)
        .inputs
        .iter()
        .map(|(n, s)| {
            format!(
                "{{\"name\":{},\"sha256\":{}}}",
                mzlab_core::jsonb::write(&mzlab_core::jsonb::text(n)),
                mzlab_core::jsonb::write(&mzlab_core::jsonb::text(s))
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    Ok((
        core::samples_json(&format!("{{\"files\":[{files}]}}"))?,
        true,
    ))
}

/// The `.mzidx` of the file `name`: `STEM.mzidx` (the extension of the file replaced), else `NAME.mzidx`, in `dir`.
fn index_path(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let base = std::path::Path::new(name)
        .file_name()
        .map_or(name.to_string(), |n| n.to_string_lossy().into_owned());
    let stem = base.rsplit_once('.').map_or(base.as_str(), |(s, _)| s);
    let first = dir.join(format!("{stem}.mzidx"));
    if first.exists() {
        first
    } else {
        dir.join(format!("{base}.mzidx"))
    }
}

fn answer(j: &Job) -> Result<(String, bool)> {
    let table = read(&j.table)?;
    let size = table.len() as u64;
    let (samples, proposed) = samples_of(j.samples.as_deref(), &table)?;
    let mut text = core::row_json(&mut table.clone(), size, &samples, j.id)?;
    if let Some(dir) = &j.index {
        let mut names = mzlab_feat::store::table_names(&mut table.clone(), size)?;
        if names.is_empty() {
            names = mzlab_feat::samples::from_json(samples.as_bytes())?
                .into_iter()
                .map(|s| s.file)
                .collect();
        }
        let open = |k: usize| {
            let name = names.get(k).map_or("", String::as_str);
            let p = index_path(dir, name);
            let f = std::fs::File::open(&p).map_err(|e| {
                Error::new("err.cli.file").with("path", format!("{}: {e}", p.display()))
            })?;
            let n = f.metadata().map_or(0, |m| m.len());
            Index::open(f, n)
        };
        let traces = core::row_traces_json(&mut table.clone(), size, &samples, j.id, j.ppm, open)?;
        text = format!("{text}\n{traces}");
    }
    Ok((text + "\n", proposed))
}

pub fn run(args: Vec<String>) -> Outcome {
    let j = match job(args) {
        Ok(Ok(j)) => j,
        Ok(Err(h)) => return Outcome::ok(format!("{h}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    match answer(&j) {
        Err(e) => Outcome::fail_error(&e),
        Ok((t, proposed)) => {
            let mut o = crate::emit(t, &j.out);
            if proposed && o.code == 0 {
                o.stderr += "note.table.row.samples: no --samples, the types and times come from the file names\n";
            }
            o
        }
    }
}

#[cfg(test)]
#[path = "table_row_cmd_tests.rs"]
mod tests;
