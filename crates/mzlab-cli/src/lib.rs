//! `mzlab-cli features FILE... [--out PATH] [--preset auto|pos|neg | --params FILE.json]`: the feature engine of the app on the command line.
//! A thin wrapper: the index of each file is built as the desktop app builds it, the recipe comes from `run::recipe_for` (the one behind the panel
//! "Pipeline"), the steps are `run::detect` and `run::align`, and the CSV is the export "full" of the table (`core::table_file`), so the numbers are the ones of the page.

use std::path::Path;

use mzlab_core::error::{Error, Result};

mod argv;
mod catalog;
#[cfg(test)]
mod catalog_tests;
mod channels_cmd;
pub mod citation;
mod cmd;
mod contract;
#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod contract_verdict_tests;
pub mod convert;
mod convert_dry;
mod convert_entries;
mod convert_help;
mod convert_report;
mod dad_cmd;
mod dad_ms;
mod dad_msat;
mod design_cmd;
mod emit;
pub mod error_text;
pub(crate) use emit::emit;
pub mod exit;
mod extract;
#[cfg(test)]
mod features_lr_tests;
mod features_series;
mod file_index_cmd;
mod file_info_cmd;
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
mod flow_curve;
#[cfg(test)]
mod flow_detect_adap_tests;
#[cfg(test)]
mod flow_detect_algorithm_tests;
mod flow_digl;
mod flow_drift;
#[cfg(test)]
mod flow_drift_tests;
mod flow_estimate;
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
mod flow_quant_targets;
#[cfg(test)]
mod flow_quant_targets_tests;
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
mod flow_tablecmp;
mod flow_tablematch;
#[cfg(test)]
mod flow_tablematch_tests;
mod flow_template;
mod flow_transitions;
mod idx_cache;
mod idx_open;
pub mod insilico_pack;
mod ion_pattern_cmd;
mod items_cmd;
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
mod mz_offset;
#[cfg(test)]
mod mz_offset_tests;
#[cfg(test)]
mod mzml_variants_tests;
#[cfg(test)]
mod name_rule_tests;
#[cfg(not(target_family = "wasm"))]
pub mod normative;
#[cfg(not(target_family = "wasm"))]
mod pack_cmd;
#[cfg(all(test, not(target_family = "wasm")))]
mod pack_cmd_tests;
#[cfg(not(target_family = "wasm"))]
mod pack_read_cmd;
mod pair_cmd;
mod par;
#[cfg(test)]
mod peak_exits_tests;
mod peaks_auto_cmd;
mod peaks_auto_lr_cmd;
mod peaks_edges_cmd;
mod plain;
mod quant_targets_cmd;
mod renamed;
mod sample_select;
mod samples_cmd;
mod schema_cmd;
mod spectrum_cmd;
mod steps_cmd;
mod table_compare_cmd;
#[cfg(test)]
mod table_compare_cmd_tests;
mod trace_cmd;
mod trace_xic;
#[cfg(not(target_family = "wasm"))]
mod validate;
mod wiff_cmd;

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

pub use features_series::{features, features_full, features_of, features_plan_of};

/// Why a run failed, in the words of `error/1`: the key the texts and docs know, its parameters and the hint.
#[derive(Debug, Clone, Default)]
pub struct Failure {
    pub key: String,
    pub params: Vec<(String, String)>,
    pub hint: String,
    /// The `problems` array of a plan that is not valid, already JSON.
    pub problems: Option<String>,
}

/// What a run of the command leaves: exit code, standard output, standard error, and for a failure its `error/1` data.
pub struct Outcome {
    pub code: u8,
    pub stdout: Vec<u8>,
    pub stderr: String,
    pub error: Option<Box<Failure>>,
}

impl Outcome {
    fn ok(stdout: Vec<u8>) -> Self {
        Outcome {
            code: 0,
            stdout,
            stderr: String::new(),
            error: None,
        }
    }

    /// A failure with the message as typed: an engine key at its start (`err.x.y k=v`) keeps it and takes its code from the table,
    /// any other text is a usage error (`err.cli.usage`) that carries the text as its `message` parameter.
    fn fail(msg: &str) -> Self {
        if let Some(o) = Self::unreadable_input(msg) {
            return o;
        }
        let key = msg.split(' ').next().unwrap_or("");
        let (code, error) = if key.starts_with("err.") || key.starts_with("error.") {
            let params = error_text::text_params(&msg[key.len()..]);
            (
                exit::code_of_key(key),
                Failure {
                    key: key.to_string(),
                    params,
                    ..Failure::default()
                },
            )
        } else {
            (
                exit::USAGE,
                Failure {
                    key: "err.cli.usage".to_string(),
                    params: vec![("message".to_string(), msg.to_string())],
                    ..Failure::default()
                },
            )
        };
        Outcome {
            code,
            stdout: Vec::new(),
            stderr: format!("error: {}\n", error_text::human(msg)),
            error: Some(Box::new(error)),
        }
    }

    /// A `PATH: reason (os error N)` text of a file that is not there or cannot be opened (not found, permission, a folder where a file was
    /// expected): an input failure (`err.cli.file`, exit 4), whatever the command that read it.
    fn unreadable_input(msg: &str) -> Option<Self> {
        let (path, rest) = msg.split_once(": ")?;
        let errno: u32 = rest
            .rsplit_once("(os error ")?
            .1
            .strip_suffix(')')?
            .parse()
            .ok()?;
        if !matches!(errno, 2 | 13 | 20 | 21) || path.contains(' ') && path.starts_with("error") {
            return None;
        }
        Some(Self::fail_error(
            &Error::new("err.cli.file").with("path", path),
        ))
    }

    /// The same failure with a hint: a `hint:` line on stderr and the `hint` of `error/1`.
    pub(crate) fn with_hint(mut self, hint: &str) -> Self {
        if !hint.is_empty() {
            self.stderr += &format!("hint: {hint}\n");
            if let Some(f) = self.error.as_mut() {
                f.hint = hint.to_string();
            }
        }
        self
    }
}

/// `mzlab-cli <args>`: `args` without the program name.
pub fn execute(args: &[String]) -> Outcome {
    let (command, o) = executed(args);
    error_text::with_help(args, contract::finish(args, command, o))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod workflow_tests;

#[cfg(test)]
mod chem_contract_tests;
#[cfg(test)]
mod chem_v3000_tests;
#[cfg(test)]
mod convert_fixtures;
#[cfg(test)]
mod convert_tests;
#[cfg(test)]
mod file_sample_contract_tests;
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
mod flow_level_ms2_tests;
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
pub mod folder_cmd;
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

fn executed(args: &[String]) -> (&'static str, Outcome) {
    if let Some(o) = cmd::top_level(args) {
        return ("", o);
    }
    let Some((e, n)) = cmd::find(args) else {
        if let Some((path, inside)) = cmd::group_of(args) {
            let n = path.split(' ').count();
            if args.len() == n || cmd::wants_help(&args[n..]) {
                return ("", Outcome::ok(cmd::group_help(&inside).into_bytes()));
            }
            let o = Outcome::fail_error(&Error::new("err.cli.usage").with(
                "message",
                format!("unknown subcommand {} of {path}", args[n]),
            ));
            return ("", o);
        }
        return ("", renamed::unknown_command(args));
    };
    let mut rest = args[n..].to_vec();
    // `db normative` has its own `--cache` (sources folder): it keeps it and never sets the index cache
    if e.name.starts_with("db normative") {
        idx_cache::set(None);
    } else if let Err(m) = idx_cache::take_flag(&mut rest) {
        return (e.name, Outcome::fail(&m));
    }
    if cmd::wants_help(&rest) {
        return (e.name, cmd::help_of(e, &rest));
    }
    if rest.iter().any(|a| a == "--json") && !cmd::reads_json(e.name) {
        if !cmd::json_by_default(e.name) {
            return (e.name, contract::json_unsupported());
        }
        rest.retain(|a| a != "--json");
    }
    let mut o = (e.run)(rest);
    // `index reused` / `index built`: what the cache did, beside the answer (a failure keeps its message first and alone)
    let notes = idx_cache::take_notes();
    if o.code == 0 {
        o.stderr += &notes;
    } else {
        // the message of a failure stays alone in the outcome (`error/1` reads it); the cache's lines still reach the terminal
        eprint!("{notes}");
    }
    (e.name, o)
}
