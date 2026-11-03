//! The catalog of the public demo: the subset of `catalog` that the proxy of mzLab accepts (`docs/schemi/strumenti-demo-1.json`, SHA-256 in its
//! allow-list), with the order and the schemas of the whole catalog. Used by the CLI export, the app and the worker of the site.

use mzlab_core::step::Step;

use super::{catalog, tools_file, Tool, UiTool};

/// The tools of the demo of the public site (`docs/studi/assistente-dati/12-demo.md`): read, propose and set a parameter, nothing that runs,
/// removes or exports; the proxy accepts only this catalog (SHA-256 in its allow-list).
pub const DEMO_TOOLS: &[&str] = &[
    "flow_list_steps",
    "flow_describe_step",
    "flow_get_plan",
    "flow_check",
    "flow_propose_plan",
    "flow_set_param",
    "data_summary",
    "data_inspect",
    "params_propose",
    "table_check",
    "ask_user",
    "chart_make",
];

/// The demo subset of `catalog`, in the order and with the schemas of the whole catalog.
pub fn demo_catalog(steps: &[Step], ui: &[UiTool], executable: &dyn Fn(&str) -> bool) -> Vec<Tool> {
    let mut all = catalog(steps, ui, executable);
    all.retain(|t| DEMO_TOOLS.contains(&t.name));
    all
}

/// `demo_catalog` as the file `strumenti/1` (`docs/schemi/strumenti-demo-1.json`).
pub fn demo_catalog_json(
    steps: &[Step],
    ui: &[UiTool],
    executable: &dyn Fn(&str) -> bool,
) -> String {
    tools_file(&demo_catalog(steps, ui, executable))
}
