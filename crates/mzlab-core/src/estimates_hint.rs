//! The `hints` of `estimate/1`: the `nominal` member of each file's `estimates/1` put together into the `--set` line that names the nominal profile and the
//! note that says why. Nothing is applied; the entry is never one of the `params` that `--emit-set` turns into settings.

use super::{instance_of, median, plan_steps, File};
use crate::json::Value;
use crate::jsonb::{int, msg, text, Obj};

/// The hint of the `nominal` profile (`hints`) and its note: the taken files whose `estimates/1` carries the `nominal` member with `hint` true. One entry per
/// instance of `features.detect` in the plan (or the default instance without a plan); the profile is not applied, the entry only names the `--set` line.
pub(super) fn nominal_hints(taken: &[&File], plan: Option<&Value>) -> (Vec<Value>, Vec<Value>) {
    let members: Vec<&Value> = taken.iter().filter_map(|f| f.doc?.get("nominal")).collect();
    let hinted = members
        .iter()
        .filter(|m| m.get("hint").and_then(Value::bool) == Some(true))
        .count();
    if hinted == 0 {
        return (Vec::new(), Vec::new());
    }
    let med = |k: &str| {
        let mut v: Vec<f64> = members
            .iter()
            .filter_map(|m| m.get(k).and_then(Value::f64))
            .collect();
        crate::sortc::by(&mut v, |a, b| a.total_cmp(b));
        median(&v)
    };
    let (between, split) = (med("between_share"), med("split_share"));
    let step = "features.detect";
    let instances: Vec<(String, Option<Value>)> = match plan.map(plan_steps) {
        Some(all) => all
            .into_iter()
            .filter(|(_, s, _)| s == step)
            .map(|(id, _, p)| (id, p))
            .collect(),
        None => vec![(instance_of(step).to_string(), None)],
    };
    let hints = instances
        .into_iter()
        .map(|(id, now)| {
            Obj::new()
                .put("set", text(&format!("{id}.preset=nominal")))
                .put("instance", text(&id))
                .put("step", text(step))
                .put("param", text("preset"))
                .put("value", text("nominal"))
                .put(
                    "current",
                    now.and_then(|p| p.get("preset").cloned())
                        .unwrap_or(Value::Null),
                )
                .put("from", text("nominal"))
                .put(
                    "rule",
                    text("the share of candidate rows between the nominal height and the height in force, or of ions split across the two grids, reaches its cut in these files (hint_between, hint_split of features.estimate; the numbers are under files[].nominal); the profile is not applied"),
                )
                .i("files", hinted as i64)
                .i("of", members.len() as i64)
                .n("between_share", between)
                .n("split_share", split)
                .done()
        })
        .collect();
    let note = msg(
        "note.estimate.nominal",
        &[
            ("files", int(hinted as i64)),
            ("of", int(members.len() as i64)),
            ("between_pct", int(libm::round(between * 100.0) as i64)),
            ("split_pct", int(libm::round(split * 100.0) as i64)),
        ],
    );
    (hints, vec![note])
}
