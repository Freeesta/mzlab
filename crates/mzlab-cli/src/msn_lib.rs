//! `mzlab-cli msn-lib add|build|export`: the library of MSn trees `msnlib/1` of the standards (`docs/agenti/msnlib.md`). `add` puts the graph of one infusion
//! into a library file, `build` does it for every row of a manifest, `export` writes the MS2 spectra of the entries as MSP. The library is always written by the
//! same function as the route `api/msnlib/aggiungi` (`mzlab_core::msngrafo::aggiungi`).

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};
use mzlab_core::msngrafo::{aggiungi, Opzioni};

pub const HELP: &str = "mzlab-cli msn-lib: the library of MSn trees (msnlib/1).

usage: mzlab-cli msn-lib add LIB.json FILE.mzML --id ID --formula ION [msn-graph options] [--out PATH]
       mzlab-cli msn-lib build MANIFEST.tsv [--dir DIR] [--out LIB.json]
       mzlab-cli msn-lib export LIB.json [--pulito] [--tutti] [--out PATH]

  add     adds the entry of the infusion to LIB (created when absent); the same entry twice is left as it is. LIB is rewritten unless --out.
          The options are those of `msn-graph` (--help of that command)
  build   one entry per row of the manifest (tab separated, header): id, formula_neutra, addotto_atteso, file (relative to DIR, by default the folder
          of the manifest); optional smiles, attivazione. The ion is \"<formula_neutra> <addotto_atteso>\". Writes LIB.json next to the manifest unless --out
  export  the MS2 spectra of every entry as MSP text (--tutti: every MS level); --pulito keeps the peaks a node of the graph explains";

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

fn peaks(s: &Value, pulito: bool) -> Vec<(f64, f64)> {
    let all = s.get("picchi").map_or(&[][..], |p| p.arr());
    let keep = |p: &&Value| !pulito || p.get("nodo").is_some_and(|n| n.str().is_some());
    all.iter()
        .filter(keep)
        .filter_map(|p| Some((p.get("mz")?.f64()?, p.get("rel")?.f64()?)))
        .collect()
}

/// The MSP text of a library: one record per spectrum (MS2 only unless `tutti`) of every entry.
pub fn export_text(lib: &str, pulito: bool, tutti: bool) -> Result<String> {
    let v = parse(lib.as_bytes())?;
    let mut t = String::new();
    for e in v.get("voci").map_or(&[][..], |x| x.arr()) {
        let (id, g) = (
            e.get("id").and_then(|x| x.str()).unwrap_or(""),
            e.get("grafo").ok_or_else(|| Error::new("err.msnlib.bad"))?,
        );
        let nodi = g.get("nodi").map_or(&[][..], |x| x.arr());
        let ion = e
            .get("ione")
            .and_then(|i| i.get("formula"))
            .and_then(|x| x.str())
            .unwrap_or("");
        let modo = if g
            .get("polarita")
            .and_then(|p| p.f64())
            .is_some_and(|p| p < 0.0)
        {
            "N"
        } else {
            "P"
        };
        for s in g.get("spettri").map_or(&[][..], |x| x.arr()) {
            let livello = s.get("livello").and_then(|x| x.f64()).unwrap_or(0.0) as i64;
            if livello != 2 && !tutti {
                continue;
            }
            let pre = s
                .get("precursore")
                .and_then(|p| p.str())
                .and_then(|p| p.strip_prefix('n')?.parse::<usize>().ok());
            let mz = pre
                .and_then(|k| nodi.get(k))
                .and_then(|n| n.get("mz")?.f64())
                .unwrap_or(0.0);
            let (alb, att, ce) = (
                s.get("albero").and_then(|x| x.str()).unwrap_or(""),
                s.get("att").and_then(|x| x.str()).unwrap_or(""),
                s.get("ce").and_then(|x| x.f64()).unwrap_or(0.0),
            );
            let pk = peaks(s, pulito);
            t += &format!("Name: {id} {alb} MS{livello} {att} CE{ce}\nPrecursorMZ: {mz:.5}\nFormula: {ion}\nIon_mode: {modo}\n");
            t += &format!("Collision_energy: {ce}\nNum Peaks: {}\n", pk.len());
            for (m, r) in pk {
                t += &format!("{m:.5} {r:.2}\n");
            }
            t.push('\n');
        }
    }
    Ok(t)
}

fn export(args: Vec<String>) -> Result<(String, Option<PathBuf>)> {
    let bad = |m: &str| Error::new("err.flow.param.type").with("detail", m);
    let (mut lib, mut out, mut pulito, mut tutti) = (None, None, false, false);
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--pulito" => pulito = true,
            "--tutti" => tutti = true,
            "--out" => out = it.next().map(PathBuf::from),
            _ if lib.is_none() => lib = Some(PathBuf::from(x)),
            _ => return Err(bad(&format!("unknown argument {x}"))),
        }
    }
    let lib = lib.ok_or_else(|| bad("a library file is needed"))?;
    Ok((
        export_text(&std::fs::read_to_string(&lib).map_err(io)?, pulito, tutti)?,
        out,
    ))
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
            match out {
                Some(p) => {
                    std::fs::write(&p, t).map_err(io)?;
                    Ok((Vec::new(), String::new()))
                }
                None => Ok((t.into_bytes(), String::new())),
            }
        }
        _ => Err(Error::new("err.flow.param.type")
            .with("detail", "usage: mzlab-cli msn-lib add|build|export")),
    }
}
