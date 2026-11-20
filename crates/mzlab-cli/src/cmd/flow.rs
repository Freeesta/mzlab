//! The `flow` commands and the list of Flow steps.

use super::Entry;
use crate::Outcome;

/// `mzlab-cli flow command PLAN.json [--json]`: the script that reruns the plan, on stdout (or `flowcommand/1` with the script as `script`).
fn command(args: Vec<String>) -> Outcome {
    let json = args.iter().any(|a| a == "--json");
    let args: Vec<String> = args.into_iter().filter(|a| a != "--json").collect();
    if let Err(e) = crate::argv::no_options(&args) {
        return Outcome::fail_error(&e);
    }
    let [path] = args.as_slice() else {
        return Outcome::fail("usage: mzlab-cli flow command PLAN.json [--json]");
    };
    let plan = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return Outcome::fail(&format!("{path}: {e}")),
    };
    match mzlab_core::flow::command_route(&plan) {
        Ok(s) if json => {
            use mzlab_core::jsonb::{text, write, Obj};
            let doc = Obj::new()
                .put("schema", text("flowcommand/1"))
                .put("script", text(&s))
                .done();
            Outcome::ok((write(&doc) + "\n").into_bytes())
        }
        Ok(s) => Outcome::ok(s.into_bytes()),
        Err(e) => Outcome::fail_error(&e),
    }
}

fn report(args: Vec<String>) -> Outcome {
    crate::flow_report::run(args)
}

pub(crate) static TABLE: &[Entry] = &[
    Entry::new("flow run", "mzlab-cli flow run PLAN.json [--in FILE|FOLDER...] --out FOLDER [--set INSTANCE.NAME=VALUE...]   a whole Flow plan", |a| crate::flow_run::run(a, None)).examples(&["mzlab-cli flow run plan.json --in data/ --out run1 --json"]),
    Entry::new("flow check", "mzlab-cli flow check PLAN.json [--set INSTANCE.NAME=VALUE...] [--json]   is the plan valid, without the input files", crate::flow_json::check).schema("flowcheck/1").examples(&["mzlab-cli flow check plan.json --json"]),
    Entry::new("flow steps", "mzlab-cli flow steps [--json] [--compact] [--id ID[@V]]   the Flow steps of this program, native or page only", crate::steps_cmd::run).examples(&["mzlab-cli flow steps --compact", "mzlab-cli flow steps --json --id features.detect"]),
    Entry::new("flow report", "mzlab-cli flow report RUNDIR [--out FILE]   a run's report", report).exits(&[0, 2, 3, 4, 6]).schema("run-report/1"),
    Entry::new("flow record", "mzlab-cli flow record RUNDIR --roles ROLES.json [--out FILE]   the mzrun/2 record of a run (a role without sha256 takes it from the run inputs)", crate::flow_mzrun::record).schema("mzrun/2"),
    Entry::new("flow verify", "mzlab-cli flow verify RECORD   whether the fingerprints of an mzrun/1 record match its files", crate::flow_mzrun::check).schema("verify/1"),
    Entry::new("flow command", "mzlab-cli flow command PLAN.json [--json]   the rerun script", command).schema("flowcommand/1").exits(&[0, 2, 3, 4]),
];
