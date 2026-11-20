//! `mzlab-cli flow steps [--json] [--id ID[@V] [--template]]`: the registry of Flow steps of this very binary, so an agent reads the versions it will run.
//! The JSON is the `step/1` array of the exported `docs/schemi/steps-*-1.json`, each step with `executable_native` and `citation` added at the end.

use mzlab_core::error::Error;
use mzlab_core::json::Value;
use mzlab_core::jsonb::{text, write, Obj};
use mzlab_core::step::json::step_json;
use mzlab_core::step::Step;

use crate::flow::{executable, steps};
use crate::flow_json::{fail, Style};
use crate::Outcome;

const HELP: &str = "mzlab-cli flow steps: the steps a Flow plan can use, from the registry of this program.

usage: mzlab-cli flow steps [--json] [--compact] [--id ID[@VERSION] [--template]]

  (no option)       one line per step: id@version, `native` when `flow run` executes it (else `page`: only the Flow page does), title
  --json            the step/1 descriptors as a JSON array, each with \"executable_native\" and \"citation\" at the end
  --compact         steps-compact/1: per step the id, version, title, ports with their types, parameter names, algorithm ids and
                    executable_native, no prose (a few KB; --json is the full descriptors)
  --id ID[@V]       one step only (with --json one step/1 object); without @V the highest version
  --template        with --id: not the descriptor but a one-step mzflow/2 plan to edit (a file input per required port, every parameter at its default; `flow check` reads it)
  --help            this text

exit codes: 0 done, 2 usage, 5 unknown step (err.flow.step.unknown)";

/// The step descriptor with `"executable_native"` after its last key.
fn with_native(s: &Step) -> String {
    let text = step_json(s);
    let body = text.strip_suffix('}').unwrap_or(&text);
    format!(
        "{body},\"executable_native\":{},{}}}",
        executable(s.id),
        crate::citation::member()
    )
}

/// The `steps-compact/1` document: what an agent needs to wire a plan, without the parameter prose.
fn compact_json(list: &[&Step]) -> String {
    let ports = |ps: &[mzlab_core::step::Port]| {
        Value::Arr(
            ps.iter()
                .map(|p| {
                    let o = Obj::new()
                        .put("name", text(p.name))
                        .put("type", text(p.ty.name()));
                    let o = if p.many { o.b("many", true) } else { o };
                    (if p.optional { o.b("optional", true) } else { o }).done()
                })
                .collect(),
        )
    };
    let steps = list
        .iter()
        .map(|s| {
            Obj::new()
                .put("id", text(s.id))
                .i("version", i64::from(s.version))
                .put("title", text(s.title))
                .put("in", ports(&s.ins))
                .put("out", ports(&s.outs))
                .put(
                    "params",
                    Value::Arr(s.params.iter().map(|p| text(p.name)).collect()),
                )
                .put(
                    "algorithms",
                    Value::Arr(s.algorithms.iter().map(|a| text(a.id)).collect()),
                )
                .b("executable_native", executable(s.id))
                .done()
        })
        .collect();
    let doc = Obj::new()
        .put("schema", text("steps-compact/1"))
        .put("citation", text(&crate::citation::text()))
        .put("steps", Value::Arr(steps))
        .done();
    write(&doc) + "\n"
}

/// `mzlab-cli flow steps ARGS...` (the arguments after `steps`).
pub(crate) fn run(args: Vec<String>) -> Outcome {
    let st = Style {
        json: args.iter().any(|a| a == "--json"),
        ..Style::default()
    };
    let usage = |msg: &str| {
        let err = Error::new("err.cli.usage").with("message", msg);
        fail(&err, msg, "mzlab-cli flow steps --help")
    };
    let mut id = None;
    let (mut compact, mut template) = (false, false);
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--json" => {}
            "--compact" => compact = true,
            "--template" => template = true,
            "--id" => match it.next() {
                Some(v) => id = Some(v),
                None => return usage("--id needs a step id"),
            },
            s => return usage(&format!("unknown option {s}")),
        }
    }
    if template && id.is_none() {
        return usage("--template needs --id STEP");
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
                &err,
                &format!("unknown step {want}"),
                "mzlab-cli flow steps lists the steps",
            );
        };
        list = vec![last];
    }
    if let (true, [one]) = (template, list.as_slice()) {
        return crate::flow_template::template(one);
    }
    let stdout = if compact {
        compact_json(&list)
    } else if st.json {
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
    Outcome::ok(stdout.into_bytes())
}
