//! `mzlab-cli msn-lib add|build|export`: the library of MSn trees `msnlib/1` of the standards (`docs/agenti/msnlib.md`). `add` puts the graph of one infusion
//! into a library file, `build` does it for every row of a manifest, `export` writes the MS2 spectra of the entries as MSP or MassBank. The library is always written by the
//! same function as the route `api/msnlib/aggiungi` (`mzlab_core::msngrafo::aggiungi`).

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::msngrafo::{aggiungi, cerca, Opzioni, OpzioniCerca};

use mzlab_core::msngrafo::{export_text, LibFormato as Formato};

pub const HELP: &str = "mzlab-cli msn-lib: the library of MSn trees (msnlib/1).

usage: mzlab-cli msn-lib add LIB.json FILE.mzML --id ID --formula ION [msn-graph options] [--out PATH]
       mzlab-cli msn-lib build MANIFEST.tsv [--dir DIR] [--out LIB.json]
       mzlab-cli msn-lib export LIB.json [--pulito] [--tutti] [--annota] [--formato msp|massbank] [--out PATH]
       mzlab-cli msn-lib search LIB.json GRAPH.json [--ppm N] [--tol N] [--min-prove N] [--top N] [--delta N] [--out PATH]

  add     adds the entry of the infusion to LIB (created when absent); the same entry twice is left as it is. LIB is rewritten unless --out.
          The options are those of `msn-graph` (--help of that command)
  build   one entry per row of the manifest (tab separated, header): id, formula_neutra, addotto_atteso, file (relative to DIR, by default the folder
          of the manifest); optional smiles, attivazione. The ion is \"<formula_neutra> <addotto_atteso>\". Writes LIB.json next to the manifest unless --out
  export  the MS2 spectra of every entry as MSP text (--tutti: every MS level); --pulito keeps the peaks a node of the graph explains;
          --annota writes the formula and the neutral loss of each explained peak (MSP: quoted note; MassBank: PK$ANNOTATION and a COMMENT);
          --formato massbank writes MassBank records (MS_TYPE MS2, MS3, ... and the path of precursors in the comment)
  search  the entries of LIB whose precursor is within --ppm (10) of the root of GRAPH (the output of `msn-graph`) and of the same polarity, as the JSON
          msncerca/1: three separate scores (MS2 entropy, tree, shared losses) and their proofs, no verdict. --tol: Da window of the peaks (0.01);
          --min-prove: peaks shared in the best MS2 pair (1); --top: candidates kept (10);
          --delta: also entries up to N Da away in precursor (0 = off), as analogues with the fragments shifted by the difference and the ones that are not";

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

fn read_lib(path: &Path) -> Result<String> {
    if path.exists() {
        std::fs::read_to_string(path).map_err(io)
    } else {
        Ok(String::new())
    }
}

/// `add LIB FILE --id ID [msn-graph options]`: returns the notice for stderr.
fn add(args: Vec<String>, tmp: &Path) -> Result<String> {
    let bad = |m: &str| Error::new("err.flow.param.type").with("detail", m);
    let (mut lib, mut id, mut rest) = (None, None, Vec::new());
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--id" => id = it.next(),
            _ if lib.is_none() => lib = Some(PathBuf::from(x)),
            _ => rest.push(x),
        }
    }
    let (lib, id) = (
        lib.ok_or_else(|| bad("a library file is needed"))?,
        id.ok_or_else(|| bad("--id is needed"))?,
    );
    let a = crate::msn_graph::parse_args(rest.into_iter())
        .map_err(|m| bad(&m))?
        .ok_or_else(|| bad("--help is not an option here"))?;
    let (g, json) = crate::msn_graph::grafo(&a.file, &a.o, tmp)?;
    let json = crate::msn_graph::con_chiavi(&a, &g, json)?;
    let (text, doppione) = aggiungi(&read_lib(&lib)?, &id, &g, &json)?;
    let out = a.out.clone().unwrap_or(lib);
    std::fs::write(&out, text).map_err(io)?;
    Ok(format!(
        "{}: {id} {}\n",
        out.display(),
        if doppione { "already there" } else { "added" }
    ))
}

/// The rows of a manifest as (column → value) lists.
fn rows(text: &str) -> Result<Vec<Vec<(String, String)>>> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let head: Vec<&str> = lines
        .next()
        .ok_or_else(|| Error::new("err.msnlib.bad"))?
        .split('\t')
        .collect();
    Ok(lines
        .map(|l| {
            head.iter()
                .zip(l.split('\t'))
                .map(|(h, v)| (h.trim().to_string(), v.trim().to_string()))
                .collect()
        })
        .collect())
}

fn column<'a>(row: &'a [(String, String)], name: &str) -> &'a str {
    row.iter()
        .find(|(k, _)| k == name)
        .map_or("", |(_, v)| v.as_str())
}

fn build(args: Vec<String>, tmp: &Path) -> Result<String> {
    let bad = |m: &str| Error::new("err.flow.param.type").with("detail", m);
    let (mut manifest, mut dir, mut out) = (None, None, None);
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--dir" => dir = it.next().map(PathBuf::from),
            "--out" => out = it.next().map(PathBuf::from),
            _ if manifest.is_none() => manifest = Some(PathBuf::from(x)),
            _ => return Err(bad(&format!("unknown argument {x}"))),
        }
    }
    let manifest = manifest.ok_or_else(|| bad("a manifest is needed"))?;
    let folder = dir.unwrap_or_else(|| {
        manifest
            .parent()
            .map_or(PathBuf::from("."), Path::to_path_buf)
    });
    let out = out.unwrap_or_else(|| manifest.with_extension("json"));
    let mut text = String::new();
    for row in rows(&std::fs::read_to_string(&manifest).map_err(io)?)? {
        let (id, file) = (column(&row, "id"), column(&row, "file"));
        let ion = format!(
            "{} {}",
            column(&row, "formula_neutra"),
            column(&row, "addotto_atteso")
        );
        let none = |s: &str| (!s.is_empty()).then(|| s.to_string());
        let o = Opzioni {
            formula: Some(ion),
            smiles: none(column(&row, "smiles")),
            attivazione: none(column(&row, "attivazione")),
            ..Opzioni::default()
        };
        if id.is_empty() || file.is_empty() {
            return Err(bad("every row needs id and file"));
        }
        let (g, json) = crate::msn_graph::grafo(&folder.join(file), &o, tmp)?;
        text = aggiungi(&text, id, &g, &json)?.0;
    }
    std::fs::write(&out, &text).map_err(io)?;
    Ok(format!("{}: {} bytes\n", out.display(), text.len()))
}

fn export(args: Vec<String>) -> Result<(String, Option<PathBuf>)> {
    let bad = |m: &str| Error::new("err.flow.param.type").with("detail", m);
    let (mut lib, mut out, mut pulito, mut tutti, mut annota) = (None, None, false, false, false);
    let mut formato = Formato::Msp;
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--pulito" => pulito = true,
            "--tutti" => tutti = true,
            "--annota" => annota = true,
            "--formato" => formato = Formato::parse(&it.next().unwrap_or_default())?,
            "--out" => out = it.next().map(PathBuf::from),
            _ if lib.is_none() => lib = Some(PathBuf::from(x)),
            _ => return Err(bad(&format!("unknown argument {x}"))),
        }
    }
    let lib = lib.ok_or_else(|| bad("a library file is needed"))?;
    Ok((
        export_text(
            &std::fs::read_to_string(&lib).map_err(io)?,
            formato,
            (pulito, tutti, annota),
        )?,
        out,
    ))
}

/// `search LIB GRAPH [options]`: the text of `msncerca/1` and the file it goes to, if any.
fn search(args: Vec<String>) -> Result<(String, Option<PathBuf>)> {
    let bad = |m: &str| Error::new("err.flow.param.type").with("detail", m);
    let (mut files, mut out, mut o) = (Vec::new(), None, OpzioniCerca::default());
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        let mut number = || {
            let v = it.next().unwrap_or_default();
            v.parse::<f64>()
                .map_err(|_| bad(&format!("{x} needs a number, not {v}")))
        };
        match x.as_str() {
            "--ppm" => o.ppm_prec = number()?,
            "--tol" => o.tol_mz = number()?,
            "--min-prove" => o.min_prove = number()? as usize,
            "--top" => o.top = number()? as usize,
            "--delta" => o.delta_max = number()?,
            "--out" => out = it.next().map(PathBuf::from),
            _ if !x.starts_with("--") => files.push(PathBuf::from(x)),
            _ => return Err(bad(&format!("unknown argument {x}"))),
        }
    }
    let [lib, graph] = files.as_slice() else {
        return Err(bad("a library file and a graph file are needed"));
    };
    let read = |p: &Path| std::fs::read_to_string(p).map_err(io);
    Ok((cerca(&read(lib)?, &read(graph)?, &o)?, out))
}

/// The text to a file or to stdout.
fn deliver(t: String, out: Option<PathBuf>) -> Result<(Vec<u8>, String)> {
    match out {
        Some(p) => {
            std::fs::write(&p, t).map_err(io)?;
            Ok((Vec::new(), String::new()))
        }
        None => Ok((t.into_bytes(), String::new())),
    }
}

/// The command: (bytes for stdout, notice for stderr); `--help` gives the help on stdout.
pub fn run(args: Vec<String>, tmp: &Path) -> Result<(Vec<u8>, String)> {
    let mut a = args.into_iter();
    let sub = a.next().unwrap_or_default();
    let rest: Vec<String> = a.collect();
    if rest.iter().any(|x| x == "--help" || x == "-h") || sub == "--help" || sub.is_empty() {
        return Ok((format!("{HELP}\n").into_bytes(), String::new()));
    }
    match sub.as_str() {
        "add" => Ok((Vec::new(), add(rest, tmp)?)),
        "build" => Ok((Vec::new(), build(rest, tmp)?)),
        "export" => {
            let (t, out) = export(rest)?;
            deliver(t, out)
        }
        "search" => {
            let (t, out) = search(rest)?;
            deliver(t, out)
        }
        _ => Err(Error::new("err.flow.param.type")
            .with("detail", "usage: mzlab-cli msn-lib add|build|export|search")),
    }
}

/// `mzlab-cli msn-lib ...`: the library of MSn trees; the data on stdout, the notices on stderr.
pub(crate) fn cmd(args: Vec<String>) -> crate::Outcome {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-lib{k}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp)
        .map_err(io)
        .and_then(|_| run(args, &tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    match r {
        Ok((data, note)) => crate::Outcome {
            code: 0,
            stdout: data,
            stderr: note,
        },
        Err(e) => crate::Outcome::fail(&e.to_string()),
    }
}
