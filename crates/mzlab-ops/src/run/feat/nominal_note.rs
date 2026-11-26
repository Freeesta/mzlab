//! The nominal-resolution hint of one unit-axis file as data: the numbers of `mzlab_feat::nominal_hint` with the cuts they are judged by, the one shape of
//! the `nominal` member of `estimates/1` and of the report choice. The cuts are `hint_between` and `hint_split` of `features.estimate`. Nothing is applied.

use mzlab_core::error::Result;
use mzlab_core::flow::value_text;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Value;
use mzlab_feat::nominal_hint::{signature, Cuts, Signature, CUT_BETWEEN, CUT_SPLIT};
use mzlab_feat::recipe::Recipe;

/// Names of the cuts among the parameters of `features.estimate`.
pub const CUT_NAMES: [&str; 2] = ["hint_between", "hint_split"];

/// The cuts the plan writes (`given`, descriptor names) and the parameters left for the recipe; a cut outside 0..1 is the default.
pub fn split_cuts(given: &[(String, Value)]) -> (Cuts, Vec<(String, Value)>) {
    let num = |name: &str, default: f64| {
        given
            .iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| v.f64())
            .filter(|x| (0.0..=1.0).contains(x))
            .unwrap_or(default)
    };
    let cuts = Cuts {
        between: num(CUT_NAMES[0], CUT_BETWEEN),
        split: num(CUT_NAMES[1], CUT_SPLIT),
    };
    let rest = given
        .iter()
        .filter(|(k, _)| !CUT_NAMES.contains(&k.as_str()))
        .cloned()
        .collect();
    (cuts, rest)
}

fn n(x: f64) -> Value {
    Value::Num(x)
}

/// The `nominal` member of `estimates/1`: counts, shares, the heights and reaches compared and the cuts.
pub fn value_of(s: &Signature) -> Value {
    let f = |k: &str, v: Value| (k.to_string(), v);
    Value::Obj(vec![
        f("rows", n(s.rows as f64)),
        f("between", n(s.between as f64)),
        f("between_share", n(s.between_share())),
        f("height_now", n(s.height_now)),
        f("height_nominal", n(s.height_nominal)),
        f("split", n(s.split as f64)),
        f("split_share", n(s.split_share())),
        f("reach_now", n(s.reach_now)),
        f("reach_nominal", n(s.reach_nominal)),
        f("cut_between", n(s.cuts.between)),
        f("cut_split", n(s.cuts.split)),
        f("hint", Value::Bool(s.hint())),
    ])
}

/// The signature of `index` under `recipe` as the `nominal` member, `None` where there is nothing to say (high resolution, profile in force).
pub fn member_of<R: ReadAt>(
    index: &mut Index<R>,
    recipe: &Recipe,
    cuts: Cuts,
) -> Result<Option<Value>> {
    Ok(signature(index, recipe, cuts)?.map(|s| value_of(&s)))
}

/// The sentence of the report choice for a file that shows the signature: the numbers and the way to the profile.
pub fn sentence(s: &Signature) -> String {
    let pct = |x: f64| value_text(&n((x * 1000.0).round() / 10.0));
    format!(
        "{} of {} candidate rows lie between height {} (the profile) and {} (in force) and {} are one ion split across the grids at a reach of {} Da that {} Da would merge ({}%, {}%; cuts {}%, {}%): {{\"preset\": \"nominal\"}} applies the profile; nothing is applied by itself",
        s.between,
        s.rows,
        value_text(&n(s.height_nominal)),
        value_text(&n(s.height_now)),
        s.split,
        value_text(&n(s.reach_now)),
        value_text(&n(s.reach_nominal)),
        pct(s.between_share()),
        pct(s.split_share()),
        pct(s.cuts.between),
        pct(s.cuts.split)
    )
}
