//! The native executor of `spectra.msn_graph@1`: the fragmentation graph `msngrafo/1` of an MSn infusion (mzML), the same bytes as
//! `mzlab-cli msn-graph FILE`. The step takes the options of `msngrafo::Opzioni` that the command has; its output is read by the page and by `msn-lib`.

use std::path::Path;

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::msngrafo::Opzioni;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

/// `spectra.msn_graph@1`: one graph per infusion file.
pub fn msn_graph_step() -> Step {
    let d = Opzioni::default();
    Step {
        ins: vec![Port::new("source", DataType::Mzml, "mzML of an MSn infusion")],
        outs: vec![Port::new("graph", DataType::Report, "Fragmentation graph (msngrafo/1)")],
        params: vec![
            Param::text("formula", "Ion of the root")
                .default_str("")
                .doc("C13H25N4O3S+, C13H24N4O3S+H or \"C13H24N4O3S [M+H]+\"; empty = masses only, no sub-formulas"),
            Param::text("smiles", "Structure (SMILES)")
                .default_str("")
                .doc("Written back in the graph; it does not change the numbers"),
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
            Param::text("scelte", "Chosen formulas")
                .default_str("")
                .doc("MZ=ION pairs separated by commas: the formula chosen for the node at that m/z"),
        ],
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

/// The options of the command from the resolved parameters.
fn opzioni(job: &Job) -> Result<Opzioni> {
    let mut o = Opzioni {
        formula: text(job, "formula"),
        smiles: text(job, "smiles"),
        attivazione: text(job, "attivazione"),
        ..Opzioni::default()
    };
    o.ppm_formula = number(job, "ppm_formula", o.ppm_formula);
    o.mz_basso = number(job, "mz_basso", o.mz_basso);
    o.ppm_basso = number(job, "ppm_basso", o.ppm_basso);
    o.prove_min = number(job, "prove_min", o.prove_min as f64) as usize;
    o.isotopi = job
        .resolved
        .params
        .iter()
        .any(|(k, v)| k == "isotopi" && v.bool() == Some(true));
    for par in text(job, "scelte").iter().flat_map(|s| s.split(',')) {
        let bad = || Error::new("err.flow.param.type").with("param", "scelte");
        let (m, f) = par.split_once('=').ok_or_else(bad)?;
        o.scelte
            .push((m.trim().parse().map_err(|_| bad())?, f.trim().to_string()));
    }
    Ok(o)
}

/// The text of the graph of the mzML at `file`, its index built in `tmp`: what the command prints.
pub fn msn_graph_text(job: &Job, file: &Path, tmp: &Path) -> Result<String> {
    let (_, json) = crate::msn_graph::grafo(file, &opzioni(job)?, tmp)?;
    Ok(json + "\n")
}
