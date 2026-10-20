//! `mzlab-cli msn-graph FILE --formula ION [...]`: the fragmentation graph `msngrafo/1` of an MSn infusion, from the engine of the app.
//! With `--verita FILE --riassunto` it prints only numbers: how many paths of a truth file (`{nodes: [{path: [nominal m/z...], formula}]}`)
//! the graph finds and with which sub-formulas, so a check on private data leaves no data behind.

use std::path::{Path, PathBuf};

use mzlab_core::budget::sha256_hex;
use mzlab_core::elements::{fmt, parse_formula};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out};
use mzlab_core::msnalbero::Raggruppa;
use mzlab_core::msngrafo::{esporta, grafo_json, Formato, Grafo, Opzioni};
use mzlab_core::mzidx::DEFAULT_BUDGET;

pub const HELP: &str = "mzlab-cli msn-graph: the fragmentation graph (msngrafo/1) of an MSn infusion.

usage: mzlab-cli msn-graph FILE.mzML [--formula ION] [options] [--out PATH]
       mzlab-cli msn-graph FILE.mzML --formula ION --verita TRUTH.json --riassunto

  --formula ION        the ion of the root: C13H25N4O3S+, C13H24N4O3S+H or \"C13H24N4O3S [M+H]+\"; without it, masses only
  --smiles TEXT        written back in the JSON
  --ppm-formula N      window of the sub-formulas, ppm (10)
  --mz-basso N         below this m/z the window may widen (150)
  --ppm-basso N        the wider window, ppm (20), for a candidate with --prove-min evidences (1)
  --attivazione A      only the spectra of this activation (cid, hcd, ...)
  --isotopi            keep the M+1 peaks as nodes
  --scelta MZ=ION      the formula chosen for the node at MZ (repeatable)
  --verita FILE        paths and formulas expected ({nodes: [{path, formula}]})
  --riassunto          one line of numbers instead of the graph
  --export FORMAT      the spectra of the tree as csv, mgf or msp text instead of the graph
  --pulito             with --export: only the peaks a node of the graph explains
  --nodo ID            with --export: only the spectra of that node of the tree
  --out PATH           write there instead of the standard output";

pub struct Args {
    file: PathBuf,
    o: Opzioni,
    verita: Option<PathBuf>,
    riassunto: bool,
    export: Option<(Formato, bool, Option<String>)>,
    pub out: Option<PathBuf>,
}

fn numero(v: Option<String>, k: &str) -> std::result::Result<f64, String> {
    v.and_then(|x| x.parse().ok())
        .ok_or(format!("{k} needs a number"))
}

/// The arguments after `msn-graph`; `Ok(None)` = `--help`.
pub fn parse_args(
    mut a: impl Iterator<Item = String>,
) -> std::result::Result<Option<Args>, String> {
    let (mut file, mut o) = (None, Opzioni::default());
    let (mut verita, mut riassunto, mut out) = (None, false, None);
    let (mut export, mut pulito, mut nodo) = (None, false, None);
    while let Some(x) = a.next() {
        match x.as_str() {
            "--help" | "-h" => return Ok(None),
            "--formula" => o.formula = a.next(),
            "--smiles" => o.smiles = a.next(),
            "--ppm-formula" => o.ppm_formula = numero(a.next(), &x)?,
            "--mz-basso" => o.mz_basso = numero(a.next(), &x)?,
            "--ppm-basso" => o.ppm_basso = numero(a.next(), &x)?,
            "--prove-min" => o.prove_min = numero(a.next(), &x)? as usize,
            "--attivazione" => o.attivazione = a.next(),
            "--isotopi" => o.isotopi = true,
            "--scelta" => {
                let v = a.next().unwrap_or_default();
                let (m, f) = v.split_once('=').ok_or("--scelta needs MZ=ION")?;
                o.scelte.push((numero(Some(m.into()), &x)?, f.into()));
            }
            "--verita" => verita = a.next().map(PathBuf::from),
            "--riassunto" => riassunto = true,
            "--export" => {
                export = Some(
                    Formato::da_nome(&a.next().unwrap_or_default())
                        .map_err(|_| "--export needs csv, mgf or msp")?,
                )
            }
            "--pulito" => pulito = true,
            "--nodo" => nodo = a.next(),
            "--out" => out = a.next().map(PathBuf::from),
            f if !f.starts_with("--") && file.is_none() => file = Some(PathBuf::from(f)),
            f => return Err(format!("unknown argument {f}")),
        }
    }
    let file = file.ok_or("an mzML file is needed")?;
    if riassunto && verita.is_none() {
        return Err("--riassunto needs --verita".into());
    }
    let export = export.map(|f| (f, pulito, nodo));
    Ok(Some(Args {
        file,
        o,
        verita,
        riassunto,
        export,
        out,
    }))
}

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// The graph of `file` (an mzML), its index built in `tmp`.
pub fn grafo(file: &Path, o: &Opzioni, tmp: &Path) -> Result<(Grafo, String)> {
    let idx = tmp.join("0.mzidx");
    crate::build_index(file, &idx, DEFAULT_BUDGET)?;
    let sha = sha256_hex(&std::fs::read(&idx).map_err(io)?);
    let scans = crate::open(&idx)?.scans;
    let mut m = std::fs::File::open(file).map_err(io)?;
    let nome = file
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    grafo_json(&scans, &mut m, (&nome, &sha), (o, &Raggruppa::default()))
}

/// The nominal path of a node of the tree (`317@cid35>261@cid35` → [317, 261]).
fn percorso(id: &str) -> Vec<i64> {
    id.split('>')
        .filter_map(|p| p.split('@').next()?.parse::<f64>().ok())
        .map(|m| m.round() as i64)
        .collect()
}

fn hill(f: &str) -> String {
    parse_formula(f.trim_end_matches(['+', '-'])).map_or(f.to_string(), |x| fmt(&x))
}

/// Nodes of each MS level, and the peaks of the spectra of each level: kept (with a node) and discarded (unexplained).
fn per_livello(o: &mut Out, g: &Grafo) {
    o.raw("\"per_livello\":{");
    let max = g.nodi.iter().map(|n| n.livello).max().unwrap_or(0);
    for l in 1..=max {
        let k = g.nodi.iter().filter(|n| n.livello == l).count();
        o.raw(if l > 1 { "," } else { "" })
            .str(&l.to_string())
            .raw(":")
            .int(k as i64);
    }
    // peaks of the spectra of each MS level: kept (with a node) and discarded (unexplained)
    o.raw("},\"picchi_per_livello\":{");
    let lv = g.spettri.iter().map(|s| s.livello).max().unwrap_or(0);
    let mut primo = true;
    for l in 2..=lv {
        let pk = g
            .spettri
            .iter()
            .filter(|s| s.livello == l)
            .flat_map(|s| &s.picchi);
        let (t, d) = pk.fold((0, 0), |(t, d), p| {
            if p.2.is_some() {
                (t + 1, d)
            } else {
                (t, d + 1)
            }
        });
        if t + d > 0 {
            o.raw(if primo { "" } else { "," }).str(&l.to_string());
            o.raw(":[").int(t).raw(",").int(d).raw("]");
            primo = false;
        }
    }
    o.raw("}");
}

/// One line of numbers: the graph against the paths of a truth file.
pub fn riassunto(g: &Grafo, verita: &str) -> Result<String> {
    let v = parse(verita.as_bytes())?;
    let attesi: Vec<(Vec<i64>, String)> = v
        .get("nodes")
        .map_or(&[][..], |n| n.arr())
        .iter()
        .filter_map(|n| {
            let p = n
                .get("path")?
                .arr()
                .iter()
                .filter_map(|x| x.f64())
                .map(|m| m.round() as i64)
                .collect();
            Some((p, hill(n.get("formula")?.str()?)))
        })
        .collect();
    let (mut trovati, mut fra, mut unica, mut ppm_max) = (0, 0, 0, 0.0f64);
    for (p, f) in &attesi {
        let Some(n) = g
            .nodi
            .iter()
            .find(|n| n.albero.iter().any(|t| percorso(t) == *p))
        else {
            continue;
        };
        trovati += 1;
        let Some(x) = &n.formula else { continue };
        if let Some(c) = x.candidati.iter().find(|c| c.formula == *f) {
            fra += 1;
            ppm_max = ppm_max.max(c.ppm.abs());
        }
        if x.scelta.as_deref() == Some(f) {
            unica += 1;
        }
    }
    let mut o = Out::default();
    o.raw("{\"percorsi_verita\":").int(attesi.len() as i64);
    o.raw(",\"percorsi_trovati\":").int(trovati);
    o.raw(",\"formula_giusta_fra_candidati\":").int(fra);
    o.raw(",\"unica_e_giusta\":").int(unica);
    o.raw(",\"ppm_max_giuste\":")
        .num(mzlab_core::json::round(ppm_max, 2));
    let annotati = g.nodi.iter().filter(|n| n.formula.is_some()).count();
    o.raw(",\"frammenti_annotati\":")
        .int(annotati.saturating_sub(1) as i64);
    o.raw(",\"scartati_formula\":")
        .int(g.scartati.formula as i64);
    o.raw(",\"avvisi\":").int(g.avvisi.len() as i64).raw(",");
    per_livello(&mut o, g);
    o.raw("}\n");
    Ok(o.0)
}

/// The text the command prints: the graph, or the line of numbers.
pub fn esegui(a: &Args, tmp: &Path) -> Result<String> {
    let (g, j) = grafo(&a.file, &a.o, tmp)?;
    if let Some((f, pulito, nodo)) = &a.export {
        return Ok(esporta(&g, *f, *pulito, nodo.as_deref()));
    }
    if !a.riassunto {
        return Ok(j + "\n");
    }
    let v =
        std::fs::read_to_string(a.verita.as_ref().expect("checked by parse_args")).map_err(io)?;
    riassunto(&g, &v)
}
