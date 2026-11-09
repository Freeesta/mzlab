//! `mzlab-cli msn-graph FILE --formula ION [...]`: the fragmentation graph `msngrafo/1` of an MSn infusion, from the engine of the app.
//! With `--verita FILE --riassunto` it prints only numbers: how many paths of a truth file (`{nodes: [{path: [nominal m/z...], formula}]}`)
//! the graph finds and with which sub-formulas, so a check on private data leaves no data behind.

use std::path::{Path, PathBuf};

use crate::Outcome;

use mzlab_core::budget::sha256_hex;
use mzlab_core::elements::{fmt, parse_formula};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out};
use mzlab_core::lettori_mzml::Peaks;
use mzlab_core::msnalbero::Grouping;
use mzlab_core::msngrafo::{
    con_interpretazione, esporta, grafo_json, Format, Graph, Options, RuleParams, PPM_PERDITA,
};
use mzlab_core::mzmlwrite::Options as MzmlOpzioni;

pub const HELP: &str = "mzlab-cli msn-graph: the fragmentation graph (msngrafo/2) of an MSn infusion (mzML or a vendor file such as a Thermo .raw).

usage: mzlab-cli msn-graph FILE.mzML [--formula ION] [options] [--out PATH]
       mzlab-cli msn-graph FILE.mzML --formula ION --truth TRUTH.json --summary
       mzlab-cli msn-graph compare A.json B.json [--out PATH]
       mzlab-cli msn-graph complete IN.json [--out PATH]
       mzlab-cli msn-graph figure IN.json [--figure svg,html,png] [--view tree|radial] [--out STEM]

  --formula ION        the ion of the root: C13H25N4O3S+, C13H24N4O3S+H or \"C13H24N4O3S [M+H]+\"; without it, masses only
  --smiles TEXT        written back in the JSON; with --structures, the structure that is cleaved
  --molfile FILE       the 2D structure of the molecule (a molfile), written back in the JSON for the drawings of the root and of the fragments
  --structures NODE    add to the JSON the key `strutture`: the bonds of --smiles whose cleavage gives the m/z of the node nK of the graph (candidates, with the
                       ppm and the peaks of its spectrum that the same piece explains); `all` = the key `strutture_tutti`, those of every node of level 2
                       or more that has a cleavage
  --interpret          add to the JSON the key `interpretazione`: for each arc the neutral loss (name, formula, exact mass, ppm, reason, confidence) and the
                       activations that observed it (param interpret)
  --loss-ppm N         window of the loss match by mass, ppm of the precursor m/z (10) (param loss_ppm)
  --rules              add to the JSON the key `regole`: the fragmentation rules of the table `fragrules/1` that the formulas alone decide (which rule explains the loss
                       of an arc, even-electron check, trap cut-off, composite and stepped losses) and the evidence `regola` of the candidates (param rules)
  --cutoff-q N         q of the trap cut-off, fragments below q/0.908 of the precursor m/z are not observable in a trap CID (0.25) (param cutoff_q)
  --loss-tol-mda N     tolerance of a loss against a rule, mDa (3) (param loss_tol_mda)
  --breaks             add to the JSON the key `rotture`: the sets of atoms of --smiles (one or two bonds cut, a few hydrogens moved) that give the m/z of each
                       node, the arcs they explain and the share of each spectrum's intensity explained (hypotheses, never an assignment) (param breaks)
  --h-max N            hydrogens moved at most beyond the plain cleavage, for --breaks (2) (param h_max)
  --max-bonds N        bonds cut at most, 1 or 2, for --breaks (2) (param max_bonds)
  --struct-ppm N       window of those cleavages and of --breaks, ppm (10) (param struct_ppm)
  --ppm-formula N      window of the sub-formulas, ppm (10) (param ppm_formula)
  --low-mz N           below this m/z the window may widen (150) (param low_mz)
  --wide-ppm N         the wider window, ppm (20), for a candidate with --min-evidence evidences (1) (params wide_ppm, min_evidence)
  --activation A       only the spectra of this activation (cid, hcd, ...) (param activation)
  --isotopes           keep the M+1 peaks as nodes (param isotopes)
  --pick MZ=ION        the formula chosen for the node at MZ (repeatable) (param pick)
  --truth FILE         paths and formulas expected ({nodes: [{path, formula}]})
  --summary            one line of numbers instead of the graph
  --export FORMAT      the spectra of the tree as csv, mgf or msp text instead of the graph
  --clean              with --export: only the peaks a node of the graph explains
  --node ID            with --export: nK = the spectra of which node nK of the graph is the precursor; msn:<id> or <id> = those of that node of
                       the tree; tutto = all
  --figure LIST        with --out: also write the figures of the graph next to it (g.json → g.svg, g.html, g.png): svg, html (one file, no external
                       resource, two views, data table) and png (only in a build with the cargo feature `png`)
  --view V             the view the figures open on: tree (default) or radial
  --out PATH           write there instead of the standard output

  figure               the figures of a graph already made (after `complete` if it has no `vista`): STEM.svg and STEM.html, STEM.png with --figure
  complete             adds the key `vista` (levels, best path, tree and radial layout) to an msngrafo/1 that has not got it, with the bytes the engine writes
  compare              validates a tree measured twice: two msngrafo/1 files in, a JSON (msnconfronto/1) out with the numbers of the five criteria
                       (isolated nodes, arcs of rel >= 10 %, formulas, MS2 entropy >= 0.8 per energy, m/z shift <= 3 ppm) and `convalidata`

old names still accepted: --mz-basso --ppm-basso --prove-min --attivazione --isotopi --scelta --verita --riassunto --pulito --nodo --strutture --interpreta --ppm-perdita --regole --toll-perdita-mda --figura --vista --rotture --max-legami --ppm-struttura (values albero|radiale, tutti)";

/// Old flag names of `msn-graph` (old, new); resolved before the match in `parse_args`.
#[rustfmt::skip]
const ALIASES: &[(&str, &str)] = &[
    ("--mz-basso", "--low-mz"), ("--ppm-basso", "--wide-ppm"), ("--prove-min", "--min-evidence"),
    ("--attivazione", "--activation"), ("--isotopi", "--isotopes"), ("--scelta", "--pick"),
    ("--verita", "--truth"), ("--riassunto", "--summary"), ("--pulito", "--clean"),
    ("--nodo", "--node"), ("--strutture", "--structures"), ("--interpreta", "--interpret"),
    ("--ppm-perdita", "--loss-ppm"), ("--regole", "--rules"), ("--toll-perdita-mda", "--loss-tol-mda"),
    ("--figura", "--figure"), ("--vista", "--view"), ("--rotture", "--breaks"),
    ("--max-legami", "--max-bonds"), ("--ppm-struttura", "--struct-ppm"),
];

/// Default window of the cleavages of `--structures`, ppm.
pub const PPM_STRUTTURA: f64 = 10.0;

use mzlab_feat::chem::rotture::Parametri;

/// The flags of the bond breaks and the window shared with `--structures`.
struct Rot {
    on: bool,
    p: Parametri,
}

impl Default for Rot {
    fn default() -> Self {
        Rot {
            on: false,
            p: Parametri {
                ppm: PPM_STRUTTURA,
                ..Parametri::default()
            },
        }
    }
}

impl Rot {
    /// Takes the flag `x` if it is one of `--breaks`, `--h-max`, `--max-bonds`, `--struct-ppm`.
    fn flag(
        &mut self,
        x: &str,
        a: &mut impl Iterator<Item = String>,
    ) -> std::result::Result<bool, String> {
        match x {
            "--breaks" => self.on = true,
            "--h-max" => self.p.h_max = number(a.next(), x)?.clamp(0.0, 4.0) as u8,
            "--max-bonds" => self.p.max_legami = number(a.next(), x)?.clamp(1.0, 2.0) as u8,
            "--struct-ppm" => self.p.ppm = number(a.next(), x)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

pub struct Args {
    pub(crate) file: PathBuf,
    pub(crate) o: Options,
    verita: Option<PathBuf>,
    summary: bool,
    export: Option<(Format, bool, Option<String>)>,
    structures: Option<(String, f64)>,
    interpreta: Option<f64>,
    cleavages: Option<Parametri>,
    figure: Vec<crate::msn_uscite::Format>,
    view: mzlab_feat::msnfigura::Vista,
    pub out: Option<PathBuf>,
}

/// The value of `--structures`: `all` is the engine's `tutti`.
#[rustfmt::skip]
fn all(v: String) -> String {
    if v == "all" { "tutti".into() } else { v }
}

fn number(v: Option<String>, k: &str) -> std::result::Result<f64, String> {
    v.and_then(|x| x.parse().ok())
        .ok_or(format!("{k} needs a number"))
}

/// The arguments after `msn-graph`; `Ok(None)` = `--help`.
/// The view named after `--view`.
fn view_of(n: Option<String>) -> std::result::Result<mzlab_feat::msnfigura::Vista, String> {
    mzlab_feat::msnfigura::Vista::da_nome(&n.unwrap_or_default())
        .ok_or_else(|| "--view needs tree or radial".into())
}

/// The refusal of `--figure` without `--out` or together with an output that is not the graph.
const FIGURE_USAGE: &str =
    "--figure needs --out and the graph itself (not --summary, --export or --structures)";

pub fn parse_args(
    mut a: impl Iterator<Item = String>,
) -> std::result::Result<Option<Args>, String> {
    let (mut file, mut o) = (None, Options::default());
    let (mut verita, mut riassunto, mut out) = (None, false, None);
    let (mut export, mut clean, mut node) = (None, false, None);
    let (mut strutture, mut rot) = (None, Rot::default());
    let (mut interpreta, mut loss_ppm) = (false, PPM_PERDITA);
    let (mut rules, mut rules_p) = (false, RuleParams::default());
    let (mut figura, mut vista) = (Vec::new(), mzlab_feat::msnfigura::Vista::Albero);
    while let Some(x) = a.next().map(|x| crate::msn_uscite::canonical(ALIASES, x)) {
        match x.as_str() {
            "--help" | "-h" => return Ok(None),
            "--formula" => o.formula = a.next(),
            "--smiles" => o.smiles = a.next(),
            "--molfile" => {
                let p = a.next().ok_or("--molfile needs a file")?;
                o.molfile = Some(std::fs::read_to_string(&p).map_err(|e| format!("{p}: {e}"))?);
            }
            "--ppm-formula" => o.ppm_formula = number(a.next(), &x)?,
            "--low-mz" => o.mz_low = number(a.next(), &x)?,
            "--wide-ppm" => o.ppm_low = number(a.next(), &x)?,
            "--min-evidence" => o.evidence_min = number(a.next(), &x)? as usize,
            "--activation" => o.activation = a.next(),
            "--isotopes" => o.isotopi = true,
            "--pick" => {
                let v = a.next().unwrap_or_default();
                let (m, f) = v.split_once('=').ok_or("--pick needs MZ=ION")?;
                o.choices.push((number(Some(m.into()), &x)?, f.into()));
            }
            "--truth" => verita = a.next().map(PathBuf::from),
            "--summary" => riassunto = true,
            "--export" => {
                export = Some(
                    Format::from_name(&a.next().unwrap_or_default())
                        .map_err(|_| "--export needs csv, mgf or msp")?,
                )
            }
            "--clean" => clean = true,
            "--node" => node = a.next(),
            "--structures" => strutture = a.next().map(all),
            "--interpret" => interpreta = true,
            "--loss-ppm" => loss_ppm = number(a.next(), &x)?,
            "--rules" => rules = true,
            "--cutoff-q" => rules_p.cutoff_q = number(a.next(), &x)?,
            "--loss-tol-mda" => rules_p.toll_mda = number(a.next(), &x)?,
            x if rot.flag(x, &mut a)? => {}
            "--figure" => figura = crate::msn_uscite::formats(&a.next().unwrap_or_default())?,
            "--view" => vista = view_of(a.next())?,
            "--out" => out = a.next().map(PathBuf::from),
            f if !f.starts_with("--") && file.is_none() => file = Some(PathBuf::from(f)),
            f => return Err(format!("unknown argument {f}")),
        }
    }
    let file = file.ok_or("an infusion file is needed")?;
    if !figura.is_empty() && (out.is_none() || riassunto || export.is_some() || strutture.is_some())
    {
        return Err(FIGURE_USAGE.into());
    }
    if riassunto && verita.is_none() {
        return Err("--summary needs --truth".into());
    }
    o.regole = rules.then_some(rules_p);
    let export = export.map(|f| (f, clean, node));
    let strutture = strutture.map(|n| (n, rot.p.ppm));
    let interpreta = interpreta.then_some(loss_ppm);
    Ok(Some(Args {
        file,
        o,
        verita,
        summary: riassunto,
        export,
        structures: strutture,
        interpreta,
        cleavages: rot.on.then_some(rot.p),
        figure: figura,
        view: vista,
        out,
    }))
}

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// The graph of `file`, its index built in `tmp`: an mzML is read as it is, any other dataset a reader takes (a Thermo `.raw`, ...) is first written to
/// `tmp` as mzML with the peaks of the file.
pub fn graph(file: &Path, o: &Options, tmp: &Path) -> Result<(Graph, String)> {
    let name = file
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let converted = tmp.join("0.mzML");
    let is_mzml = name.to_lowercase().ends_with(".mzml");
    if !is_mzml {
        let (peaks, opz) = (Peaks::File, MzmlOpzioni::default());
        crate::flow_base::write_mzml(file, None, &converted, peaks, opz)?;
    }
    let source = if is_mzml { file } else { &converted };
    let idx = tmp.join("0.mzidx");
    crate::build_index(source, &idx, mzlab_core::budget_auto::native_index_budget())?;
    let sha = sha256_hex(&std::fs::read(&idx).map_err(io)?);
    let scans = crate::open(&idx)?.scans;
    let mut m = std::fs::File::open(source).map_err(io)?;
    let o = &mzlab_feat::chem::regole::con_attese(o);
    grafo_json(&scans, &mut m, (&name, &sha), (o, &Grouping::default()))
}

/// The nominal path of a node of the tree (`317@cid35>261@cid35` → [317, 261]).
fn file_path(id: &str) -> Vec<i64> {
    id.split('>')
        .filter_map(|p| p.split('@').next()?.parse::<f64>().ok())
        .map(|m| m.round() as i64)
        .collect()
}

fn hill(f: &str) -> String {
    parse_formula(f.trim_end_matches(['+', '-'])).map_or(f.to_string(), |x| fmt(&x))
}

/// Nodes of each MS level, and the peaks of the spectra of each level: kept (with a node) and discarded (unexplained).
fn per_livello(o: &mut Out, g: &Graph) {
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
    let lv = g.spettri.iter().map(|s| s.level).max().unwrap_or(0);
    let mut first_one = true;
    for l in 2..=lv {
        let pk = g
            .spettri
            .iter()
            .filter(|s| s.level == l)
            .flat_map(|s| &s.picchi);
        let (t, d) = pk.fold((0, 0), |(t, d), p| {
            if p.2.is_some() {
                (t + 1, d)
            } else {
                (t, d + 1)
            }
        });
        if t + d > 0 {
            o.raw(if first_one { "" } else { "," }).str(&l.to_string());
            o.raw(":[").int(t).raw(",").int(d).raw("]");
            first_one = false;
        }
    }
    o.raw("}");
}

/// One line of numbers: the graph against the paths of a truth file.
pub fn summary(g: &Graph, verita: &str) -> Result<String> {
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
    let (mut found, mut between, mut single, mut ppm_max) = (0, 0, 0, 0.0f64);
    for (p, f) in &attesi {
        let Some(n) = g
            .nodi
            .iter()
            .find(|n| n.albero.iter().any(|t| file_path(t) == *p))
        else {
            continue;
        };
        found += 1;
        let Some(x) = &n.formula else { continue };
        if let Some(c) = x.candidates.iter().find(|c| c.formula == *f) {
            between += 1;
            ppm_max = ppm_max.max(c.ppm.abs());
        }
        if x.choice.as_deref() == Some(f) {
            single += 1;
        }
    }
    let mut o = Out::default();
    o.raw("{\"percorsi_verita\":").int(attesi.len() as i64);
    o.raw(",\"percorsi_trovati\":").int(found);
    o.raw(",\"formula_giusta_fra_candidati\":").int(between);
    o.raw(",\"unica_e_giusta\":").int(single);
    o.raw(",\"ppm_max_giuste\":")
        .num(mzlab_core::json::round(ppm_max, 2));
    let annotated = g.nodi.iter().filter(|n| n.formula.is_some()).count();
    o.raw(",\"frammenti_annotati\":")
        .int(annotated.saturating_sub(1) as i64);
    o.raw(",\"scartati_formula\":")
        .int(g.scartati.formula as i64);
    o.raw(",\"avvisi\":").int(g.avvisi.len() as i64).raw(",");
    per_livello(&mut o, g);
    o.raw("}\n");
    Ok(o.0)
}

/// The graph text `j` of `g` with the key `strutture` of node `nodo` (`nK` or `K`): the cleavages of `o.smiles` that give its m/z within `ppm`.
pub fn structures_text(
    g: &Graph,
    j: &str,
    o: &Options,
    (nodo, ppm): (&str, f64),
) -> Result<String> {
    mzlab_feat::chem::strutture::strutture_testo(g, j, o.smiles.as_deref(), (nodo, ppm))
}

/// `msn-graph complete IN.json [--out PATH]`: the graph of the file with the key `vista` added; the text and the file to write it to.
pub fn complete_cmd(
    mut a: impl Iterator<Item = String>,
) -> std::result::Result<(String, Option<PathBuf>), String> {
    let (mut file, mut out) = (None, None);
    while let Some(x) = a.next() {
        match x.as_str() {
            "--out" => out = a.next().map(PathBuf::from),
            f if !f.starts_with("--") && file.is_none() => file = Some(PathBuf::from(f)),
            f => return Err(format!("unknown argument {f}")),
        }
    }
    let file = file.ok_or("complete needs an msngrafo/1 file")?;
    let json = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let done = mzlab_core::msngrafo::completa(&json).map_err(|e| e.to_string())?;
    Ok((done + "\n", out))
}

/// The graph text `j` of `g` with the keys `interpretazione` and `rotture` the options ask for (also used by `msn-lib add`).
pub fn with_keys(a: &Args, g: &Graph, mut j: String) -> Result<String> {
    if let Some(ppm) = a.interpreta {
        j = con_interpretazione(&j, g, ppm);
    }
    if let Some(p) = a.cleavages {
        j = mzlab_feat::chem::rotture::rotture_testo(g, &j, a.o.smiles.as_deref(), p)?;
    }
    Ok(j)
}

/// The text the command prints: the graph, or the line of numbers.
pub fn execute(a: &Args, tmp: &Path) -> Result<String> {
    let (g, j) = graph(&a.file, &a.o, tmp)?;
    let j = with_keys(a, &g, j)?;
    if let Some((f, pulito, nodo)) = &a.export {
        return Ok(esporta(&g, *f, *pulito, nodo.as_deref()));
    }
    if let Some((nodo, ppm)) = &a.structures {
        return Ok(structures_text(&g, &j, &a.o, (nodo, *ppm))? + "\n");
    }
    if !a.summary {
        return Ok(j + "\n");
    }
    let v =
        std::fs::read_to_string(a.verita.as_ref().expect("checked by parse_args")).map_err(io)?;
    summary(&g, &v)
}

/// With `--figura`: writes the figures of the graph text `graph` next to the file of `--out`; the lines for the standard output (the files made).
pub fn figure_files(a: &Args, graph: &str) -> std::result::Result<String, String> {
    let Some(out) = a.out.as_ref().filter(|_| !a.figure.is_empty()) else {
        return Ok(String::new());
    };
    let made = crate::msn_uscite::scrivi(graph, &a.figure, a.view, &out.with_extension(""))
        .map_err(|e| e.to_string())?;
    Ok(made.iter().map(|p| format!("{}\n", p.display())).collect())
}

/// `mzlab-cli msn-graph ...`: the graph (or the line of numbers) to the standard output or `--out`.
pub(crate) fn cmd(args: impl Iterator<Item = String>) -> Outcome {
    let mut args = args.peekable();
    if args.next_if(|a| a == "compare").is_some() {
        return match crate::msn_graph_compare::esegui(args) {
            Ok((t, out)) => crate::emit(t, &out),
            Err(e) => Outcome::fail(&e.to_string()),
        };
    }
    if args.next_if(|a| a == "complete").is_some() {
        return match complete_cmd(args) {
            Ok((t, out)) => crate::emit(t, &out),
            Err(e) => Outcome::fail(&e),
        };
    }
    if args.next_if(|a| a == "figure").is_some() {
        return crate::msn_uscite::figure_cmd(args)
            .map_or_else(|e| Outcome::fail(&e), |t| Outcome::ok(t.into_bytes()));
    }
    let a = match parse_args(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    // one folder per call: the tests run several in one process
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-msn{k}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp)
        .map_err(crate::io)
        .and_then(|_| execute(&a, &tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    let r = r
        .map_err(|e| e.to_string())
        .and_then(|t| figure_files(&a, &t).map(|f| (t, f)));
    match r {
        Err(e) => Outcome::fail(&e),
        Ok((t, files)) => {
            let mut o = crate::emit(t, &a.out);
            o.stdout.extend(files.into_bytes());
            o
        }
    }
}

#[cfg(test)]
#[path = "msn_graph_rotture_tests.rs"]
mod rotture_tests;
