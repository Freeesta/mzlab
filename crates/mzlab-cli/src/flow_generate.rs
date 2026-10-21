//! The native executor of `tp.generate@1`: the registry of the hypotheses (`ipotesi/1`) of a parent, `mzlab_feat::hypo::generate` on the rules of
//! the campaign, the call the page makes for the hypotheses of a series (`run_hypotheses`: no suspect index, no families, no imported list).
//! The parent file is a formula, a SMILES (its groups and cleavage products select the rules) or a JSON `{"name","formula","smiles"}`. A bare line is a formula only when it is written as `elements::fmt` writes it.

use crate::flow::{read_one, write_made, FileRunner};
use mzlab_core::elements;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Artifact;
use mzlab_core::flow::Job;
use mzlab_core::json::{parse, Value};
use mzlab_core::step::DataType;
use mzlab_feat::chem::grafo::analyse_json;
use mzlab_feat::hypo::{generate, HypoParams, Inputs, Rules, Structure};

const RULES: &[u8] = include_bytes!("../../../TP_Mine/data/regole_tp_2.json");

fn param<'a>(job: &'a Job, name: &str) -> Option<&'a Value> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
}

/// The parameters of the step over `HypoParams::default()`.
pub fn hypo_params(job: &Job) -> HypoParams {
    let d = HypoParams::default();
    let num = |n: &str| param(job, n).and_then(Value::f64);
    let flag = |n: &str, dflt: bool| param(job, n).and_then(Value::bool).unwrap_or(dflt);
    HypoParams {
        polarity: num("polarity").map_or(d.polarity, |v| if v < 0.0 { -1 } else { 1 }),
        max_steps: num("max_steps").map_or(d.max_steps, |v| v as u8),
        min_mz: num("min_mz").unwrap_or(d.min_mz),
        max_suspects: num("max_suspects").map_or(d.max_suspects, |v| v as usize),
        other_polarity: flag("other_polarity", d.other_polarity),
        nominal: flag("nominal", d.nominal),
    }
}

/// Formula and `grafo/1` text of the parent file: a SMILES gives both, a formula only itself.
fn parent_of(bytes: &[u8]) -> Result<(String, String)> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim();
    let (formula, smiles) = match parse(text.as_bytes()) {
        Ok(v) if v.get("formula").is_some() || v.get("smiles").is_some() => {
            let s = |k: &str| {
                v.get(k)
                    .and_then(Value::str)
                    .unwrap_or("")
                    .trim()
                    .to_string()
            };
            (s("formula"), s("smiles"))
        }
        _ => {
            let line = text.lines().next().unwrap_or("").trim().to_string();
            // a formula is written as the engine writes it ("C13H24N4O3S"); "CCO" and "CC(C)O" are SMILES
            let is_formula =
                elements::parse_formula(&line).is_ok_and(|f| elements::fmt(&f) == line);
            if is_formula {
                (line, String::new())
            } else {
                (String::new(), line)
            }
        }
    };
    if smiles.is_empty() {
        if formula.is_empty() {
            return Err(Error::new("error.hypo.noParent"));
        }
        return Ok((formula, String::new()));
    }
    let structure = analyse_json(&smiles)?;
    let own = Structure::parse(&structure)?.formula.unwrap_or_default();
    Ok((if formula.is_empty() { own } else { formula }, structure))
}

/// `ipotesi/1` text of the parent file under the parameters of the job.
pub fn generate_text(job: &Job, parent: &[u8]) -> Result<String> {
    let (formula, structure) = parent_of(parent)?;
    let st = Structure::parse(&structure)?;
    st.check_parent(&formula)?;
    let inp = Inputs {
        parent: &formula,
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &st.fragments,
        groups: st.groups.as_deref(),
    };
    let rules = Rules::parse_profile(RULES, st.profile.as_deref())?;
    Ok(generate(&rules, &inp, &hypo_params(job))?.to_json())
}

pub(crate) fn generate_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let parent = read_one(r, job, "parent")?;
    let text = generate_text(job, &parent.1)?;
    write_made(r, job, text, DataType::Transform)
}
