//! `mzlab-cli steps [--json] [--id ID[@V]]`: the registry of Flow steps of this very binary, so an agent reads the versions it will run.
//! The JSON is the `step/1` array of the exported `docs/schemi/steps-*-1.json`, each step with `executable_native` added at the end.

use mzlab_core::error::Error;
use mzlab_core::step::json::step_json;
use mzlab_core::step::Step;

use crate::flow::{executable, steps};
use crate::flow_json::{fail, Style};
use crate::Outcome;

const HELP: &str = "mzlab-cli steps: the steps a Flow plan can use, from the registry of this program.

usage: mzlab-cli steps [--json] [--id ID[@VERSION]]

  (no option)       one line per step: id@version, `native` when `flow run` executes it (else `page`: only the Flow page does), title
  --json            the step/1 descriptors as a JSON array, each with \"executable_native\" at the end
  --id ID[@V]       one step only (with --json one step/1 object); without @V the highest version
  --help            this text

exit codes: 0 done, 2 usage or an unknown step (err.flow.step.unknown)";

/// The step descriptor with `"executable_native"` after its last key.
fn with_native(s: &Step) -> String {
    let text = step_json(s);
    let body = text.strip_suffix('}').unwrap_or(&text);
    format!("{body},\"executable_native\":{}}}", executable(s.id))
}

/// `mzlab-cli steps ARGS...` (the arguments after `steps`).
pub(crate) fn run(args: Vec<String>) -> Outcome {
    let st = Style {
        json: args.iter().any(|a| a == "--json"),
        jsonl: false,
    };
    let usage = |msg: &str| {
        let err = Error::new("err.cli.usage").with("detail", msg);
        fail(st, 2, &err, msg, "mzlab-cli steps --help")
    };
    let mut id = None;
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--help" | "-h" => {
                return Outcome {
                    code: 0,
                    stdout: format!("{HELP}\n").into_bytes(),
                    stderr: String::new(),
                }
            }
            "--json" => {}
            "--id" => match it.next() {
                Some(v) => id = Some(v),
                None => return usage("--id needs a step id"),
            },
            s => return usage(&format!("unknown option {s}")),
        }
    }
    let reg = steps();
    let mut list: Vec<&Step> = reg.iter().collect();
    if let Some(want) = &id {
        let (name, version) = want
            .split_once('@')
            .map_or((want.as_str(), None), |(n, v)| (n, v.parse::<u32>().ok()));
        list.retain(|s| s.id == name && version.is_none_or(|v| s.version == v));
        list.sort_by_key(|s| s.version);
        let Some(last) = list.pop() else {
            let err = Error::new("err.flow.step.unknown").with("step", want);
            return fail(
                st,
                2,
                &err,
                &format!("unknown step {want}"),
                "mzlab-cli steps lists the steps",
            );
        };
        list = vec![last];
    }
    let stdout = if st.json {
        match (&id, list.as_slice()) {
            (Some(_), [one]) => format!("{}\n", with_native(one)),
            _ => format!(
                "[\n{}\n]\n",
                list.iter()
                    .map(|s| with_native(s))
                    .collect::<Vec<_>>()
                    .join(",\n")
            ),
        }
    } else {
        let lines: Vec<String> = list
            .iter()
            .map(|s| {
                format!(
                    "{}@{}  {}  {}\n",
                    s.id,
                    s.version,
                    if executable(s.id) { "native" } else { "page" },
                    s.title
                )
            })
            .collect();
        lines.concat()
    };
    Outcome {
        code: 0,
        stdout: stdout.into_bytes(),
        stderr: String::new(),
    }
}
