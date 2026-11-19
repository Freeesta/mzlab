//! `mzlab-cli msn lib add|build|export`: the library of MSn trees `msn-lib/1` of the standards (`docs/agenti/msnlib.md`). `add` puts the graph of one infusion
//! into a library file, `build` does it for every row of a manifest, `export` writes the MS2 spectra of the entries as MSP or MassBank. The library is always written by the
//! same function as the route `api/msnlib/add` (`mzlab_core::msngraph::aggiungi`).

use std::path::{Path, PathBuf};

use mzlab_core::elements::parse_formula;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Out;
use mzlab_core::msngraph::{aggiungi, cerca, Options, OpzioniCerca};

use mzlab_core::msngraph::{export_text, LibFormato as Format};

pub const HELP: &str = "mzlab-cli msn lib: the library of MSn trees (msn-lib/1).

usage: mzlab-cli msn lib add LIB.json FILE.mzML --id ID --formula ION [msn graph options] [--out PATH] [--dry-run] [--json]
       mzlab-cli msn lib build MANIFEST.tsv [--dir DIR] [--out LIB.json] [--dry-run] [--json]
       mzlab-cli msn lib export LIB.json [--clean] [--all] [--annotate] [--format msp|massbank] [--out PATH] [--json]
       mzlab-cli msn lib search LIB.json GRAPH.json [--ppm N] [--tol N] [--min-evidence N] [--top N] [--delta N] [--out PATH]

  add     adds the entry of the infusion to LIB (created when absent); the same entry twice is left as it is. LIB is rewritten unless --out.
          The options are those of `msn graph` (--help of that command). stdout: msn-lib-op/1 (library, id, added), --json is accepted for symmetry; --dry-run only checks the
          arguments, the infusion file and the library, and writes nothing (would_write: where it would)
  build   one entry per row of the manifest (tab separated, header): id, neutral_formula, expected_adduct, file (relative to DIR, by default the folder
          of the manifest); optional smiles, activation. The ion is \"<neutral_formula> <expected_adduct>\". Writes LIB.json next to the manifest unless --out.
          stdout: msn-lib-op/1 (library, ids, bytes); --dry-run checks the manifest (id, file, formula and adduct of every row) and that every file is there, and writes nothing (would_write: where it would)
  export  the MS2 spectra of every entry as MSP text (--all: every MS level); --clean keeps the peaks a node of the graph explains;
          --annotate writes the formula and the neutral loss of each explained peak (MSP: quoted note; MassBank: PK$ANNOTATION and a COMMENT);
          --format massbank writes MassBank records (MS_TYPE MS2, MS3, ... and the path of precursors in the comment). The text goes to --out or
          stdout; with --json stdout is msn-lib-export/1 (format, text_bytes, out, and the text when there is no --out)
  search  the entries of LIB whose precursor is within --ppm (10) of the root of GRAPH (the output of `msn graph`) and of the same polarity, as the JSON
          msn-search/1: three separate scores (MS2 entropy, tree, shared losses) and their proofs, no verdict. --tol: Da window of the peaks (0.01);
          --min-evidence: peaks shared in the best MS2 pair (1); --top: candidates kept (10);
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

/// The `msn-lib-op/1` document of `add` and `build`: what was done (or, with `dry_run`, would be done) to which library.
fn op_json(op: &str, lib: &Path, ids: &[&str], bytes: usize, extra: &str, dry: bool) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"msn-lib-op/1\",\"op\":").str(op);
    o.raw(",\"library\":").str(&lib.display().to_string());
    o.raw(",\"ids\":[");
    for (i, id) in ids.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(id);
    }
    o.raw("],\"dry_run\":")
        .raw(if dry { "true" } else { "false" });
    o.raw(",\"library_bytes\":")
        .int(bytes as i64)
        .raw(extra)
        .raw("}\n");
    o.0
}

/// `add LIB FILE --id ID [msn graph options]`: the `msn-lib-op/1` document and the notice for stderr.
fn add(args: Vec<String>, tmp: &Path) -> Result<(String, String)> {
    let bad = |m: &str| Error::new("err.cli.line").with("message", m);
    let (mut lib, mut id, mut rest, mut dry) = (None, None, Vec::new(), false);
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--id" => id = it.next(),
            "--dry-run" => dry = true,
            "--json" => {}
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
    let out = a.out.clone().unwrap_or_else(|| lib.clone());
    if dry {
        // the infusion and the library are read, the graph is not made
        std::fs::metadata(&a.file).map_err(io)?;
        let bytes = read_lib(&lib)?.len();
        let extra =
            ",\"would_write\":".to_string() + &Out::default().str(&out.display().to_string()).0;
        return Ok((
            op_json("add", &out, &[&id], bytes, &extra, true),
            String::new(),
        ));
    }
    let (g, json) = crate::msn_graph::graph(&a.file, &a.o, tmp)?;
    let json = crate::msn_graph::with_keys(&a, &g, json)?;
    let (text, duplicate) = aggiungi(&read_lib(&lib)?, &id, &g, &json)?;
    std::fs::write(&out, &text).map_err(io)?;
    let extra = format!(",\"added\":{}", !duplicate);
    Ok((
        op_json("add", &out, &[&id], text.len(), &extra, false),
        format!(
            "{}: {id} {}\n",
            out.display(),
            if duplicate { "already there" } else { "added" }
        ),
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

fn build(args: Vec<String>, tmp: &Path) -> Result<(String, String)> {
    let bad = |m: &str| Error::new("err.cli.line").with("message", m);
    let (mut manifest, mut dir, mut out, mut dry) = (None, None, None, false);
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--dry-run" => dry = true,
            "--json" => {}
            "--dir" => dir = it.next().map(PathBuf::from),
            "--out" => out = it.next().map(PathBuf::from),
            _ if x.starts_with("--") => return Err(bad(&format!("unknown option {x}"))),
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
    let mut ids = Vec::new();
    for row in rows(&std::fs::read_to_string(&manifest).map_err(io)?)? {
        let (id, file) = (column(&row, "id"), column(&row, "file"));
        if id.is_empty() || file.is_empty() {
            return Err(bad("every row needs id and file"));
        }
        ids.push(id.to_string());
        let (neutral, adduct) = (
            column(&row, "neutral_formula"),
            column(&row, "expected_adduct"),
        );
        if neutral.is_empty() || adduct.is_empty() {
            return Err(Error::new("err.msnlib.noIon"));
        }
        parse_formula(neutral)?;
        if dry {
            // every file of the manifest must be there; no graph is made
            std::fs::metadata(folder.join(file)).map_err(io)?;
            continue;
        }
        let ion = format!("{neutral} {adduct}");
        let none = |s: &str| (!s.is_empty()).then(|| s.to_string());
        let o = Options {
            formula: Some(ion),
            smiles: none(column(&row, "smiles")),
            activation: none(column(&row, "activation")),
            ..Options::default()
        };
        let (g, json) = crate::msn_graph::graph(&folder.join(file), &o, tmp)?;
        text = aggiungi(&text, id, &g, &json)?.0;
    }
    let names: Vec<&str> = ids.iter().map(String::as_str).collect();
    if dry {
        let extra =
            ",\"would_write\":".to_string() + &Out::default().str(&out.display().to_string()).0;
        return Ok((
            op_json("build", &out, &names, 0, &extra, true),
            String::new(),
        ));
    }
    std::fs::write(&out, &text).map_err(io)?;
    Ok((
        op_json("build", &out, &names, text.len(), "", false),
        format!("{}: {} bytes\n", out.display(), text.len()),
    ))
}

fn export(args: Vec<String>) -> Result<(String, Option<PathBuf>, bool, &'static str)> {
    let bad = |m: &str| Error::new("err.cli.line").with("message", m);
    let (mut lib, mut out, mut clean, mut all, mut annotate) = (None, None, false, false, false);
    let mut json = false;
    let mut lib_format = Format::Msp;
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--clean" => clean = true,
            "--all" => all = true,
            "--annotate" => annotate = true,
            "--json" => json = true,
            "--format" => lib_format = Format::parse(&it.next().unwrap_or_default())?,
            "--out" => out = it.next().map(PathBuf::from),
            _ if x.starts_with("--") => return Err(bad(&format!("unknown option {x}"))),
            _ if lib.is_none() => lib = Some(PathBuf::from(x)),
            _ => return Err(bad(&format!("unknown argument {x}"))),
        }
    }
    let lib = lib.ok_or_else(|| bad("a library file is needed"))?;
    let name = match lib_format {
        Format::Msp => "msp",
        _ => "massbank",
    };
    Ok((
        export_text(
            &std::fs::read_to_string(&lib).map_err(io)?,
            lib_format,
            (clean, all, annotate),
        )?,
        out,
        json,
        name,
    ))
}

/// The `msn-lib-export/1` document of `export --json`: the text is in it only when it did not go to `--out`.
fn export_json(text: &str, out: &Option<PathBuf>, format: &str) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"msn-lib-export/1\",\"format\":")
        .str(format);
    o.raw(",\"text_bytes\":").int(text.len() as i64);
    match out {
        Some(p) => o.raw(",\"out\":").str(&p.display().to_string()),
        None => o.raw(",\"out\":null,\"text\":").str(text),
    };
    o.raw("}\n");
    o.0
}

/// `search LIB GRAPH [options]`: the text of `msn-search/1` and the file it goes to, if any.
fn search(args: Vec<String>) -> Result<(String, Option<PathBuf>)> {
    let bad = |m: &str| Error::new("err.cli.line").with("message", m);
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
            "--min-evidence" => o.min_evidence = number()? as usize,
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
        "add" => {
            let (doc, note) = add(rest, tmp)?;
            Ok((doc.into_bytes(), note))
        }
        "build" => {
            let (doc, note) = build(rest, tmp)?;
            Ok((doc.into_bytes(), note))
        }
        "export" => {
            let (t, out, json, format) = export(rest)?;
            if json {
                let doc = export_json(&t, &out, format);
                if let Some(p) = &out {
                    std::fs::write(p, &t).map_err(io)?;
                }
                return Ok((doc.into_bytes(), String::new()));
            }
            deliver(t, out)
        }
        "search" => {
            let (t, out) = search(rest)?;
            deliver(t, out)
        }
        _ => Err(Error::new("err.cli.line").with(
            "message",
            "usage: mzlab-cli msn lib add|build|export|search",
        )),
    }
}

/// `mzlab-cli msn lib ...`: the library of MSn trees; the data on stdout, the notices on stderr.
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
            error: None,
        },
        Err(e) => crate::Outcome::fail_error(&e),
    }
}
