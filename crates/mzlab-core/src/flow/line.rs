//! The short command line of a plan (`mzlab-cli flow run plan.json --in … --out … --set INSTANCE.NAME=VALUE …`) and its way back: the `--set` of a line applied to
//! a plan, typed by the descriptors. One code for `mzlab-cli`, the site (`api/flow/line`) and the desktop app, so a line gives the same plan everywhere.

use super::{parse, plan_json, Plan};
use crate::error::{Error, Result};
use crate::json::{self, Value};
use crate::step::registry_from_value;
use crate::step::{Kind, Step};

use super::command::FOLDER_PLACEHOLDER;

/// The value of `text` read with the type of `kind`.
fn typed(kind: &Kind, text: &str) -> Option<Value> {
    let num = |s: &str| s.trim().parse::<f64>().ok().map(Value::Num);
    match kind {
        Kind::Number | Kind::Integer => num(text),
        Kind::Bool => match text {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            _ => None,
        },
        Kind::Text => Some(Value::Str(text.into())),
        Kind::Pair(_) => {
            let (a, b) = text.split_once(',')?;
            Some(Value::Arr(vec![num(a)?, num(b)?]))
        }
        Kind::Choice(list) => {
            let as_num = list.iter().any(|v| matches!(v, Value::Num(_)));
            if as_num {
                num(text)
            } else {
                Some(Value::Str(text.into()))
            }
        }
    }
}

/// The instance `who` names: its id, or the id of a step the plan has once.
fn instance_index(plan: &Plan, who: &str) -> std::result::Result<usize, String> {
    if let Some(k) = plan.steps.iter().position(|s| s.id == who) {
        return Ok(k);
    }
    let of_step: Vec<usize> = (0..plan.steps.len())
        .filter(|&k| plan.steps[k].step == who)
        .collect();
    match of_step.as_slice() {
        [k] => Ok(*k),
        [] => {
            let ids: Vec<&str> = plan.steps.iter().map(|s| s.id.as_str()).collect();
            Err(format!(
                "--set {who}: no instance or step of that name (instances: {})",
                ids.join(", ")
            ))
        }
        _ => Err(format!(
            "--set {who}: the plan has more than one instance of that step; use the instance id"
        )),
    }
}

/// Applies one `--set` (`INSTANCE.NAME=VALUE`) to `plan`, its value typed by the descriptor.
pub fn apply_set(plan: &mut Plan, reg: &[Step], set: &str) -> std::result::Result<(), String> {
    let bad = || format!("--set {set}: expected INSTANCE.NAME=VALUE");
    let (lhs, text) = set.split_once('=').ok_or_else(bad)?;
    let (who, name) = lhs.rsplit_once('.').ok_or_else(bad)?;
    let k = instance_index(plan, who)?;
    let inst = &mut plan.steps[k];
    match name {
        "algorithm" => inst.algorithm = Some(text.into()),
        "preset" => inst.preset = Some(text.into()),
        _ => {
            let step = reg
                .iter()
                .find(|s| s.id == inst.step && s.version == inst.version);
            let step = step.ok_or(format!("--set {set}: unknown step {}", inst.step_ref()))?;
            let alg = inst.algorithm.as_deref().or(step.default_algorithm);
            let param = step
                .param(name)
                .or_else(|| alg.and_then(|a| step.algorithm_param(a, name)));
            let Some(param) = param else {
                let own = step.params.iter().map(|p| p.name);
                let of_alg = step
                    .algorithms
                    .iter()
                    .filter(|a| Some(a.id) == alg)
                    .flat_map(|a| a.params.iter().map(|p| p.name));
                let names: Vec<&str> = own.chain(of_alg).collect();
                return Err(format!(
                    "--set {set}: {} has no parameter {name} (parameters: {})",
                    inst.id,
                    names.join(", ")
                ));
            };
            let v = typed(&param.kind, text)
                .ok_or(format!("--set {set}: {text} is not a value of {name}"))?;
            inst.params.retain(|(n, _)| n != name);
            inst.params.push((name.into(), v));
        }
    }
    Ok(())
}

/// Applies `sets` in the order of the CLI: the algorithms and presets first (they decide which parameters exist), then the rest. The error is `err.cli.set`
/// with the `set` that failed and the `message` the CLI prints.
pub fn apply_sets(plan: &mut Plan, reg: &[Step], sets: &[String]) -> Result<()> {
    let (setting, plain): (Vec<&String>, Vec<&String>) = sets
        .iter()
        .partition(|s| s.contains(".algorithm=") || s.contains(".preset="));
    for s in setting.into_iter().chain(plain) {
        apply_set(plan, reg, s).map_err(|message| {
            Error::new("err.cli.set")
                .with("set", s)
                .with("message", message)
        })?;
    }
    Ok(())
}

/// `text` as one word of a POSIX shell: bare when it is plain, else in single quotes.
fn quoted(text: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c);
    if !text.is_empty() && text.chars().all(plain) {
        text.into()
    } else {
        format!("'{}'", text.replace('\'', "'\\''"))
    }
}

/// The `--set` value text of `v`, as `typed` reads it back; None for a value a line cannot hold (an object, a list that is not a pair of numbers).
fn value_word(v: &Value) -> Option<String> {
    let num = |v: &Value| v.f64().map(|n| super::value_text(&Value::Num(n)));
    match v {
        Value::Num(_) => num(v),
        Value::Bool(b) => Some(b.to_string()),
        Value::Str(s) => Some(s.clone()),
        Value::Arr(a) if a.len() == 2 => Some(format!("{},{}", num(&a[0])?, num(&a[1])?)),
        _ => None,
    }
}

/// The short line of `plan`: one `--set` for the algorithm and the preset an instance names and for every parameter whose value is not the default of the
/// descriptor, in the order of the plan.
pub fn line(plan: &Plan, reg: &[Step]) -> String {
    let mut s = format!(
        "mzlab-cli flow run plan.json --in {} --out mzlab-out",
        quoted(FOLDER_PLACEHOLDER)
    );
    let mut set = |who: &str, name: &str, value: &str| {
        s += &format!(" --set {}", quoted(&format!("{who}.{name}={value}")));
    };
    for inst in &plan.steps {
        let step = reg
            .iter()
            .find(|d| d.id == inst.step && d.version == inst.version);
        if let Some(a) = &inst.algorithm {
            set(&inst.id, "algorithm", a);
        }
        if let Some(p) = &inst.preset {
            set(&inst.id, "preset", p);
        }
        for (name, v) in &inst.params {
            let alg = inst
                .algorithm
                .as_deref()
                .or(step.and_then(|d| d.default_algorithm));
            let param = step.and_then(|d| {
                d.param(name)
                    .or_else(|| alg.and_then(|a| d.algorithm_param(a, name)))
            });
            if param.and_then(|p| p.default.as_ref()) == Some(v) {
                continue;
            }
            if let Some(w) = value_word(v) {
                set(&inst.id, name, &w);
            }
        }
    }
    s
}

/// The words of `text` as a POSIX shell reads them: single quotes, double quotes (a backslash escapes `"`, `\` and `$` inside) and backslash outside. None for an
/// open quote.
fn words(text: &str) -> Option<Vec<String>> {
    let (mut out, mut cur, mut open) = (Vec::new(), String::new(), false);
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\'' => {
                open = true;
                loop {
                    match it.next()? {
                        '\'' => break,
                        x => cur.push(x),
                    }
                }
            }
            '"' => {
                open = true;
                loop {
                    match it.next()? {
                        '"' => break,
                        '\\' if matches!(it.peek(), Some('"' | '\\' | '$')) => cur.push(it.next()?),
                        x => cur.push(x),
                    }
                }
            }
            '\\' => {
                open = true;
                cur.push(it.next()?);
            }
            c if c.is_whitespace() => {
                if open {
                    out.push(std::mem::take(&mut cur));
                    open = false;
                }
            }
            c => {
                open = true;
                cur.push(c);
            }
        }
    }
    if open {
        out.push(cur);
    }
    Some(out)
}

/// The `--set` words of a line `mzlab-cli flow run …`. The other options of `flow run` are read and ignored (the page runs the plan itself); an unknown option or
/// a line that is not a `flow run` is `err.cli.line`.
pub fn sets_of(text: &str) -> Result<Vec<String>> {
    let bad = |why: &str| Error::new("err.cli.line").with("message", why);
    let w = words(text).ok_or_else(|| bad("unclosed quote"))?;
    if w.len() < 3 || w[..3] != ["mzlab-cli", "flow", "run"] {
        return Err(bad("the line starts with: mzlab-cli flow run"));
    }
    let (mut sets, mut it) = (Vec::new(), w.into_iter().skip(3).peekable());
    while let Some(x) = it.next() {
        let mut value = || it.next().ok_or_else(|| bad(&format!("{x} needs a value")));
        match x.as_str() {
            "--set" => sets.push(value()?),
            "--out" | "--workers" | "--budget-mb" | "--progress" => {
                value()?;
            }
            "--in" => {
                value()?;
                while it.next_if(|n| !n.starts_with("--")).is_some() {}
            }
            "--dry-run" | "--json" | "--fresh" | "--jsonl" => {}
            s if s.starts_with("--") => return Err(bad(&format!("unknown option {s}"))),
            _ => {}
        }
    }
    Ok(sets)
}

/// The route `api/flow/line`: `{"plan": <mzflow/2 object or text>, "registry": <step/1 array>, "line"?: text}`. Without `line` the answer is the short line of the plan; with
/// it, the plan text (canonical) with the `--set` of the line applied. `fallback` is the registry when the body has none (the desktop app holds it).
pub fn route(body: &str, fallback: Option<&[Step]>) -> Result<String> {
    let v = json::parse(body.as_bytes())?;
    let plan = match v.get("plan") {
        Some(Value::Str(t)) => parse(t.as_bytes())?,
        Some(p) => parse(super::value_text(p).as_bytes())?,
        None => return Err(Error::new("err.flow.plan")),
    };
    let owned;
    let reg = match (v.get("registry"), fallback) {
        (Some(r), _) => {
            owned = registry_from_value(r)?;
            &owned[..]
        }
        (None, Some(f)) => f,
        (None, None) => return Err(Error::new("err.flow.plan")),
    };
    match v.get("line").and_then(Value::str) {
        None => Ok(line(&plan, reg)),
        Some(text) => {
            let mut plan = plan;
            apply_sets(&mut plan, reg, &sets_of(text)?)?;
            Ok(plan_json(&plan))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::Param;

    fn reg() -> Vec<Step> {
        vec![Step {
            params: vec![
                Param::number("ppm", "Tolerance").default_num(5.0),
                Param::boolean("flag", "Flag").default_bool(false),
                Param::text("name", "Name"),
            ],
            ..Step::core("graph.make", "Graph", "Makes a graph.")
        }]
    }

    const PLAN: &str = r#"{"format":"mzflow/2","inputs":[],"steps":[{"id":"graph","step":"graph.make@1","params":{"ppm":7,"flag":false,"name":"a b'c"}}]}"#;

    #[test]
    fn the_line_has_one_set_per_value_that_is_not_the_default() {
        let plan = parse(PLAN.as_bytes()).unwrap();
        let l = line(&plan, &reg());
        assert!(l.starts_with("mzlab-cli flow run plan.json --in "));
        assert!(l.contains(" --set graph.ppm=7"));
        assert!(l.contains(" --set 'graph.name=a b'\\''c'"));
        assert!(!l.contains(" --set graph.flag=false"));
    }

    #[test]
    fn a_line_gives_the_plan_the_cli_set_gives() {
        let plan = parse(PLAN.as_bytes()).unwrap();
        let l = line(&plan, &reg());
        let mut back = parse(PLAN.as_bytes()).unwrap();
        back.steps[0].params.clear();
        apply_sets(&mut back, &reg(), &sets_of(&l).unwrap()).unwrap();
        let mut one = parse(PLAN.as_bytes()).unwrap();
        one.steps[0].params.retain(|(n, _)| n != "flag");
        assert_eq!(plan_json(&back), plan_json(&one));
        // a changed value goes the same way as `--set graph.ppm=9`
        let mut a = parse(PLAN.as_bytes()).unwrap();
        apply_sets(
            &mut a,
            &reg(),
            &sets_of("mzlab-cli flow run plan.json --in x --set graph.ppm=9").unwrap(),
        )
        .unwrap();
        let mut b = parse(PLAN.as_bytes()).unwrap();
        apply_set(&mut b, &reg(), "graph.ppm=9").unwrap();
        assert_eq!(plan_json(&a), plan_json(&b));
    }

    #[test]
    fn a_wrong_set_is_the_error_of_the_cli() {
        let mut plan = parse(PLAN.as_bytes()).unwrap();
        let e = apply_sets(&mut plan, &reg(), &["graph.ppm=x".into()]).unwrap_err();
        assert_eq!(e.key, "err.cli.set");
        assert!(e.params.contains(&(
            "message",
            "--set graph.ppm=x: x is not a value of ppm".into()
        )));
        assert_eq!(sets_of("ls -l").unwrap_err().key, "err.cli.line");
        assert_eq!(
            sets_of("mzlab-cli flow run p --bogus").unwrap_err().key,
            "err.cli.line"
        );
        assert_eq!(
            sets_of("mzlab-cli flow run p --set 'a").unwrap_err().key,
            "err.cli.line"
        );
    }

    #[test]
    fn the_route_builds_the_line_and_applies_it() {
        let body = format!(r#"{{"plan":{PLAN},"registry":[]}}"#);
        assert!(route(&body, Some(&reg()))
            .unwrap()
            .contains("--set graph.ppm=7"));
        let body =
            format!(r#"{{"plan":{PLAN},"line":"mzlab-cli flow run plan.json --set graph.ppm=9"}}"#);
        assert!(route(&body, Some(&reg())).unwrap().contains("\"ppm\":9"));
    }
}
