//! The `flow` commands and the list of Flow steps.

use super::Entry;
use crate::Outcome;

/// `mzlab-cli flow command PLAN.json`: the script that reruns the plan, on stdout.
fn command(args: Vec<String>) -> Outcome {
    if let Err(e) = crate::argv::no_options(&args) {
        return Outcome::fail_error(&e);
    }
    let [path] = args.as_slice() else {
        return Outcome::fail("usage: mzlab-cli flow command PLAN.json");
    };
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"));
    match text.and_then(|t| mzlab_core::flow::command_route(&t).map_err(|e| e.to_string())) {
        Ok(s) => Outcome::ok(s.into_bytes()),
        Err(e) => Outcome::fail(&e),
    }
}

fn report(args: Vec<String>) -> Outcome {
    crate::flow_report::run(args.into_iter()).map_or_else(|e| Outcome::fail(&e), Outcome::ok)
}

pub(crate) static TABLE: &[Entry] = &[
    Entry::new("flow run", "mzlab-cli flow run PLAN.json [--in FILE|FOLDER...] --out FOLDER [--set INSTANCE.NAME=VALUE...]   a whole Flow plan", |a| crate::flow_run::run(a, None)),
    Entry::new("flow check", "mzlab-cli flow check PLAN.json [--set INSTANCE.NAME=VALUE...] [--json]   is the plan valid, without the input files", crate::flow_json::check),
    Entry::new("flow steps", "mzlab-cli flow steps [--json] [--id ID[@V]]   the Flow steps of this program, native or page only", crate::steps_cmd::run),
    Entry::new("flow report", "mzlab-cli flow report RUNDIR [--out FILE]   the run-report/1 report of a run folder (prov.json and the files beside it)", report),
    Entry::new("flow record", "mzlab-cli flow record RUNDIR --roles ROLES.json [--out FILE]   the mzrun/2 record of a run (a role without sha256 takes it from the run inputs)", crate::flow_mzrun::record),
    Entry::new("flow verify", "mzlab-cli flow verify RECORD   whether the fingerprints of an mzrun/1 record match its files", crate::flow_mzrun::check),
    Entry::new("flow command", "mzlab-cli flow command PLAN.json   the script that reruns the plan, on stdout", command),
    Entry::new("explain-missing", "mzlab-cli explain-missing TABLE.mzfeat EXPECTED.csv   where a table loses each expected product", crate::flow_explain::run),
];
