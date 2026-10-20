//! `mzlab-cli features FILE... [--out PATH] [--preset auto|pos|neg | --params FILE.json]`: the feature engine of the app on the command line.
//! A thin wrapper: the index of each file is built as the desktop app builds it, the recipe comes from `run::recipe_for` (the one behind the panel
//! "Pipeline"), the steps are `run::detect` and `run::align`, and the CSV is the export "full" of the table (`core::table_file`), so the numbers are the ones of the page.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::Index;
use mzlab_core::json::Out;
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::mzidx::{build_mzml_spill, DEFAULT_BUDGET};
use mzlab_core::upload::{impronte, indicizza_lettore, prepare};
use mzlab_feat_table_wasm::core::{samples_json, table_file};
use mzlab_feat_wasm::run;

pub mod flow;
mod flow_formula;
mod flow_match;

const HELP: &str = "mzlab-cli features: the feature table of one or more LC-MS files as CSV, from the engine of the mzLab app.

usage: mzlab-cli features FILE... [--out PATH] [--preset auto|pos|neg | --params FILE.json]

  FILE...            mzML, or any single file the app reads (.raw, mzXML, .cdf, ...); several files form one series, in the order given
  --out PATH         a .csv file, or a folder (then features.csv is written in it); default: standard output
  --preset NAME      auto (default: both polarities as the panel does), pos or neg
  --params FILE.json the numbers of the panel Pipeline: {\"polarity\", \"mode\", \"floor\", \"min_height\", \"min_points\", \"ppm\", \"rt_tol\", \"fill\"}
  --help             this text";

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// The `.mzidx` of `path` written to `idx`: an mzML through its scan table, any other file through the reader of the registry.
fn build_index(path: &Path, idx: &Path) -> Result<()> {
    let name = path
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let size = std::fs::metadata(path).map_err(io)?.len();
    let mut out = File::create(idx).map_err(io)?;
    if name.to_lowercase().ends_with(".mzml") {
        let p = prepare(&mut File::open(path).map_err(io)?, size, &name)?;
        build_mzml_spill(
            &p.table,
            BufReader::new(File::open(path).map_err(io)?),
            &mut out,
            DEFAULT_BUDGET,
            None,
        )?;
    } else {
        let mut fs = FileSet {
            membri: vec![Membro {
                nome: name.clone(),
                dim: size,
                dati: Box::new(File::open(path).map_err(io)?),
                sha1: None,
            }],
        };
        let imp = impronte(&mut fs)?;
        let mut l = mzlab_lettori::apri(fs, &Limiti::desktop())?;
        indicizza_lettore(
            &mut *l,
            0,
            &imp,
            &name,
            &mut out,
            DEFAULT_BUDGET,
            &mut |_| {},
        )?;
    }
    Ok(())
}

fn open(idx: &Path) -> Result<Index<File>> {
    let f = File::open(idx).map_err(io)?;
    let size = f.metadata().map_err(io)?.len();
    Index::open(f, size)
}

/// The CSV of the series `files`, indexes kept in `tmp` (the caller owns it: WASI has no temporary folder).
pub fn features(files: &[PathBuf], params: &str, tmp: &Path) -> Result<String> {
    let idxs: Vec<PathBuf> = (0..files.len())
        .map(|k| tmp.join(format!("{k}.mzidx")))
        .collect();
    for (f, i) in files.iter().zip(&idxs) {
        eprintln!("index {}", f.display());
        build_index(f, i)?;
    }
    let recipe = run::recipe_for(params, &mut open(&idxs[0])?)?;
    let mut detected = Vec::new();
    for (f, i) in files.iter().zip(&idxs) {
        eprintln!("detect {}", f.display());
        let name = f
            .file_name()
            .map_or(String::new(), |n| n.to_string_lossy().into_owned());
        detected.push(run::detect(&mut open(i)?, &name, &recipe)?);
    }
    let (tables, origin) = run::read_files(&detected)?;
    eprintln!("align {} file(s)", files.len());
    let table = run::align(&tables, &origin, &recipe, |k| open(&idxs[k]))?;
    let mut list = Out::default();
    list.raw("{\"files\":[");
    for (k, (name, sha)) in origin.inputs.iter().enumerate() {
        list.raw(if k > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(name)
            .raw(",\"sha256\":")
            .str(sha)
            .raw("}");
    }
    list.raw("]}");
    let samples = samples_json(&list.0)?;
    let polarity = recipe.detect.polarity.map_or(1, i32::from);
    table_file(
        &mut mzlab_feat::store::Slice(&table),
        table.len() as u64,
        &samples,
        "",
        &sha256_hex(&table),
        polarity,
        "full",
    )
}

struct Args {
    files: Vec<PathBuf>,
    out: Option<PathBuf>,
    params: String,
}

fn parse(mut a: impl Iterator<Item = String>) -> std::result::Result<Option<Args>, String> {
    let mut r = Args {
        files: Vec::new(),
        out: None,
        params: String::new(),
    };
    let mut set = false;
    while let Some(x) = a.next() {
        let mut value = |what: &str| a.next().ok_or(format!("{x} needs {what}"));
        match x.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => r.out = Some(value("a path")?.into()),
            "--preset" | "--params" if set => {
                return Err("use --preset or --params, not both".into())
            }
            "--preset" => {
                r.params = match value("auto, pos or neg")?.as_str() {
                    "auto" => String::new(),
                    "pos" => "{\"polarity\":1}".into(),
                    "neg" => "{\"polarity\":-1}".into(),
                    p => return Err(format!("unknown preset {p}")),
                };
                set = true;
            }
            "--params" => {
                let p = value("a JSON file")?;
                r.params = std::fs::read_to_string(&p).map_err(|e| format!("{p}: {e}"))?;
                set = true;
            }
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            _ => r.files.push(x.into()),
        }
    }
    if r.files.is_empty() {
        return Err("no input file".into());
    }
    Ok(Some(r))
}

/// What a run of the command leaves: exit code, standard output, standard error.
pub struct Outcome {
    pub code: u8,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Outcome {
    fn ok(stdout: Vec<u8>) -> Self {
        Outcome {
            code: 0,
            stdout,
            stderr: String::new(),
        }
    }

    fn fail(msg: &str) -> Self {
        Outcome {
            code: 2,
            stdout: Vec::new(),
            stderr: format!("error: {msg}\n(mzlab-cli features --help)\n"),
        }
    }
}

/// `mzlab-cli <args>`: `args` without the program name.
pub fn execute(args: &[String]) -> Outcome {
    let mut args = args.iter().cloned();
    match args.next().as_deref() {
        Some("features") => {}
        Some("--help" | "-h") | None => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Some(c) => return Outcome::fail(&format!("unknown command {c}")),
    }
    let a = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp)
        .map_err(io)
        .and_then(|_| features(&a.files, &a.params, &tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    let csv = match r {
        Ok(c) => c,
        Err(e) => return Outcome::fail(&e.to_string()),
    };
    match &a.out {
        None => Outcome::ok(csv.into_bytes()),
        Some(p) => {
            let to = if p.is_dir() {
                p.join("features.csv")
            } else {
                p.clone()
            };
            std::fs::write(to, csv).map_or_else(
                |e| Outcome::fail(&e.to_string()),
                |_| Outcome::ok(Vec::new()),
            )
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod flow_ident_tests;
