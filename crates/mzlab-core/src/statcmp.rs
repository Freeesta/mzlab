//! `compare/1`: every feature of a time-course table against its reference (t0) and its blank, to flag candidate transformation products. It wraps
//! `statref::reference_difference` (reference level, maximum, shape over time, pseudocount) and adds the blank's own ratio and time course. The class
//! (`candidate`, `blank_artefact`, ...) only sorts and filters: every row is written, nothing is removed, the user decides.

use crate::json::Value;
use crate::jsonb::{int, num, opt, text, Obj};
use crate::statcmp_plan::{check, choice, names_of, plan_of, table_of, Plan};
use crate::statistics::R;
use crate::statref::{reference_difference, spearman, RefIn, RefOpts};

pub use crate::statcmp_plan::plan;

/// The rules in the order they are tried; the first that matches decides (written to `choices` and to the schema).
pub const RULES: &str = "1 no_signal: max_treated <= pseudocount; 2 blank_artefact: ratio_blank < factor (blank_level) or blank_rho >= rho_min with the blank maximum above the pseudocount (blank_rises); 3 candidate: absent_in_reference or ratio_ref >= factor (above_reference); 4 decreasing: ratio_ref <= 1/factor (below_reference) or shape falls (falls); 5 unchanged";

pub const CLASSES: [&str; 5] = [
    "candidate",
    "blank_artefact",
    "decreasing",
    "unchanged",
    "no_signal",
];

/// What is known of one column: its name, and (from a `samples/1` document) its role and time in minutes.
pub struct CmpCol {
    pub name: String,
    pub role: Option<String>,
    pub time: Option<f64>,
}

/// The table: where it came from, one row per feature and one column per file.
pub struct CmpIn<'a> {
    pub path: &'a str,
    pub sha256: &'a str,
    pub ids: &'a [String],
    pub mz: &'a [f64],
    pub rt: &'a [f64],
    pub matrix: &'a [Vec<f64>],
    pub names: &'a [String],
    /// One entry per column from the samples document (`None` for a column it does not know); `None` when no samples document was given.
    pub meta: Option<&'a [Option<CmpCol>]>,
}

/// Options as the user gave them: a missing one takes its default and becomes a `choices` entry.
#[derive(Default)]
pub struct CmpOpts {
    pub ref_sel: String,
    pub blank_sel: String,
    pub quantity: Option<String>,
    pub factor: Option<f64>,
    pub pseudo: Option<f64>,
    pub floor: Option<f64>,
    pub rho_min: Option<f64>,
}

fn ratio(x: f64, y: f64, pc: f64) -> f64 {
    (x + pc) / (y + pc)
}

/// The class of one feature and why (rules in `RULES`).
#[allow(clippy::too_many_arguments)]
fn classify(
    max: f64,
    pc: f64,
    r_ref: f64,
    r_blank: f64,
    absent: bool,
    shape: Option<&str>,
    blank_max: f64,
    blank_rho: Option<f64>,
    factor: f64,
    rho_min: f64,
) -> (&'static str, Vec<&'static str>) {
    if max <= pc {
        return ("no_signal", vec!["below_pseudocount"]);
    }
    let mut why = Vec::new();
    if r_blank < factor {
        why.push("blank_level");
    }
    if blank_rho.is_some_and(|r| r >= rho_min) && blank_max > pc {
        why.push("blank_rises");
    }
    if !why.is_empty() {
        return ("blank_artefact", why);
    }
    if absent {
        why.push("absent_in_reference");
    }
    if r_ref >= factor {
        why.push("above_reference");
    }
    if !why.is_empty() {
        return ("candidate", why);
    }
    if r_ref <= 1.0 / factor {
        why.push("below_reference");
    }
    if shape == Some("falls") {
        why.push("falls");
    }
    if !why.is_empty() {
        return ("decreasing", why);
    }
    ("unchanged", why)
}

/// The `roles`, `params` and `choices` of the report: what was selected, the numbers used, and why the pseudocount is what it is.
fn header(inp: &CmpIn, o: &CmpOpts, p: &Plan, pc: f64, origin: &str) -> (Value, Value, Vec<Value>) {
    let sel = |s: &str, v: &[usize]| {
        Obj::new()
            .put("selector", text(s))
            .put("columns", names_of(inp, v))
            .done()
    };
    let roles_v = Obj::new()
        .put("reference", sel(&o.ref_sel, &p.reference))
        .put("blank", sel(&o.blank_sel, &p.blank))
        .put(
            "treated",
            Obj::new().put("columns", names_of(inp, &p.treated)).done(),
        )
        .done();
    let params = Obj::new()
        .put("quantity", text(&p.quantity))
        .n("factor", p.factor)
        .n("pseudocount", pc)
        .put("pseudocount_origin", text(origin))
        .n("floor", p.floor)
        .n("rho_min", p.rho_min)
        .done();
    let mut choices = p.choices.clone();
    let why = match origin {
        "given" => "given with --pseudocount",
        "blanks" => "3 x the median of the positive values of the blank columns, not below the floor",
        "reference" => "no positive blank value: 3 x the median of the positive values of the reference, not below the floor",
        _ => "no positive blank or reference value: the floor",
    };
    choices.push(choice("pseudocount", num(pc), why));
    (roles_v, params, choices)
}

/// `statref::reference_difference` on the roles of the plan (reference = "dark", blank = "blank", the rest "sample"): its report, the pseudocount and where it came from.
fn reference(inp: &CmpIn, o: &CmpOpts, p: &Plan) -> R<(Value, f64, String)> {
    let mut roles = vec![String::new(); inp.names.len()];
    // the reference columns are never a blank and never treated; the pseudocount reads the blanks
    p.reference.iter().for_each(|&i| roles[i] = "dark".into());
    p.blank.iter().for_each(|&i| roles[i] = "blank".into());
    p.treated.iter().for_each(|&i| roles[i] = "sample".into());
    let times: Vec<Option<f64>> = p.cols.iter().map(|c| c.time).collect();
    let refcols: Vec<i64> = p.reference.iter().map(|&i| i as i64).collect();
    let ropts = RefOpts {
        factor: p.factor,
        pseudo: o.pseudo,
        floor: p.floor,
        rho_min: p.rho_min,
        ..RefOpts::default()
    };
    let rin = RefIn {
        matrix: inp.matrix,
        roles: &roles,
        times: &times,
        groups: None,
        reference: Some(&refcols),
    };
    let diff = reference_difference(&rin, &ropts)?;
    let params = diff.get("params");
    let pc = params
        .and_then(|v| v.get("pseudo"))
        .and_then(Value::f64)
        .unwrap_or(p.floor);
    let origin = params
        .and_then(|v| v.get("pseudo_source"))
        .and_then(Value::str)
        .unwrap_or("floor")
        .to_string();
    Ok((diff, pc, origin))
}

/// The blank columns that have a time, and those times: the blank's own course.
struct BlankCourse {
    cols: Vec<usize>,
    times: Vec<f64>,
}

/// The row of feature `f` and its class, from the line `d` that `statref` gave.
fn feature_row(
    inp: &CmpIn,
    p: &Plan,
    bc: &BlankCourse,
    d: &Value,
    f: usize,
    pc: f64,
) -> (Value, &'static str) {
    let get = |k: &str| d.get(k).and_then(Value::f64).unwrap_or(0.0);
    let (ref_level, max_treated) = (get("ref_level"), get("treated_max"));
    let a = &inp.matrix[f];
    let blank_max = p.blank.iter().fold(0.0f64, |m, &i| m.max(a[i]));
    let by: Vec<f64> = bc.cols.iter().map(|&i| a[i]).collect();
    let blank_rho = if bc.times.len() >= 3 {
        spearman(&bc.times, &by)
    } else {
        None
    };
    let (r_ref, r_blank) = (
        ratio(max_treated, ref_level, pc),
        ratio(max_treated, blank_max, pc),
    );
    let absent = d
        .get("absent_in_reference")
        .and_then(Value::bool)
        .unwrap_or(false);
    let shape = d.get("shape").and_then(Value::str);
    let (class, why) = classify(
        max_treated,
        pc,
        r_ref,
        r_blank,
        absent,
        shape,
        blank_max,
        blank_rho,
        p.factor,
        p.rho_min,
    );
    let file = d
        .get("max_file")
        .and_then(Value::f64)
        .map_or(Value::Null, |i| text(&inp.names[i as usize]));
    let row = Obj::new()
        .put("id", text(&inp.ids[f]))
        .n("mz", inp.mz[f])
        .n("rt_min", inp.rt[f])
        .n("ref_level", ref_level)
        .n("blank_level", blank_max)
        .n("max_treated", max_treated)
        .put("max_file", file)
        .put("t_max_min", d.get("t_max").cloned().unwrap_or(Value::Null))
        .n("ratio_ref", r_ref)
        .n("ratio_blank", r_blank)
        .n("log2fc_ref", libm::log2(r_ref))
        .n("log2fc_blank", libm::log2(r_blank))
        .b("absent_in_reference", absent)
        .put("shape", shape.map_or(Value::Null, text))
        .put("rho", d.get("rho").cloned().unwrap_or(Value::Null))
        .put("blank_rho", opt(blank_rho))
        .put("class", text(class))
        .put("reasons", Value::Arr(why.into_iter().map(text).collect()))
        .done();
    (row, class)
}

/// `compare/1` of `inp` under `o`; the rows keep the order of the table, so the bytes are stable.
pub fn compare(inp: &CmpIn, o: &CmpOpts) -> R<Value> {
    check(inp)?;
    let p = plan_of(inp, o)?;
    let (diff, pc, origin) = reference(inp, o, &p)?;
    let cols: Vec<usize> = p
        .blank
        .iter()
        .copied()
        .filter(|&i| p.cols[i].time.is_some())
        .collect();
    let times = cols
        .iter()
        .map(|&i| p.cols[i].time.unwrap_or(0.0))
        .collect();
    let bc = BlankCourse { cols, times };
    let mut counts = [0i64; 5];
    let mut rows = Vec::with_capacity(inp.matrix.len());
    for (f, d) in diff
        .get("rows")
        .map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        let (row, class) = feature_row(inp, &p, &bc, d, f, pc);
        counts[CLASSES.iter().position(|c| *c == class).unwrap_or(3)] += 1;
        rows.push(row);
    }
    let (roles_v, params, choices) = header(inp, o, &p, pc, &origin);
    let cnt = CLASSES
        .iter()
        .zip(counts)
        .fold(Obj::new(), |o, (c, k)| o.put(c, int(k)))
        .done();
    Ok(Obj::new()
        .put("schema", text("compare/1"))
        .put("table", table_of(inp))
        .put("roles", roles_v)
        .put("params", params)
        .put("choices", Value::Arr(choices))
        .put("counts", cnt)
        .put("rows", Value::Arr(rows))
        .done())
}
