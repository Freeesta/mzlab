//! The native executor of `tp.generate@1`: the registry of the hypotheses (`ipotesi/1`) of a parent, `mzlab_feat::hypo::generate` on the rules of
//! the campaign, the call the page makes for the hypotheses of a series (`run_hypotheses`: no suspect index, no families, no imported list).
//! The parent file is a formula, a SMILES (its groups and cleavage products select the rules) or a JSON `{"name","formula","smiles"}`. A bare line is a formula only when it is written as `elements::fmt` writes it.

use crate::flow::{all_inputs, read_one, write_made, FileRunner, Made};
use crate::flow_nature::{estimates, of_files, written, Nature};
use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::json::Value;
use mzlab_core::step::DataType;
use mzlab_feat::chem::grafo::analyse_json;
use mzlab_feat::hypo::HypoParams;
use mzlab_feat_wasm::run_tp;

const RULES: &[u8] = include_bytes!("../../../TP_Mine/data/regole_tp_2.json");

fn param<'a>(job: &'a Job, name: &str) -> Option<&'a Value> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
}

/// The parameters of the step over `HypoParams::default()`; `nature` (the series connected to `index`) fills the polarity and the unit resolution the plan leaves out.
pub fn hypo_params(job: &Job, nature: &Nature) -> HypoParams {
    let mut p = run_tp::hypo_params(&|n| param(job, n).cloned());
    if !written(job, "polarity") {
        p.polarity = nature.polarity.unwrap_or(p.polarity);
    }
    if !written(job, "nominal") {
        p.nominal = p.nominal || nature.unit;
    }
    p
}

/// `ipotesi/1` text of the parent file under the parameters of the job.
pub fn generate_text(job: &Job, parent: &[u8], nature: &Nature) -> Result<String> {
    let p = run_tp::read_parent(parent);
    let structure = if p.smiles.is_empty() {
        String::new()
    } else {
        analyse_json(&p.smiles)?
    };
    let formula = run_tp::parent_formula(&p, &structure)?;
    run_tp::generate_text(RULES, &formula, &structure, &hypo_params(job, nature))
}

pub(crate) fn generate_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let parent = read_one(r, job, "parent")?;
    let nature = of_files(&all_inputs(r, job, "index"));
    let text = generate_text(job, &parent.1, &nature)?;
    Ok(Made {
        estimates: estimates(job, &nature),
        ..Made::new("hypotheses", write_made(r, job, text, DataType::Transform)?)
    })
}
