//! `mzlab-cli msn-graph FILE --formula ION [...]`: the fragmentation graph `msngrafo/1` of an MSn infusion, from the engine of the app.
//! With `--verita FILE --riassunto` it prints only numbers: how many paths of a truth file (`{nodes: [{path: [nominal m/z...], formula}]}`)
//! the graph finds and with which sub-formulas, so a check on private data leaves no data behind.

use std::path::{Path, PathBuf};

use mzlab_core::budget::sha256_hex;
use mzlab_core::elements::{fmt, parse_formula};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out};
use mzlab_core::lettori_mzml::Picchi;
use mzlab_core::msnalbero::Raggruppa;
use mzlab_core::msngrafo::{
    con_interpretazione, esporta, grafo_json, Formato, Grafo, Opzioni, PPM_PERDITA,
};
use mzlab_core::mzidx::DEFAULT_BUDGET;
use mzlab_core::mzmlwrite::Opzioni as MzmlOpzioni;

pub const HELP: &str = "mzlab-cli msn-graph: the fragmentation graph (msngrafo/1) of an MSn infusion (mzML or a vendor file such as a Thermo .raw).

usage: mzlab-cli msn-graph FILE.mzML [--formula ION] [options] [--out PATH]
       mzlab-cli msn-graph FILE.mzML --formula ION --verita TRUTH.json --riassunto
       mzlab-cli msn-graph compare A.json B.json [--out PATH]

  --formula ION        the ion of the root: C13H25N4O3S+, C13H24N4O3S+H or \"C13H24N4O3S [M+H]+\"; without it, masses only
  --smiles TEXT        written back in the JSON; with --strutture, the structure that is cleaved
  --strutture NODO     add to the JSON the key `strutture`: the bonds of --smiles whose cleavage gives the m/z of the node nK of the graph (candidates, with the
                       ppm and the peaks of its spectrum that the same piece explains)
  --interpreta         add to the JSON the key `interpretazione`: for each arc the neutral loss (name, formula, exact mass, ppm, reason, confidence) and the
                       activations that observed it
  --ppm-perdita N      window of the loss match by mass, ppm of the precursor m/z (10)
  --ppm-struttura N    window of those cleavages, ppm (10)
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
  --nodo ID            with --export: nK = the spectra of which node nK of the graph is the precursor; msn:<id> or <id> = those of that node of
                       the tree; tutto = all
  --out PATH           write there instead of the standard output

  compare              validates a tree measured twice: two msngrafo/1 files in, a JSON (msnconfronto/1) out with the numbers of the five criteria
                       (isolated nodes, arcs of rel >= 10 %, formulas, MS2 entropy >= 0.8 per energy, m/z shift <= 3 ppm) and `convalidata`";

/// Default window of the cleavages of `--strutture`, ppm.
pub const PPM_STRUTTURA: f64 = 10.0;

pub struct Args {
    pub(crate) file: PathBuf,
    pub(crate) o: Opzioni,
    verita: Option<PathBuf>,
    riassunto: bool,
    export: Option<(Formato, bool, Option<String>)>,
    strutture: Option<(String, f64)>,
    interpreta: Option<f64>,
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
    let (mut strutture, mut ppm_struttura) = (None, PPM_STRUTTURA);
    let (mut interpreta, mut ppm_perdita) = (false, PPM_PERDITA);
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
            "--strutture" => strutture = a.next(),
            "--interpreta" => interpreta = true,
            "--ppm-perdita" => ppm_perdita = numero(a.next(), &x)?,
            "--ppm-struttura" => ppm_struttura = numero(a.next(), &x)?,
            "--out" => out = a.next().map(PathBuf::from),
            f if !f.starts_with("--") && file.is_none() => file = Some(PathBuf::from(f)),
            f => return Err(format!("unknown argument {f}")),
        }
    }
    let file = file.ok_or("an infusion file is needed")?;
    if riassunto && verita.is_none() {
        return Err("--riassunto needs --verita".into());
    }
    let export = export.map(|f| (f, pulito, nodo));
    let strutture = strutture.map(|n| (n, ppm_struttura));
    let interpreta = interpreta.then_some(ppm_perdita);
    Ok(Some(Args {
        file,
        o,
        verita,
        riassunto,
        export,
        strutture,
        interpreta,
        out,
    }))
}

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// The graph of `file`, its index built in `tmp`: an mzML is read as it is, any other dataset a reader takes (a Thermo `.raw`, ...) is first written to
/// `tmp` as mzML with the peaks of the file.
pub fn grafo(file: &Path, o: &Opzioni, tmp: &Path) -> Result<(Grafo, String)> {
    let nome = file
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let converted = tmp.join("0.mzML");
    let is_mzml = nome.to_lowercase().ends_with(".mzml");
    if !is_mzml {
        let (peaks, opz) = (Picchi::File, MzmlOpzioni::default());
        crate::flow_base::write_mzml(file, None, &converted, peaks, opz)?;
    }
    let source = if is_mzml { file } else { &converted };
    let idx = tmp.join("0.mzidx");
    crate::build_index(source, &idx, DEFAULT_BUDGET)?;
    let sha = sha256_hex(&std::fs::read(&idx).map_err(io)?);
    let scans = crate::open(&idx)?.scans;
    let mut m = std::fs::File::open(source).map_err(io)?;
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

/// The graph text `j` of `g` with the key `strutture` of node `nodo` (`nK` or `K`): the cleavages of `o.smiles` that give its m/z within `ppm`.
pub fn strutture_testo(
    g: &Grafo,
    j: &str,
    o: &Opzioni,
    (nodo, ppm): (&str, f64),
) -> Result<String> {
    mzlab_feat::chem::strutture::strutture_testo(g, j, o.smiles.as_deref(), (nodo, ppm))
}

/// The text the command prints: the graph, or the line of numbers.
pub fn esegui(a: &Args, tmp: &Path) -> Result<String> {
    let (g, mut j) = grafo(&a.file, &a.o, tmp)?;
    if let Some(ppm) = a.interpreta {
        j = con_interpretazione(&j, &g, ppm);
    }
    if let Some((f, pulito, nodo)) = &a.export {
        return Ok(esporta(&g, *f, *pulito, nodo.as_deref()));
    }
    if let Some((nodo, ppm)) = &a.strutture {
        return Ok(strutture_testo(&g, &j, &a.o, (nodo, *ppm))? + "\n");
    }
    if !a.riassunto {
        return Ok(j + "\n");
    }
    let v =
        std::fs::read_to_string(a.verita.as_ref().expect("checked by parse_args")).map_err(io)?;
    riassunto(&g, &v)
}
