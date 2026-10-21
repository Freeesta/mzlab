//! Plan checks the agent runs itself: `flow_check` and `flow_propose_plan` take a plan `mzflow/2` as an argument and the problems of
//! `flow::check` go back to the model as a tool result. Pure; the registry is the one of the program.

use mzlab_core::flow::parse;
use mzlab_core::flow::{check, Platform, Problem};
use mzlab_core::json::{Out, Value};
use mzlab_core::step::Step;

use crate::tools::value;

fn problem_json(o: &mut Out, p: &Problem) {
    o.raw("{\"error_key\":").str(p.error_key);
    o.raw(",\"instance\":").str(&p.instance);
    o.raw(",\"port\":").str(&p.port).raw(",\"params\":{");
    for (k, (name, v)) in p.params.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .str(name)
            .raw(":")
            .str(v);
    }
    o.raw("}}");
}

/// The problems of the plan in `args.plan` as `{"problems":[...]}` text; `Err` is the error key when the plan cannot be read at all.
pub fn problems(
    args: &Value,
    registry: &[Step],
    platform: Platform,
) -> Result<(usize, String), String> {
    let Some(plan) = args.get("plan") else {
        return Err("err.llm.tool.args".into());
    };
    let mut text = Out::default();
    value(&mut text, plan);
    let plan = parse(text.0.as_bytes()).map_err(|e| e.key.to_string())?;
    let found = check(&plan, registry, platform);
    let mut o = Out::default();
    o.raw("{\"problems\":[");
    for (k, p) in found.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" });
        problem_json(&mut o, p);
    }
    o.raw("]}");
    Ok((found.len(), o.0))
}
