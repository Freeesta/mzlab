//! The native executor of `spectra.msn_graph@1`: the fragmentation graph `msngrafo/1` of an MSn infusion (mzML or a vendor file such as a Thermo `.raw`),
//! the same bytes as `mzlab-cli msn-graph FILE`. The step takes the options of `msngrafo::Opzioni` that the command has, plus the name of the molecule
//! looked up in a molecules list the user gives; its output is read by the page and by `msn-lib`.

use std::path::Path;

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::molecules::{cerca, from_csv};
use mzlab_core::msngrafo::{Opzioni, ParamRegole};
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use crate::flow::{one_input, write_made, FileRunner};

/// `spectra.msn_graph@1`: one graph per infusion file.
pub fn msn_graph_step() -> Step {
    Step {
        ins: vec![
            Port::new("source", DataType::Raw, "Vendor file or mzML of an MSn infusion").accepts(&[DataType::Mzml]),
            Port {
                optional: true,
                ..Port::new("molecules", DataType::Structure, "Molecules list (CSV) for the name of the molecule")
                    .accepts(&[DataType::Table, DataType::Report])
            },
        ],
        outs: vec![Port::new("graph", DataType::Report, "Fragmentation graph (msngrafo/1)")],
        params: params(),
        runs_in: RunsIn {
            wasm: Some("wasip1 (mzlab-cli)"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.file.numpress"],
        ..Step::core(
            "spectra.msn_graph",
            "MSn fragmentation graph",
            "Merges the MSn spectra of an infusion into one graph of ions and neutral losses, with the sub-formulas of the fragments when the ion of the root is given. It proposes candidates; it does not identify.",
        )
    }
}

/// The parameters of the step: one per option of the command.
/// The parameters of the bond breaks (`rotture`).
fn rotture_params() -> Vec<Param> {
    vec![
            Param::boolean("rotture", "Break the bonds of the structure")
                .default_bool(false)
                .doc("Adds the key `rotture`: the sets of atoms of the structure (one or two bonds cut, a few hydrogens moved) that give the m/z of each node, the arcs they explain and the share of each spectrum's intensity explained (needs the structure; window `ppm_struttura`)"),
            Param::integer("h_max", "Hydrogens moved")
                .min(0.0)
                .max(4.0)
                .default_num(2.0)
                .doc("Hydrogens moved at most beyond the plain cleavage, for the bond breaks"),
            Param::integer("max_legami", "Bonds cut")
                .min(1.0)
                .max(2.0)
                .default_num(2.0)
                .doc("Bonds cut at most for one piece, 1 or 2"),
    ]
}

fn params() -> Vec<Param> {
    let d = Opzioni::default();
    let mut v = vec![
            Param::text("formula", "Ion of the root")
                .default_str("")
                .doc("C13H25N4O3S+, C13H24N4O3S+H or \"C13H24N4O3S [M+H]+\"; empty = masses only, no sub-formulas"),
            Param::text("molecule", "Name of the molecule")
                .default_str("")
                .doc("Name, synonym, id or formula looked up in the molecules list of the port `molecules`; it gives the formula (and the SMILES) of the root with `adduct`. `formula` wins when both are given"),
            Param::text("adduct", "Adduct of the root")
                .default_str("[M+H]+")
                .doc("Adduct written after the formula of `molecule`, e.g. [M+H]+ or [M-H]-"),
            Param::text("smiles", "Structure (SMILES)")
                .default_str("")
                .doc("Written back in the graph; it does not change the numbers"),
            Param::text("molfile", "Structure (molfile)")
                .default_str("")
                .doc("The 2D structure as a molfile, written back in the graph for the drawings; it does not change the numbers"),
            Param::number("ppm_formula", "Sub-formula window")
                .min(0.0)
                .default_num(d.ppm_formula)
                .unit("ppm")
                .doc("A sub-formula is a candidate of a fragment within this distance of its m/z"),
            Param::number("mz_basso", "Low m/z limit")
                .min(0.0)
                .default_num(d.mz_basso)
                .unit("m/z")
                .doc("Below this m/z the window may widen to the low-mass window"),
            Param::number("ppm_basso", "Low-mass window")
                .min(0.0)
                .default_num(d.ppm_basso)
                .unit("ppm")
                .doc("The wider window, for a candidate with enough evidences"),
            Param::integer("prove_min", "Evidences for the wider window")
                .min(0.0)
                .default_num(d.prove_min as f64),
            Param::text("attivazione", "Activation")
                .default_str("")
                .doc("Only the spectra of this activation (cid, hcd, ...); empty = all"),
            Param::boolean("isotopi", "Keep M+1 peaks")
                .default_bool(d.isotopi)
                .doc("The M+1 peaks of the fragments become nodes with their monoisotopic peak"),
            Param::text("strutture_nodo", "Node for the structure cleavages")
                .default_str("")
                .doc("Node nK of the graph: adds the key `strutture`, the bonds of the structure whose cleavage gives its m/z (needs the structure); `tutti` = the key `strutture_tutti` for every node of level 2 or more; empty = none"),
            Param::number("ppm_struttura", "Cleavage window")
                .min(0.0)
                .default_num(crate::msn_graph::PPM_STRUTTURA)
                .unit("ppm")
                .doc("Distance of a cleaved piece from the m/z of the node"),
            Param::boolean("interpreta", "Interpret the losses")
                .default_bool(false)
                .doc("Adds the key `interpretazione`: for each arc the neutral loss (name, formula, exact mass, ppm, reason, confidence) and the activations that saw it"),
            Param::number("ppm_perdita", "Loss window")
                .min(0.0)
                .default_num(mzlab_core::msngrafo::PPM_PERDITA)
                .unit("ppm")
                .doc("Window of the loss match by mass, ppm of the precursor m/z"),
            Param::boolean("regole", "Apply the fragmentation rules")
                .default_bool(false)
                .doc("Adds the key `regole`: the rules of the table `fragrules/1` that the formulas alone decide (which rule explains the loss of an arc, even-electron check, trap cut-off, composite and stepped losses)"),
            Param::number("cutoff_q", "Trap cut-off q")
                .min(0.0)
                .default_num(ParamRegole::default().cutoff_q)
                .doc("Fragments below q/0.908 of the precursor m/z are not observable in a trap CID"),
            Param::number("toll_perdita_mda", "Rule tolerance")
                .min(0.0)
                .default_num(ParamRegole::default().toll_mda)
                .unit("mDa")
                .doc("Tolerance of a loss against the mass of a rule"),
            Param::text("scelte", "Chosen formulas")
                .default_str("")
                .doc("MZ=ION pairs separated by commas: the formula chosen for the node at that m/z"),
        ];
    v.extend(rotture_params());
    v
}

fn text(job: &Job, name: &str) -> Option<String> {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found
        .and_then(|(_, v)| v.str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn number(job: &Job, name: &str, fallback: f64) -> f64 {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64()).unwrap_or(fallback)
}

/// The molecule `name` of the CSV `list`: the one whose name, synonym or formula is the best match; two best matches are ambiguous and a name the list does
/// not hold is an error. Returns its ion text (`C8H9NO2 [M+H]+`) and SMILES.
fn molecule(name: &str, adduct: &str, list: &str) -> Result<(String, Option<String>)> {
    let bad = |detail: String| {
        Error::new("err.flow.param.type")
            .with("param", "molecule")
            .with("detail", detail)
    };
    let ms = from_csv(list, "importata", None).map_err(|e| {
        bad(format!(
            "molecules list: row {}: {}",
            e[0].line, e[0].detail
        ))
    })?;
    let by_id = ms.iter().position(|m| m.id == name).map(|k| (k, 0.0));
    let hits = by_id.map_or_else(|| cerca(&ms, name, 8, 5.0), |h| vec![h]);
    let Some(&(k, best)) = hits.first() else {
        return Err(bad(format!("{name} is not in the molecules list")));
    };
    // 0 = the same name or formula; a worse match shared by several molecules is a guess, not a choice
    if best > 0.0 && hits.iter().filter(|h| h.1 == best).count() > 1 {
        return Err(bad(format!(
            "{name} matches several molecules of the list: give the exact name or its id"
        )));
    }
    let m = &ms[k];
    Ok((format!("{} {adduct}", m.formula), m.smiles.clone()))
}

/// The options of the command from the resolved parameters; `molecules` is the text of the list the port gives, if any.
fn opzioni(job: &Job, molecules: Option<&str>) -> Result<Opzioni> {
    let mut o = Opzioni {
        formula: text(job, "formula"),
        smiles: text(job, "smiles"),
        molfile: text(job, "molfile"),
        attivazione: text(job, "attivazione"),
        ..Opzioni::default()
    };
    if let (Some(name), None) = (text(job, "molecule"), &o.formula) {
        let list = molecules
            .ok_or_else(|| Error::new("err.flow.port.missing").with("port", "molecules"))?;
        let adduct = text(job, "adduct").unwrap_or_else(|| "[M+H]+".into());
        let (ion, smiles) = molecule(&name, &adduct, list)?;
        o.formula = Some(ion);
        o.smiles = o.smiles.or(smiles);
    }
    o.ppm_formula = number(job, "ppm_formula", o.ppm_formula);
    o.mz_basso = number(job, "mz_basso", o.mz_basso);
    o.ppm_basso = number(job, "ppm_basso", o.ppm_basso);
    o.prove_min = number(job, "prove_min", o.prove_min as f64) as usize;
    o.isotopi = job
        .resolved
        .params
        .iter()
        .any(|(k, v)| k == "isotopi" && v.bool() == Some(true));
    if job
        .resolved
        .params
        .iter()
        .any(|(k, v)| k == "regole" && v.bool() == Some(true))
    {
        let d = ParamRegole::default();
        o.regole = Some(ParamRegole {
            cutoff_q: number(job, "cutoff_q", d.cutoff_q),
            toll_mda: number(job, "toll_perdita_mda", d.toll_mda),
        });
    }
    for par in text(job, "scelte").iter().flat_map(|s| s.split(',')) {
        let bad = || Error::new("err.flow.param.type").with("param", "scelte");
        let (m, f) = par.split_once('=').ok_or_else(bad)?;
        o.scelte
            .push((m.trim().parse().map_err(|_| bad())?, f.trim().to_string()));
    }
    Ok(o)
}

/// The text of the graph of the dataset at `file`, its index built in `tmp`: what the command prints.
pub fn msn_graph_text(
    job: &Job,
    file: &Path,
    molecules: Option<&str>,
    tmp: &Path,
) -> Result<String> {
    let o = opzioni(job, molecules)?;
    let (g, mut json) = crate::msn_graph::grafo(file, &o, tmp)?;
    if job
        .resolved
        .params
        .iter()
        .any(|(k, v)| k == "interpreta" && v.bool() == Some(true))
    {
        let ppm = number(job, "ppm_perdita", mzlab_core::msngrafo::PPM_PERDITA);
        json = mzlab_core::msngrafo::con_interpretazione(&json, &g, ppm);
    }
    if job
        .resolved
        .params
        .iter()
        .any(|(k, v)| k == "rotture" && v.bool() == Some(true))
    {
        let p = mzlab_feat::chem::rotture::Parametri {
            ppm: number(job, "ppm_struttura", crate::msn_graph::PPM_STRUTTURA),
            h_max: number(job, "h_max", 2.0) as u8,
            max_legami: number(job, "max_legami", 2.0) as u8,
        };
        json = mzlab_feat::chem::rotture::rotture_testo(&g, &json, o.smiles.as_deref(), p)?;
    }
    match text(job, "strutture_nodo") {
        Some(nodo) => {
            let ppm = number(job, "ppm_struttura", crate::msn_graph::PPM_STRUTTURA);
            Ok(crate::msn_graph::strutture_testo(&g, &json, &o, (&nodo, ppm))? + "\n")
        }
        None => Ok(json + "\n"),
    }
}

/// The executor of the step: one graph per infusion file.
pub(crate) fn job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let file = one_input(r, job, "source")?;
    let list = match one_input(r, job, "molecules") {
        Ok(p) => Some(std::fs::read_to_string(p).map_err(crate::flow::io)?),
        Err(_) => None,
    };
    let tmp = r.work.join(format!("{}-msn", job.instance.id));
    std::fs::create_dir_all(&tmp).map_err(crate::flow::io)?;
    let text = msn_graph_text(job, &file, list.as_deref(), &tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    write_made(r, job, text?, DataType::Report)
}
