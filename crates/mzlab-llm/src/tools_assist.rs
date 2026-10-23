//! The tools of the data assistant (`data_inspect`, `params_propose`, `table_check`, `table_rows`, `ask_user`): read or propose only, never an edit
//! or a run. Their arguments are closed schemas; the answers are built by Rust (`ispezione/1`, `proposta-param/1`, `tabcheck/1`). Used by `tools::catalog`.

use crate::tools::{object, quote, s, step_refs, Class, Tool};
use mzlab_core::step::Step;

/// The check ids of `tabcheck/1` (`docs/schemi/tabcheck-1.json`).
const CHECKS: &str = "[\"size\",\"blank\",\"contaminants\",\"qc_cv\",\"qc_intensity\",\"rt_drift\",\"duplicates\",\"adducts\",\"isotopes\",\"insource\",\"long_traces\",\"homologous\",\"missing\",\"structure\",\"polarity_units\"]";

fn list(item: &str, max: usize) -> String {
    format!("{{\"type\":\"array\",\"items\":{item},\"maxItems\":{max}}}")
}

fn question() -> String {
    let props = [
        ("id", s("Short id of the question.")),
        ("text", "{\"type\":\"string\",\"minLength\":1}".to_string()),
        (
            "options",
            "{\"type\":\"array\",\"minItems\":2,\"maxItems\":8,\"items\":{\"type\":\"string\"}}"
                .to_string(),
        ),
    ];
    object(&props, &["id", "text", "options"])
}

/// A closed row filter: one column, one comparison, one value.
fn filter() -> String {
    let props = [
        ("column", s("Column of the table.")),
        (
            "op",
            "{\"enum\":[\"eq\",\"lt\",\"le\",\"gt\",\"ge\"]}".to_string(),
        ),
        (
            "value",
            "{\"anyOf\":[{\"type\":\"number\"},{\"type\":\"string\"}]}".to_string(),
        ),
    ];
    object(&props, &["column", "op", "value"])
}

/// The arguments of `code_export`.
pub(crate) fn code_schema() -> String {
    object(
        &[
            ("language", "{\"enum\":[\"python\",\"r\",\"shell\"]}".into()),
            ("goal", s("What the script is for.")),
        ],
        &["language", "goal"],
    )
}

/// The five tools, in catalog order; `steps` is sorted by id.
pub fn tools(steps: &[&Step]) -> Vec<Tool> {
    let alias = list(&s("Alias of an open file."), 50);
    let rows = format!(
        "{{\"type\":\"integer\",\"minimum\":1,\"maximum\":50,\"default\":20,\"description\":{}}}",
        quote("At most 50 rows.")
    );
    let step_ids = list(&format!("{{\"enum\":{}}}", step_refs(steps)), 40);
    let ids = list(&s("Row id."), 50);
    let checks = list(&format!("{{\"enum\":{CHECKS}}}"), 15);
    vec![
        Tool { name: "data_inspect", class: Class::Read, description: "What each open file is (instrument, polarity, series, blanks and QC) as the inspection of the program; no raw data.", input_schema: object(&[("files", alias)], &[]) },
        Tool { name: "params_propose", class: Class::Propose, description: "Parameter values for the given steps from the rules of the program, each with its source, confidence and reason; shown as a proposal.", input_schema: object(&[("steps", step_ids)], &["steps"]) },
        Tool { name: "table_check", class: Class::Read, description: "The checks of the program on the feature table of the last run: flags and the numbers behind them.", input_schema: object(&[("checks", checks)], &[]) },
        Tool { name: "table_rows", class: Class::Read, description: "Reduced rows of the feature table, by id or by one filter; at most 50.", input_schema: object(&[("ids", ids), ("where", filter()), ("limit", rows)], &[]) },
        Tool { name: "ask_user", class: Class::Propose, description: "Ask the user up to 5 closed questions, never one the program can answer; the answer comes back as the tool result.", input_schema: object(&[("questions", format!("{{\"type\":\"array\",\"minItems\":1,\"maxItems\":5,\"items\":{}}}", question()))], &["questions"]) },
    ]
}
