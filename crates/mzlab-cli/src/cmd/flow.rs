//! The `flow` commands and the list of Flow steps.

use super::Entry;
use crate::Outcome;

/// `mzlab-cli flow command PLAN.json`: the script that reruns the plan, on stdout.
fn command(args: Vec<String>) -> Outcome {
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

/// `mzlab-cli flow` with a word that is not a subcommand.
fn usage(_: Vec<String>) -> Outcome {
    Outcome::fail("usage: mzlab-cli flow run PLAN.json ... | mzlab-cli flow check PLAN.json | mzlab-cli flow report RUNDIR [--out FILE] | mzlab-cli flow record RUNDIR --roles ROLES.json | mzlab-cli flow verify RECORD | mzlab-cli flow command PLAN.json")
}

pub(crate) static TABLE: &[Entry] = &[
    Entry { name: "flow run", aliases: &[], help: "mzlab-cli flow run PLAN.json [--in FILE|FOLDER...] --out FOLDER [--set INSTANCE.NAME=VALUE...]   a whole Flow plan (flow run --help)", run: |a| crate::flow_run::run(a, None) },
    Entry { name: "flow check", aliases: &[], help: "mzlab-cli flow check PLAN.json [--set INSTANCE.NAME=VALUE...] [--json]   is the plan valid, without the input files (flow check --help)", run: crate::flow_json::check },
    Entry { name: "steps", aliases: &[], help: "mzlab-cli steps [--json] [--id ID[@V]]   the Flow steps of this program, native or page only (steps --help)", run: crate::steps_cmd::run },
    Entry { name: "flow report", aliases: &[], help: "mzlab-cli flow report RUNDIR [--out FILE]   the corsa/1 report of a run folder (prov.json and the files beside it)", run: report },
    Entry { name: "flow record", aliases: &[], help: "mzlab-cli flow record RUNDIR --roles ROLES.json [--out FILE]   the mzrun/1 record of a run; flow verify RECORD checks its fingerprints", run: crate::flow_mzrun::record },
    Entry { name: "flow verify", aliases: &[], help: "", run: crate::flow_mzrun::check },
    Entry { name: "flow command", aliases: &[], help: "", run: command },
    Entry { name: "flow", aliases: &[], help: "", run: usage },
];
