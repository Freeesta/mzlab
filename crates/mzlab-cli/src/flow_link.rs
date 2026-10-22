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
use mzlab_feat::hypo::{generate, HypoParams, Inputs, Rules, Structure};
use mzlab_feat::interp::ms2link::LinkParams;
use mzlab_feat::interp::run::run_to_with;
use mzlab_feat::legacy::lr::rows::OFFSET_MAX;
use mzlab_feat::legacy::series;
use mzlab_feat::store::{read_table_from, Slice};
use mzlab_feat::tp::TpRoles;
use mzlab_feat_wasm::run::{self, RowSources};

use crate::flow::{all_inputs, io, read_one, FileRunner, Lineage};
use crate::flow_base::{open, rebuild, Files};
use mzlab_core::flow::Artifact;
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
    let d = LinkParams::default();
    LinkParams {
        select_ppm: param(job, "select_ppm").unwrap_or(d.select_ppm),
        purity_ppm: param(job, "purity_ppm").unwrap_or(d.purity_ppm),
        purity_min: param(job, "purity_min").unwrap_or(d.purity_min),
        match_ppm: param(job, "match_ppm").unwrap_or(d.match_ppm),
        iso_corr: param(job, "iso_corr").unwrap_or(d.iso_corr),
        iso_min_scans: param(job, "iso_min_scans").map_or(d.iso_min_scans, |v| v as usize),
        iso_fragments: param(job, "iso_fragments").map_or(d.iso_fragments, |v| v as usize),
    }
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

/// The parent as the interpretation reads it (`series::parent`): the SMILES or the neutral formula, the adduct of the polarity and the time.
fn parent_json(p: &Parent, polarity: i8, rt: f64) -> String {
    let mut o = mzlab_core::json::Out::default();
    o.raw("{");
    if p.smiles.is_empty() {
        o.raw("\"neutral\":").str(&p.formula);
    } else {
        o.raw("\"smiles\":").str(&p.smiles);
    }
    o.raw(",\"adduct\":")
        .str(if polarity < 0 { "[M-H]-" } else { "[M+H]+" });
    o.raw(",\"rt\":").num(rt).raw("}");
    o.0
}

/// The retention time of the parent when the file does not say it: the strongest row of the table within 5 ppm of its ion (on a unit-resolution
/// table, within the offset the low-resolution Dig accepts for the parent, `OFFSET_MAX` Da).
fn parent_rt(table: &[u8], mz: f64) -> Result<f64> {
    let (t, meta) = read_table_from(&mut Slice(table), table.len() as u64)?;
    let unit = meta
        .get("recipe")
        .and_then(Value::str)
        .is_some_and(mzlab_feat::recipe::is_unit_text);
    let tol = if unit { OFFSET_MAX } else { mz * 5e-6 };
    let near = (0..t.len()).filter(|&r| (t.mz[r] - mz).abs() <= tol);
    let total = |r: usize| {
        t.area[r * t.n_files..(r + 1) * t.n_files]
            .iter()
            .sum::<f64>()
    };
    let best = near.max_by(|&a, &b| total(a).total_cmp(&total(b)));
    best.map(|r| t.rt[r])
        .ok_or_else(|| Error::new("err.flow.link.parent").with("mz", mz))
}

/// The hypotheses of `tp.generate` for the parent, before the search: what the input `hypotheses` has to be.
fn expected_hypotheses(p: &Parent, polarity: i8) -> Result<String> {
    let st = Structure::parse(&p.structure)?;
    let inp = Inputs {
        parent: &p.formula,
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &st.fragments,
        groups: st.groups.as_deref(),
    };
    let rules = Rules::parse_profile(RULES, st.profile.as_deref())?;
    let hp = HypoParams {
        polarity,
        ..HypoParams::default()
    };
    Ok(generate(&rules, &inp, &hp)?.to_json())
}

/// The polarity of the link: the one the plan sets, else the one of the detection (a detection that took both polarities, 0, or none is read as
/// positive, like a form left at its default).
fn polarity_of(job: &Job, recipe: &mzlab_feat::recipe::Recipe) -> i8 {
    let given = job.instance.params.iter().any(|(k, _)| k == "polarity");
    match param(job, "polarity") {
        Some(v) if given => v.signum() as i8,
        _ => recipe.detect.polarity.filter(|&p| p != 0).unwrap_or(1),
    }
}

/// The m/z of the ion of the parent in the polarity.
fn parent_mz(p: &Parent, polarity: i8) -> Result<f64> {
    let json = parent_json(p, polarity, 0.0);
    let parent = series::parent(&json);
    Ok(parent
        .map_err(|e| Error::new("error.interp.failed").with("detail", e))?
        .mz)
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
    let polarity = polarity_of(job, &recipe);
    recipe.detect.polarity = Some(polarity);
    let p = parent_of(parent)?;
    if String::from_utf8_lossy(hypotheses).trim() != expected_hypotheses(&p, polarity)? {
        return Err(Error::new("err.flow.link.hypotheses").with("parent", &p.formula));
    }
    let roles = TpRoles::from_json(&String::from_utf8_lossy(samples))?;
    let mz = parent_mz(&p, polarity)?;
    let rt = p.rt.map_or_else(|| parent_rt(table, mz), Ok)?;
    let mut ix = series_of
        .indexes
        .iter()
        .map(open)
        .collect::<Result<Vec<_>>>()?;
    let clock = Instant::now();
    let mut stored = Files(lin.peaks.clone());
    let hypo = run::hypotheses_stored(
        &mut ix,
        &mut stored,
        RULES,
        &p.structure,
        &p.formula,
        p.rt,
        &recipe,
    )?;
    let t_hypo = clock.elapsed();
    let budget = MemBudget::mb(r.budget_mb);
    let mut with_tp = Vec::new();
    let mut again = Files(lin.peaks.clone());
    let mut reopen = |k: usize| open(&series_of.indexes[k]);
    run::evidence_sources_to(
        &mut Slice(table),
        table.len() as u64,
        &hypo,
        &roles.minutes,
        Some(&roles),
        Some((mz, rt)),
        &recipe,
        &budget,
        Some(RowSources {
            stored: &mut again,
            open: &mut reopen,
        }),
        &mut with_tp,
    )?;
    let t_evidence = clock.elapsed();
    let mut infusion = roles
        .infusion
        .first()
        .map(|&k| open(&series_of.indexes[k]))
        .transpose()?;
    let mut out = Vec::new();
    run_to_with(
        &mut Slice(&with_tp),
        with_tp.len() as u64,
        (
            &parent_json(&p, polarity, rt),
            &hypo.registry.to_json(),
            polarity,
        ),
        &link_params(job),
        &mut reopen,
        infusion.as_mut(),
        &mut out,
    )?;
    report_phases(t_hypo, t_evidence, clock.elapsed());
    Ok(out)
}

/// `tp.link`: the `.mzfeat` of the series with `tp/1` and `interp/1`, written next to the other outputs of the run.
pub(crate) fn link_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
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
    let bytes = link_bytes(r, job, &series, files)?;
    let out = r.work.join(format!("{}.mzfeat", job.instance.id));
    std::fs::write(&out, bytes).map_err(io)?;
    r.keep(out, DataType::Tp)
}
