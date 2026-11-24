//! `mzlab-cli schema [NAME|--list]`: the JSON Schemas of `docs/schemi/`, embedded at build time. `--list` is the `schemas/1` document; `NAME`
//! (`error/1`, `mzflow/2`) prints that schema. The step and tool catalogs of the folder are data, not schemas, and are not embedded.

use mzlab_core::json::{parse, Value};
use mzlab_core::jsonb::{text, write, Obj};

use crate::Outcome;

macro_rules! embed {
    ($($f:literal),* $(,)?) => {
        pub(crate) static FILES: &[(&str, &str)] = &[$(($f, include_str!(concat!("../../../docs/schemi/", $f)))),*];
    };
}

embed!(
    "analysis-1.json",
    "annotations-1.json",
    "assist-roles-1.json",
    "channels-1.json",
    "check-1.json",
    "companions-1.json",
    "compare-1.json",
    "cip-1.json",
    "commands-1.json",
    "commands-find-1.json",
    "convert-1.json",
    "compact-steps-1.json",
    "context-1.json",
    "cut-1.json",
    "descriptors-1.json",
    "design-1.json",
    "design_canonical-1.json",
    "design_check-1.json",
    "design_proposal-1.json",
    "draw-1.json",
    "draw_edit-1.json",
    "draw-image-1.json",
    "elements-1.json",
    "estimate-1.json",
    "estimates-1.json",
    "error-1.json",
    "evalrun-1.json",
    "evid-1.json",
    "feature_pipeline-3.json",
    "features-notes-1.json",
    "features-plan-1.json",
    "features-report-1.json",
    "file_info-1.json",
    "filtri-1.json",
    "flowcheck-1.json",
    "flowcommand-1.json",
    "flowdry-1.json",
    "flowlive-1.json",
    "flowrun-1.json",
    "folder-1.json",
    "fragrules-1.json",
    "grafico-1.json",
    "groups-1.json",
    "highlight-1.json",
    "hypotheses-3.json",
    "insilico-1.json",
    "inspection-1.json",
    "interp-1.json",
    "ion_mz-1.json",
    "ion_pattern-1.json",
    "index-build-1.json",
    "items-1.json",
    "raw-cache-1.json",
    "raw-info-1.json",
    "raw-plan-1.json",
    "pack-run-1.json",
    "unpack-run-1.json",
    "verify-pack-1.json",
    "label-1.json",
    "libmatch-1.json",
    "link-1.json",
    "liste-1.json",
    "llm-conv-1.json",
    "llm-provider-1.json",
    "llmlog-1.json",
    "llmreg-1.json",
    "massacc-1.json",
    "quant-targets-1.json",
    "mechanism-1.json",
    "molecules-1.json",
    "msn-figures-1.json",
    "msn-graph-1.json",
    "msn-lib-export-1.json",
    "msn-lib-op-1.json",
    "msn-lib-1.json",
    "msn-rules-1.json",
    "msn-rules-sum-1.json",
    "msn-search-1.json",
    "msn-summary-1.json",
    "mz-match-1.json",
    "mzflow-2.json",
    "mzml-validation-1.json",
    "mzrun-2.json",
    "noise-floor-1.json",
    "normative-build-1.json",
    "open-1.json",
    "param-choices-1.json",
    "param-proposal-1.json",
    "peak-auto-1.json",
    "peak-auto-lr-1.json",
    "plausible-1.json",
    "prov-1.json",
    "reading-1.json",
    "replay-1.json",
    "reflist-1.json",
    "regole_tp-3.json",
    "rete-1.json",
    "run-report-2.json",
    "runorder-1.json",
    "sample-import-1.json",
    "sample-list-1.json",
    "sample-select-1.json",
    "samples-1.json",
    "scanlist-1.json",
    "schemas-1.json",
    "screen-1.json",
    "selection-1.json",
    "spectrum-1.json",
    "step-params-1.json",
    "substructure-1.json",
    "substructure_db-1.json",
    "tabcheck-1.json",
    "tools-compact-1.json",
    "tools-schema-1.json",
    "tp-2.json",
    "trace-1.json",
    "trace-set-1.json",
    "union-1.json",
    "uvband-1.json",
    "verify-1.json",
    "version-1.json",
    "xicplan-1.json",
);

/// The name (`$id`) and the title of an embedded schema.
fn head(src: &str) -> (String, String) {
    let v = parse(src.as_bytes()).unwrap_or(Value::Null);
    let get = |k: &str| v.get(k).and_then(Value::str).unwrap_or("").to_string();
    (get("$id"), get("title"))
}

/// Every embedded schema: name, title, text; sorted by name.
pub(crate) fn all() -> Vec<(String, String, &'static str)> {
    let mut v: Vec<_> = FILES
        .iter()
        .map(|(_, src)| {
            let (n, t) = head(src);
            (n, t, *src)
        })
        .collect();
    v.sort();
    v
}

pub(crate) fn list_json() -> String {
    let items = all()
        .iter()
        .map(|(n, t, s)| {
            Obj::new()
                .put("name", text(n))
                .put("title", text(t))
                .i("bytes", s.len() as i64)
                .done()
        })
        .collect();
    write(
        &Obj::new()
            .put("schema", text("schemas/1"))
            .put("schemas", Value::Arr(items))
            .done(),
    ) + "\n"
}

pub(crate) fn run(args: Vec<String>) -> Outcome {
    let mut name = None;
    let mut list = false;
    for a in args {
        match a.as_str() {
            "--list" => list = true,
            "--json" => {}
            s if s.starts_with('-') => return Outcome::fail(&format!("unknown option {s}")),
            _ if name.is_none() => name = Some(a),
            _ => return Outcome::fail("one schema name at most"),
        }
    }
    let Some(want) = name.filter(|_| !list) else {
        return Outcome::ok(list_json().into_bytes());
    };
    match all().into_iter().find(|(n, _, _)| *n == want) {
        Some((_, _, src)) => Outcome::ok(src.as_bytes().to_vec()),
        None => {
            let err = mzlab_core::Error::new("err.cli.schema.unknown").with("name", want);
            Outcome::fail_error(&err).with_hint("mzlab-cli schema --list lists the schemas")
        }
    }
}
