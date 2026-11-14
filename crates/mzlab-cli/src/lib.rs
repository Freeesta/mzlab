//! `mzlab-cli features FILE... [--out PATH] [--preset auto|pos|neg | --params FILE.json]`: the feature engine of the app on the command line.
//! A thin wrapper: the index of each file is built as the desktop app builds it, the recipe comes from `run::recipe_for` (the one behind the panel
//! "Pipeline"), the steps are `run::detect` and `run::align`, and the CSV is the export "full" of the table (`core::table_file`), so the numbers are the ones of the page.

use std::path::{Path, PathBuf};

use mzlab_core::budget::{sha256_hex, MemBudget};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Out;
use mzlab_ops::run::feat::run;
use mzlab_ops::run::table::core::{samples_json, table_file};

mod channels_cmd;
mod cmd;
pub mod convert;
mod convert_entries;
mod convert_report;
pub mod error_text;
pub mod flow;
#[cfg(test)]
mod flow_align_join_tests;
mod flow_apply;
mod flow_base;
#[cfg(test)]
mod flow_bentazone_export_tests;
#[cfg(test)]
mod flow_bentazone_tests;
mod flow_cache;
mod flow_centroid;
#[cfg(test)]
mod flow_centroid_tests;
mod flow_chimeric;
#[cfg(test)]
mod flow_chimeric_tests;
mod flow_compare;
#[cfg(test)]
mod flow_detect_adap_tests;
#[cfg(test)]
mod flow_detect_algorithm_tests;
mod flow_digl;
mod flow_drift;
#[cfg(test)]
mod flow_drift_tests;
mod flow_estimate;
mod flow_explain;
mod flow_export;
#[cfg(test)]
mod flow_export_tests;
mod flow_formula;
#[cfg(test)]
mod flow_formula_web_tests;
mod flow_generate;
mod flow_group;
#[cfg(test)]
mod flow_group_tests;
mod flow_hash;
#[cfg(test)]
mod flow_hr_tests;
mod flow_import;
#[cfg(test)]
mod flow_import_tests;
mod flow_inclusion;
#[cfg(test)]
mod flow_inclusion_tests;
mod flow_inputs;
mod flow_insilico;
#[cfg(test)]
mod flow_iso_grouper_tests;
mod flow_json;
mod flow_kmd;
mod flow_level;
mod flow_link;
#[cfg(test)]
mod flow_link_tests;
mod flow_lookup;
#[cfg(test)]
mod flow_lookup_tests;
#[cfg(test)]
mod flow_lr_bench_tests;
#[cfg(test)]
mod flow_lr_tests;
mod flow_made;
mod flow_massacc;
#[cfg(test)]
mod flow_massacc_tests;
mod flow_match;
#[cfg(test)]
mod flow_match_tests;
mod flow_match_tree;
#[cfg(test)]
mod flow_match_web_tests;
mod flow_ms2;
mod flow_msnfigure;
mod flow_msngraph;
mod flow_msnlib;
#[cfg(test)]
mod flow_msnlib_tests;
#[cfg(test)]
mod flow_msnraw_tests;
mod flow_mzrun;
mod flow_nature;
#[cfg(test)]
mod flow_neglr_tests;
mod flow_network;
mod flow_out;
mod flow_pace;
#[cfg(test)]
mod flow_pace_tests;
mod flow_pca;
#[cfg(test)]
mod flow_pca_tests;
mod flow_position;
#[cfg(test)]
mod flow_position_tests;
mod flow_quant;
mod flow_reactivity;
#[cfg(test)]
mod flow_reactivity_tests;
mod flow_read;
mod flow_refilter;
#[cfg(test)]
mod flow_refilter_tests;
mod flow_render;
#[cfg(test)]
mod flow_render_tests;
pub mod flow_report;
mod flow_run;
mod flow_set;
mod flow_stats;
mod flow_table;
#[cfg(test)]
mod flow_table_tests;
mod flow_tablematch;
#[cfg(test)]
mod flow_tablematch_tests;
mod flow_transitions;
mod idx_open;
pub mod insilico_pack;
mod ion_pattern_cmd;
pub mod llm;
mod llm_ctx;
mod manual_table_cmd;
mod mol_cut_cmd;
mod mol_label_cmd;
mod mol_search_cmd;
#[cfg(test)]
mod msn_figure_tests;
pub mod msn_graph;
mod msn_graph_compare;
mod msn_lib;
pub mod msn_outputs;
mod msn_png;
mod msn_rules;
#[cfg(test)]
mod mzml_variants_tests;
#[cfg(test)]
mod name_rule_tests;
#[cfg(not(target_family = "wasm"))]
pub mod normative;
mod par;
mod peaks_auto_cmd;
mod peaks_edges_cmd;
mod plain;
mod samples_cmd;
mod steps_cmd;
#[cfg(not(target_family = "wasm"))]
mod validate;

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// The `.mzidx` of `path` written to `idx` with `budget` bytes of working memory: an mzML through its scan table, any other file or dataset (a `.wiff` with
/// its `.wiff.scan`, a `.D`/`.d`/`.raw` folder) through the reader of the registry. `label` is the plan input (`NAME@ID` names the sample `ID`).
pub(crate) use flow_pace::build_index_of;

pub(crate) fn build_index(path: &Path, idx: &Path, budget: usize) -> Result<()> {
    build_index_of(path, "", idx, budget)
}

pub(crate) use idx_open::open;

/// The CSV of the series `files`, indexes kept in `tmp` (the caller owns it: WASI has no temporary folder).
pub fn features(files: &[PathBuf], params: &str, tmp: &Path) -> Result<String> {
    let idxs: Vec<PathBuf> = (0..files.len())
        .map(|k| tmp.join(format!("{k}.mzidx")))
        .collect();
    let (recipe, detected, weak) = crate::par::index_and_detect(files, &idxs, params)?;
    let mut series = crate::par::Series(detected, weak);
    let origin = run::origin_of(&mut series)?;
    eprintln!("align {} file(s)", files.len());
    let mut table = Vec::new();
    let budget = MemBudget::mb(mzlab_core::budget_auto::native_auto_mb());
    run::align_to_par(
        &mut series,
        &budget,
        &recipe,
        &|k| open(&idxs[k]),
        &mut table,
    )?;
    let mut list = Out::default();
    list.raw("{\"files\":[");
    for (k, (name, sha)) in origin.inputs.iter().enumerate() {
        list.raw(if k > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(name)
            .raw(",\"sha256\":")
            .str(sha)
            .raw("}");
    }
    list.raw("]}");
    let samples = samples_json(&list.0)?;
    let polarity = recipe.detect.polarity.map_or(1, i32::from);
    table_file(
        &mut mzlab_feat::store::Slice(&table),
        table.len() as u64,
        &samples,
        "",
        &sha256_hex(&table),
        polarity,
        "full",
    )
}

/// What a run of the command leaves: exit code, standard output, standard error.
pub struct Outcome {
    pub code: u8,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Outcome {
    fn ok(stdout: Vec<u8>) -> Self {
        Outcome {
            code: 0,
            stdout,
            stderr: String::new(),
        }
    }

    fn fail(msg: &str) -> Self {
        Outcome {
            code: 2,
            stdout: Vec::new(),
            stderr: format!("error: {}\n", error_text::human(msg)),
        }
    }
}

/// The text to the standard output, or to the file of `--out`.
fn emit(t: String, out: &Option<PathBuf>) -> Outcome {
    match out {
        None => Outcome::ok(t.into_bytes()),
        Some(p) => std::fs::write(p, t).map_or_else(
            |e| Outcome::fail(&e.to_string()),
            |_| Outcome::ok(Vec::new()),
        ),
    }
}

/// `mzlab-cli <args>`: `args` without the program name.
pub fn execute(args: &[String]) -> Outcome {
    error_text::with_help(args, executed(args))
}

fn executed(args: &[String]) -> Outcome {
    if args.is_empty() || matches!(args[0].as_str(), "--help" | "-h") {
        return cmd::help();
    }
    match cmd::find(args) {
        Some((e, n)) => (e.run)(args[n..].to_vec()),
        None => Outcome::fail(&format!("unknown command {}", plain::canonical(&args[0]))),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod convert_fixtures;
#[cfg(test)]
mod convert_tests;
#[cfg(test)]
mod flow_budget_tests;
#[cfg(test)]
mod flow_cache_tests;
#[cfg(test)]
mod flow_command_tests;
#[cfg(test)]
mod flow_compare_tests;
#[cfg(test)]
mod flow_estimate_tests;
#[cfg(test)]
mod flow_generate_tests;
#[cfg(test)]
mod flow_ident_tests;
#[cfg(test)]
mod flow_json_tests;
#[cfg(test)]
mod flow_kmd_tests;
#[cfg(test)]
mod flow_level_tests;
#[cfg(test)]
mod flow_ms2_tests;
#[cfg(test)]
mod flow_msngraph_tests;
#[cfg(test)]
mod flow_mzrun_tests;
#[cfg(test)]
mod flow_network_tests;
#[cfg(test)]
mod flow_peaks_tests;
#[cfg(test)]
mod flow_report_tests;
#[cfg(test)]
mod flow_rule_tests;
#[cfg(test)]
mod flow_run_tests;
#[cfg(test)]
mod flow_stats_tests;
#[cfg(test)]
mod llm_agent_tests;
#[cfg(test)]
mod llm_eval_tests;
#[cfg(test)]
mod llm_replay_tests;
#[cfg(test)]
mod llm_tools_tests;
#[cfg(test)]
mod msn_graph_compare_tests;
#[cfg(test)]
mod msn_graph_structures_tests;
#[cfg(test)]
mod msn_graph_tests;
#[cfg(test)]
mod msn_lib_tests;
#[cfg(all(test, not(target_family = "wasm")))]
mod validate_tests;
