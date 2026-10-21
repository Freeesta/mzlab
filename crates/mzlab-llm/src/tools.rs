//! The catalog of tools an assistant may call (`strumenti/1`). A few fixed meta-tools whose arguments are generated from the step registry, so a new
//! step or parameter reaches the catalog without a line written here: `flow_add_step.step` is the enum of every `id@version`, `flow_set_param` has one
//! branch per step with the names `--set` accepts (algorithm and preset included) and the value schema of each parameter.

use std::collections::BTreeSet;

use mzlab_core::budget::sha256_hex;
use mzlab_core::json::{Out, Value};
use mzlab_core::step::json::registry_json;
use mzlab_core::step::{Kind, Param, Step};

/// What a tool may do, read by the policy of the autonomy level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Read,
    Propose,
    Edit,
    Run,
    Ui,
}

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Class::Read => "read",
            Class::Propose => "propose",
            Class::Edit => "edit",
            Class::Run => "run",
            Class::Ui => "ui",
        }
    }
}

/// One tool of the catalog; `input_schema` is JSON text (a JSON Schema, draft 2020-12).
#[derive(Clone, Debug, PartialEq)]
pub struct Tool {
    pub name: &'static str,
    pub class: Class,
    pub description: &'static str,
    pub input_schema: String,
}

/// A view-only action of the page the assistant may ask for: `action` is the id in the registry `azioni/1`, `args` the properties of its call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiTool {
    pub name: &'static str,
    pub action: &'static str,
    pub description: &'static str,
    /// Properties of the arguments as (name, JSON Schema text); all are required.
    pub args: &'static [(&'static str, &'static str)],
}

/// The closed list of `ui_show` targets; nothing else of the interface is reachable.
pub const UI_TOOLS: &[UiTool] = &[
    UiTool {
        name: "show_xic",
        action: "spec.xic",
        description: "Open the XIC of an m/z.",
        args: &[("mz", "{\"type\":\"number\",\"exclusiveMinimum\":0}")],
    },
    UiTool {
        name: "show_spectrum_at_rt",
        action: "chrom.specAtRt",
        description: "Show the spectrum at a retention time.",
        args: &[("rt_min", "{\"type\":\"number\",\"minimum\":0}")],
    },
    UiTool {
        name: "show_row",
        action: "table.showRow",
        description: "Scroll a result table to a row and highlight it.",
        args: &[
            ("table", "{\"type\":\"string\"}"),
            ("row", "{\"type\":\"integer\",\"minimum\":0}"),
        ],
    },
    UiTool {
        name: "select_node",
        action: "flow.selectNode",
        description: "Select a step of the Flow plan.",
        args: &[("instance", "{\"type\":\"string\"}")],
    },
];

const OBJ: &str = "{\"type\":\"object\",\"additionalProperties\":false";
const ANY_OBJECT: &str = "{\"type\":\"object\"}";
const INSTANCE: &str = "{\"type\":\"string\",\"description\":\"Instance id in the plan.\"}";

fn value(o: &mut Out, v: &Value) {
    match v {
        Value::Null => {
            o.raw("null");
        }
        Value::Bool(b) => {
            o.raw(if *b { "true" } else { "false" });
        }
        Value::Num(n) => {
            o.num(*n);
        }
        Value::Str(s) => {
            o.str(s);
        }
        Value::Arr(a) => {
            o.raw("[");
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    o.raw(",");
                }
                value(o, x);
            }
            o.raw("]");
        }
        Value::Obj(m) => {
            o.raw("{");
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    o.raw(",");
                }
                o.str(k).raw(":");
                value(o, x);
            }
            o.raw("}");
        }
    }
}

/// The schema of the value of `p` (type, limits, default, description).
fn param_schema(p: &Param) -> String {
    let mut o = Out::default();
    match &p.kind {
        Kind::Number => o.raw("{\"type\":\"number\""),
        Kind::Integer => o.raw("{\"type\":\"integer\""),
        Kind::Bool => o.raw("{\"type\":\"boolean\""),
        Kind::Text => o.raw("{\"type\":\"string\""),
        Kind::Pair(min) => o
            .raw("{\"type\":\"array\",\"items\":{\"type\":\"number\",\"minimum\":")
            .num(*min)
            .raw("},\"minItems\":2,\"maxItems\":2"),
        Kind::Choice(v) => {
            o.raw("{\"enum\":");
            value(&mut o, &Value::Arr(v.clone()));
            &mut o
        }
    };
    for (k, v) in [("minimum", p.min), ("maximum", p.max)] {
        if let Some(v) = v {
            o.raw(",").str(k).raw(":").num(v);
        }
    }
    if let Some(d) = &p.default {
        o.raw(",\"default\":");
        value(&mut o, d);
    }
    o.raw(",\"description\":").str(p.doc.unwrap_or(p.title));
    o.raw("}");
    o.0
}

fn strs(items: impl IntoIterator<Item = String>) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, s) in items.into_iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(&s);
    }
    o.raw("]");
    o.0
}

fn step_refs(steps: &[Step]) -> String {
    strs(steps.iter().map(|s| format!("{}@{}", s.id, s.version)))
}

/// An object schema with the given (name, schema) properties; the names in `required` must be among them.
fn object(props: &[(&str, String)], required: &[&str]) -> String {
    let mut o = Out::default();
    o.raw(OBJ).raw(",\"properties\":{");
    for (i, (k, v)) in props.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(k).raw(":").raw(v);
    }
    o.raw("},\"required\":")
        .raw(&strs(required.iter().map(|s| s.to_string())));
    o.raw("}");
    o.0
}

fn any_of(mut schemas: Vec<String>) -> String {
    let mut seen = BTreeSet::new();
    schemas.retain(|s| seen.insert(s.clone()));
    match schemas.len() {
        1 => schemas.remove(0),
        _ => format!("{{\"anyOf\":[{}]}}", schemas.join(",")),
    }
}

/// The names `--set` accepts for `step` (common parameters, the parameters of every algorithm, `algorithm`, `preset`) with the schema of each value.
pub fn settable(step: &Step) -> Vec<(String, String)> {
    let mut names: Vec<(String, Vec<String>)> = Vec::new();
    let mut add = |name: &str, schema: String| match names.iter_mut().find(|(n, _)| n == name) {
        Some((_, v)) => v.push(schema),
        None => names.push((name.to_string(), vec![schema])),
    };
    if !step.algorithms.is_empty() {
        let ids = strs(step.algorithms.iter().map(|a| a.id.to_string()));
        add("algorithm", format!("{{\"enum\":{ids}}}"));
    }
    if !step.presets.is_empty() {
        let ids = strs(step.presets.iter().map(|p| p.id.to_string()));
        add("preset", format!("{{\"enum\":{ids}}}"));
    }
    for p in &step.params {
        add(p.name, param_schema(p));
    }
    for p in step.algorithms.iter().flat_map(|a| a.params.iter()) {
        add(p.name, param_schema(p));
    }
    names.into_iter().map(|(n, s)| (n, any_of(s))).collect()
}

fn set_param_schema(steps: &[Step]) -> String {
    let mut o = Out::default();
    o.raw(OBJ)
        .raw(",\"properties\":{\"instance\":")
        .raw(INSTANCE);
    o.raw("},\"required\":[\"instance\"],\"oneOf\":[");
    let mut first = true;
    for s in steps {
        let set = settable(s);
        if set.is_empty() {
            continue;
        }
        if !first {
            o.raw(",");
        }
        first = false;
        let names = strs(set.iter().map(|(n, _)| n.clone()));
        let values = any_of(set.into_iter().map(|(_, v)| v).collect());
        let sref = format!("{}@{}", s.id, s.version);
        o.raw("{\"properties\":{\"step\":{\"const\":").str(&sref);
        o.raw("},\"name\":{\"enum\":").raw(&names);
        o.raw("},\"value\":").raw(&values);
        o.raw("},\"required\":[\"step\",\"name\",\"value\"]}");
    }
    o.raw("]}");
    o.0
}

fn ui_schema(ui: &[UiTool]) -> String {
    let mut o = Out::default();
    o.raw("{\"oneOf\":[");
    for (i, t) in ui.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        let props: Vec<(&str, String)> = t.args.iter().map(|(k, v)| (*k, v.to_string())).collect();
        let req: Vec<&str> = t.args.iter().map(|(k, _)| *k).collect();
        o.raw("{\"type\":\"object\",\"additionalProperties\":false,\"properties\":{\"show\":{\"const\":");
        o.str(t.name).raw("},\"args\":").raw(&object(&props, &req));
        o.raw("},\"required\":[\"show\",\"args\"],\"description\":")
            .str(t.description);
        o.raw(",\"x-action\":").str(t.action).raw("}");
    }
    o.raw("]}");
    o.0
}

fn tool(name: &'static str, class: Class, description: &'static str, input_schema: String) -> Tool {
    Tool {
        name,
        class,
        description,
        input_schema,
    }
}

fn s(desc: &str) -> String {
    format!("{{\"type\":\"string\",\"description\":{}}}", quote(desc))
}

fn quote(t: &str) -> String {
    let mut o = Out::default();
    o.str(t);
    o.0
}

/// The tools of the catalog, in a fixed order. `executable` says whether a step id has a native executor.
pub fn catalog(steps: &[Step], ui: &[UiTool], executable: &dyn Fn(&str) -> bool) -> Vec<Tool> {
    use Class::*;
    let refs = step_refs(steps);
    let step_enum = format!("{{\"enum\":{refs}}}");
    let none = object(&[], &[]);
    let native = strs(
        steps
            .iter()
            .filter(|x| executable(x.id))
            .map(|x| x.id.to_string()),
    );
    let list_steps = format!(
        "{},\"x-executable-native\":{native}}}",
        &none[..none.len() - 1]
    );
    let plan = |d: &str| {
        (
            "plan",
            format!("{{\"type\":\"object\",\"description\":{}}}", quote(d)),
        )
    };
    let run = object(
        &[(
            "executor",
            "{\"enum\":[\"native\",\"web\"],\"description\":\"Where to run; the default is the one available.\"}".to_string(),
        )],
        &[],
    );
    let read = object(
        &[
            ("instance", INSTANCE.to_string()),
            ("port", s("Output port name.")),
            (
                "rows",
                "{\"type\":\"integer\",\"minimum\":1,\"maximum\":50,\"default\":20}".into(),
            ),
        ],
        &["instance", "port"],
    );
    let add = object(
        &[("step", step_enum.clone()), ("label", s("Optional label."))],
        &["step"],
    );
    let chart = object(&[("spec", ANY_OBJECT.to_string())], &["spec"]);
    let code = object(
        &[
            ("language", "{\"enum\":[\"python\",\"r\",\"shell\"]}".into()),
            ("goal", s("What the script is for.")),
        ],
        &["language", "goal"],
    );
    vec![
        tool("flow_list_steps", Read, "List the steps of the Flow registry: id, title, ports, where each runs and whether it runs natively.", list_steps),
        tool("flow_describe_step", Read, "Describe one step: parameters, algorithms, presets and references.", object(&[("step", step_enum)], &["step"])),
        tool("flow_get_plan", Read, "Return the current plan in canonical form with its problems.", none.clone()),
        tool("flow_check", Read, "Check a proposed plan and return its problems.", object(&[plan("A plan mzflow/2.")], &["plan"])),
        tool("flow_propose_plan", Propose, "Propose a whole plan, shown to the user as a proposal.", object(&[plan("A plan mzflow/2."), ("note", s("Why this plan."))], &["plan"])),
        tool("flow_add_step", Edit, "Add a step to the plan; its inputs are wired automatically.", add),
        tool("flow_set_param", Edit, "Set a parameter of a step instance; the value follows the schema of the parameter.", set_param_schema(steps)),
        tool("flow_remove_step", Edit, "Remove a step from the plan (undoable; no data is touched).", object(&[("instance", INSTANCE.to_string())], &["instance"])),
        tool("flow_run", Run, "Run the plan with the available executor.", run),
        tool("flow_status", Read, "State of the current run.", none.clone()),
        tool("flow_read_result", Read, "Summary of one output of a run, at most 50 rows.", read),
        tool("data_summary", Read, "Summary of the files open in the Data view.", none),
        tool("chart_make", Propose, "Propose a chart from data of the session.", chart),
        tool("code_export", Propose, "Write a script for the user; it is never run.", code),
        tool("ui_show", Ui, "Show something in the interface; view-only actions.", ui_schema(ui)),
    ]
}

/// The catalog as the file `strumenti/1`.
pub fn catalog_json(steps: &[Step], ui: &[UiTool], executable: &dyn Fn(&str) -> bool) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"strumenti/1\",\"registry_sha256\":")
        .str(&sha256_hex(registry_json(steps).as_bytes()))
        .raw(",\"tools\":[\n");
    for (i, t) in catalog(steps, ui, executable).iter().enumerate() {
        if i > 0 {
            o.raw(",\n");
        }
        o.raw("{\"name\":")
            .str(t.name)
            .raw(",\"class\":")
            .str(t.class.name());
        o.raw(",\"description\":").str(t.description);
        o.raw(",\"input_schema\":").raw(&t.input_schema).raw("}");
    }
    o.raw("\n]}\n");
    o.0
}
