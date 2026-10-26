//! The native executor of `tp.link@2`: the chain the page runs for a parent in a series, on files. The hypotheses are searched in the series
//! (`run::hypotheses_stored`), the proofs and the view of the products (`tp/1`) are built on the table (`run::evidence_sources_to`) and each row is
//! interpreted (`interp::run::run_to_with`: modification, linked MS2, comparison with the parent). The `.mzfeat` that comes out holds the blocks
//! `tp/1` and `interp/1` that `tp.network` reads. The table has to be one this plan made: its lineage gives the `.mzfeatf` of the files and the recipe.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use mzlab_core::budget::MemBudget;
use mzlab_core::elements;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::json::{parse, Value};
use mzlab_feat::chem::grafo::analyse_json;
use mzlab_feat::hypo::Structure;
use mzlab_feat::interp::ms2link::LinkParams;
use mzlab_feat::interp::run::run_to_with;
use mzlab_feat::store::Slice;
use mzlab_feat::tp::TpRoles;
use mzlab_feat_wasm::run_link::{link_evidence, LinkParent, Series as LinkSeries};

use crate::flow::{all_inputs, io, read_one, FileRunner, Lineage, Made};
use crate::flow_base::{open, rebuild, Files};
use mzlab_core::step::DataType;

const RULES: &[u8] = include_bytes!("../../../TP_Mine/data/regole_tp_2.json");

/// The parent of the file: neutral formula, SMILES when there is one, and the retention time when the file says it.
pub struct Parent {
    pub formula: String,
    pub smiles: String,
    pub structure: String,
    pub rt: Option<f64>,
}

fn param(job: &Job, name: &str) -> Option<f64> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| v.f64())
}

/// The thresholds of the step over `LinkParams::default()`.
pub fn link_params(job: &Job) -> LinkParams {
    mzlab_feat_wasm::interp::link_params(&|n| param(job, n))
}

/// The parent file: a JSON `{"formula"|"neutral", "smiles", "rt"}`, or one line with a formula (written as `elements::fmt` writes it) or a SMILES.
pub fn parent_of(bytes: &[u8]) -> Result<Parent> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim();
    let json = parse(text.as_bytes()).ok().filter(|v| {
        v.get("rt").is_some()
            || v.get("formula").is_some()
            || v.get("neutral").is_some()
            || v.get("smiles").is_some()
    });
    let (mut formula, smiles, rt) = match &json {
        Some(v) => {
            let s = |k: &str| {
                v.get(k)
                    .and_then(Value::str)
                    .unwrap_or("")
                    .trim()
                    .to_string()
            };
            let f = [s("formula"), s("neutral")]
                .into_iter()
                .find(|x| !x.is_empty());
            (
                f.unwrap_or_default(),
                s("smiles"),
                v.get("rt").and_then(Value::f64).filter(|t| t.is_finite()),
            )
        }
        None => {
            let line = text.lines().next().unwrap_or("").trim().to_string();
            let is_formula =
                elements::parse_formula(&line).is_ok_and(|f| elements::fmt(&f) == line);
            if is_formula {
                (line, String::new(), None)
            } else {
                (String::new(), line, None)
            }
        }
    };
    let structure = if smiles.is_empty() {
        String::new()
    } else {
        analyse_json(&smiles)?
    };
    if formula.is_empty() {
        formula = Structure::parse(&structure)?.formula.unwrap_or_default();
    }
    if formula.is_empty() {
        return Err(Error::new("error.hypo.noParent"));
    }
    Ok(Parent {
        formula,
        smiles,
        structure,
        rt,
    })
}

/// The plan sets the polarity of the link only when the instance writes it; else the detection's polarity decides.
fn given_polarity(job: &Job) -> Option<f64> {
    let given = job.instance.params.iter().any(|(k, _)| k == "polarity");
    param(job, "polarity").filter(|_| given)
}

/// What `link` needs of the job, read from the files.
pub struct Series {
    pub lineage: Lineage,
    pub indexes: Vec<PathBuf>,
}

fn check_series(series_of: &Series) -> Result<()> {
    let lin = &series_of.lineage;
    if lin.peaks.is_empty() || lin.peaks.len() != series_of.indexes.len() {
        return Err(Error::new("err.flow.link.series")
            .with("peaks", lin.peaks.len())
            .with("index", series_of.indexes.len()));
    }
    Ok(())
}

/// Seconds of each phase of `link_bytes` on stderr when `MZLAB_PHASES` is set (for profiling real runs).
fn report_phases(hypo: Duration, evidence: Duration, total: Duration) {
    if std::env::var_os("MZLAB_PHASES").is_some() {
        eprintln!(
            "tp.link phases: hypotheses {:.1} s, evidence {:.1} s, interpretation {:.1} s",
            hypo.as_secs_f64(),
            (evidence - hypo).as_secs_f64(),
            (total - evidence).as_secs_f64()
        );
    }
}

/// The `.mzfeat` with `tp/1` and `interp/1` of the table `table` of the series `series` (the files of `lin`), for the parent file `parent` and the
/// `campioni/2` `samples`; `hypotheses` is the `ipotesi/1` of `tp.generate`, which has to be the one of this parent.
pub fn link_bytes(
    r: &FileRunner,
    job: &Job,
    series_of: &Series,
    files: [&[u8]; 4],
) -> Result<Vec<u8>> {
    let [table, samples, parent, hypotheses] = files;
    let lin = &series_of.lineage;
    check_series(series_of)?;
    let mut recipe = rebuild(lin)?;
    let p = parent_of(parent)?;
    let roles = TpRoles::from_json(&String::from_utf8_lossy(samples))?;
    let mut ix = series_of
        .indexes
        .iter()
        .map(open)
        .collect::<Result<Vec<_>>>()?;
    let clock = Instant::now();
    let (mut stored, mut again) = (Files(lin.peaks.clone()), Files(lin.peaks.clone()));
    let mut reopen = |k: usize| open(&series_of.indexes[k]);
    let mut laps = Vec::new();
    let made = link_evidence(
        RULES,
        &LinkParent {
            formula: &p.formula,
            smiles: &p.smiles,
            structure: &p.structure,
            rt: p.rt,
        },
        &String::from_utf8_lossy(hypotheses),
        table,
        &roles,
        &mut recipe,
        given_polarity(job),
        LinkSeries {
            indexes: &mut ix,
            peaks: &mut stored,
            peaks_again: &mut again,
            open: &mut reopen,
        },
        &MemBudget::mb(r.budget_mb),
        &mut |_| laps.push(clock.elapsed()),
    )?;
    let mut infusion = made
        .infusion
        .map(|k| open(&series_of.indexes[k]))
        .transpose()?;
    let mut out = Vec::new();
    run_to_with(
        &mut Slice(&made.with_tp),
        made.with_tp.len() as u64,
        (&made.parent, &made.registry, made.polarity),
        &link_params(job),
        &mut reopen,
        infusion.as_mut(),
        &mut out,
    )?;
    if let [hypo, evidence] = laps[..] {
        report_phases(hypo, evidence, clock.elapsed());
    }
    Ok(out)
}

/// `tp.link`: the `.mzfeat` with `tp/1` and `interp/1` of the series; no t0 among the samples adds `warn.flow.link.noT0` to the run.
pub(crate) fn link_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let table_in = job
        .inputs
        .iter()
        .find(|(p, _)| p == "table")
        .and_then(|(_, v)| v.first());
    let lineage = table_in
        .and_then(|a| r.lineage(a))
        .filter(|l| !l.peaks.is_empty())
        .ok_or_else(|| Error::new("err.flow.lineage").with("step", job.step.id))?;
    let indexes = all_inputs(r, job, "index");
    let (table, samples) = (read_one(r, job, "table")?, read_one(r, job, "samples")?);
    let (parent, hypotheses) = (read_one(r, job, "parent")?, read_one(r, job, "hypotheses")?);
    let series = Series { lineage, indexes };
    let files = [
        &table.1[..],
        &samples.1[..],
        &parent.1[..],
        &hypotheses.1[..],
    ];
    let roles = TpRoles::from_json(&String::from_utf8_lossy(files[1]))?;
    let bytes = link_bytes(r, job, &series, files)?;
    let out = r.work.join(format!("{}.mzfeat", job.instance.id));
    std::fs::write(&out, bytes).map_err(io)?;
    let mut made = Made::new("tp", r.keep(out, DataType::Tp)?);
    if no_t0(&roles) {
        made.warnings.push(WARN_NO_T0.into());
    }
    Ok(made)
}

/// The key the run records when the treated files have no t0 to compare with.
pub(crate) const WARN_NO_T0: &str = "warn.flow.link.noT0";

/// Treated files and no t0: the gate of the formation then runs on blank and dark only.
pub(crate) fn no_t0(roles: &TpRoles) -> bool {
    !roles.treated.is_empty() && roles.t0.is_empty()
}
