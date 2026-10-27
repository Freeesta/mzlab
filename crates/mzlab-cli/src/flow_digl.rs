//! The native executor of `tp.dig_lr@1`: the low-resolution Dig of mzFinder (`legacy::lr::dig`: calibration on the parent, candidates of the rules of
//! `regole_tp_2.json`, criteria, score and labels) on a unit-resolution series made by this plan. Its output is the JSON `api.run` of the page.

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::json::Out;
use mzlab_core::step::{DataType, Facets, Param, Port, RunsIn, Step};
use mzlab_feat::legacy::lr::{self, plan::Settings, summary::summary};
use mzlab_feat_wasm::run;

use crate::flow::{all_inputs, read_one, write_made, FileRunner};
use crate::flow_base::{open, rebuild, Files};
use crate::flow_link::parent_of;
use mzlab_core::flow::Artifact;

// `lib.rs` is at its line limit, so the module of `chrom.uv_peaks` hangs here, where the steps of the crate are listed.
#[path = "flow_uv.rs"]
pub(crate) mod uv;

const RULES: &[u8] = include_bytes!("../../../TP_Mine/data/regole_tp_2.json");

/// The steps of this crate besides `features.table`, in the order of the registry.
pub fn cli_steps() -> Vec<Step> {
    vec![
        crate::flow_ms2::ms2_step(),
        crate::flow_msngraph::msn_graph_step(),
        crate::flow_msnlib::library_step(),
        crate::flow_msnlib::table_step(),
        crate::flow_msnfigura::figure_step(),
        crate::flow_compare::compare_step(),
        mzlab_feat::steps::table_match(),
        crate::flow_lookup::lookup_step(),
        dig_lr_step(),
        uv::uv_step(),
    ]
}

/// `tp.dig_lr@1`: the candidates of the transformation products of a parent in a unit-resolution series.
pub fn dig_lr_step() -> Step {
    let d = Settings::default();
    let lr = Facets {
        resolution: vec!["lr"],
        ..Facets::default()
    };
    Step {
        ins: vec![
            Port::new("table", DataType::Table, "Aligned feature table of the series, made by this plan").requires(lr.clone()),
            Port::new("samples", DataType::Samples, "Roles and times of the files"),
            Port::new("parent", DataType::Structure, "Parent compound: formula or SMILES"),
            Port::new("index", DataType::Mzidx, "Scan index of each file of the series, in the order of the table").many().requires(lr),
        ],
        outs: vec![Port::new("dig", DataType::Report, "Candidates with criteria, score and label (the JSON of the Dig)")],
        params: vec![
            Param::number("tol_da", "m/z tolerance").min(0.0).default_num(d.tol_da).unit("Da"),
            Param::number("rt_tol_min", "RT tolerance").min(0.0).default_num(d.rt_tol_min).unit("min"),
            Param::number("max_steps", "Steps of the rules").min(1.0).default_num(d.max_steps),
            Param::number("rt_min", "End of the dead volume").min(0.0).default_num(d.rt_min).unit("min"),
            Param::boolean("discover", "Look for unexpected ions").default_bool(d.discover),
            Param::integer("max_unexpected", "Unexpected ions kept").min(0.0).default_num(d.max_unexpected as f64),
            Param::boolean("auto_offset", "Calibrate on the parent").default_bool(d.auto_offset),
            Param::integer("min_files", "Min. samples").min(0.0).default_num(d.min_files as f64),
            Param::number("frag_min_rel", "Fragments from").min(0.0).max(100.0).default_num(d.frag_min_rel).unit("%"),
            Param::number("min_snr", "Min. S/N of a peak").min(0.0).default_num(d.min_snr),
            Param::number("max_fwhm_s", "Max. FWHM of a peak").min(0.0).default_num(d.max_fwhm_s).unit("s"),
            Param::integer("min_points", "Min. scans of a peak").min(1.0).default_num(d.min_points as f64),
            Param::number("dead_factor", "Dead-volume factor").min(0.0).default_num(d.dead_factor),
            Param::boolean("parent_decay", "Parent must fall over time").default_bool(d.parent_decay),
        ],
        runs_in: RunsIn {
            wasm: Some("wasip1 (mzlab-cli)"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.flow.lineage", "err.flow.link.series"],
        ..Step::core(
            "tp.dig_lr",
            "Dig the unit-resolution series",
            "Scores the products a parent can form in a series at unit resolution: peaks checked on their XIC, persistence, growth, blanks, chance. It proposes candidates; it does not identify.",
        )
    }
}

/// The settings the page sends: only the keys the plan wrote, so the rest keeps the defaults of the module.
fn settings_json(job: &Job) -> String {
    let mut o = Out::default();
    o.raw("{");
    for (i, (k, v)) in job.resolved.params.iter().enumerate() {
        o.raw(if i == 0 { "" } else { "," }).str(k).raw(":");
        match (v.f64(), v.bool()) {
            (Some(x), _) => o.num(x),
            (_, Some(b)) => o.raw(if b { "true" } else { "false" }),
            _ => o.str("null"),
        };
    }
    o.raw("}");
    o.0
}

/// `tp.dig_lr`: the columns are the files of the table, in order; the hypotheses come from the rules of the Dig.
pub(crate) fn dig_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table_in = job
        .inputs
        .iter()
        .find(|(p, _)| p == "table")
        .and_then(|(_, v)| v.first());
    let lin = table_in
        .and_then(|a| r.lineage(a))
        .filter(|l| !l.peaks.is_empty())
        .ok_or_else(|| Error::new("err.flow.lineage").with("step", job.step.id))?;
    let indexes = all_inputs(r, job, "index");
    if lin.peaks.len() != indexes.len() {
        return Err(Error::new("err.flow.link.series")
            .with("peaks", lin.peaks.len())
            .with("index", indexes.len()));
    }
    let names: Vec<String> = job
        .inputs
        .iter()
        .filter(|(p, _)| p == "index")
        .flat_map(|(_, v)| v)
        .map(|a| r.source_name(a))
        .collect();
    let (table, samples, parent) = (
        read_one(r, job, "table")?,
        read_one(r, job, "samples")?,
        read_one(r, job, "parent")?,
    );
    let p = parent_of(&parent.1)?;
    let mut recipe = rebuild(&lin)?;
    // the sign of the parent adduct is the polarity the products are looked for in
    if p.adduct.ends_with('-') {
        recipe.detect.polarity = Some(-1);
    }
    let mut ix = indexes.iter().map(open).collect::<Result<Vec<_>>>()?;
    let hypo = run::hypotheses_stored(
        &mut ix,
        &mut Files(lin.peaks.clone()),
        RULES,
        &p.structure,
        &p.formula,
        None,
        &recipe,
    )?;
    let mut files = Out::default();
    files.raw("{\"files\":[");
    for (i, n) in names.iter().enumerate() {
        files
            .raw(if i == 0 { "" } else { "," })
            .raw("{\"name\":")
            .str(n)
            .raw(",\"kind\":\"full\"}");
    }
    files
        .raw("],\"campioni\":")
        .raw(String::from_utf8_lossy(&samples.1).trim())
        .raw("}");
    let mut who = Out::default();
    who.raw("{\"name\":\"parent\",\"neutral\":")
        .str(&p.formula)
        .raw(",\"adduct\":")
        .str(&p.adduct)
        .raw("}");
    let d = lr::dig(
        &files.0,
        &who.0,
        &settings_json(job),
        &table.1,
        &hypo.registry.to_json(),
        &mut |k| open(&indexes[k]).map_err(|e| e.key.to_string()),
    )
    .map_err(|e| Error::new("err.flow.dig").with("detail", e))?;
    write_made(r, job, summary(&d, "{}") + "\n", DataType::Report)
}
