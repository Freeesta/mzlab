//! `compare/1`: every feature of a time-course table against its reference (t0) and its blank, to flag candidate transformation products. It wraps
//! `statref::reference_difference` (reference level, maximum, shape over time, pseudocount) and adds the blank's own ratio and time course. The class
//! (`candidate`, `blank_artefact`, ...) only sorts and filters: every row is written, nothing is removed, the user decides.

use crate::jsonb::{int, num, opt, text, Obj};
use crate::json::Value;
use crate::statistics::{bad, user, R};
use crate::statref::{reference_difference, spearman, RefIn, RefOpts, FLOOR};

/// The rules in the order they are tried; the first that matches decides (written to `choices` and to the schema).
pub const RULES: &str = "1 no_signal: max_treated <= pseudocount; 2 blank_artefact: ratio_blank < factor (blank_level) or blank_rho >= rho_min with the blank maximum above the pseudocount (blank_rises); 3 candidate: absent_in_reference or ratio_ref >= factor (above_reference); 4 decreasing: ratio_ref <= 1/factor (below_reference) or shape falls (falls); 5 unchanged";

pub const CLASSES: [&str; 5] = ["candidate", "blank_artefact", "decreasing", "unchanged", "no_signal"];

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

struct Col {
    time: Option<f64>,
    origin: &'static str,
    role: Option<String>,
}

struct Plan {
    cols: Vec<Col>,
    reference: Vec<usize>,
    blank: Vec<usize>,
    treated: Vec<usize>,
    quantity: String,
    factor: f64,
    floor: f64,
    rho_min: f64,
    choices: Vec<Value>,
}

fn choice(key: &str, value: Value, reason: &str) -> Value {
    Obj::new().put("key", text(key)).put("value", value).put("reason", text(reason)).done()
}

/// `*` and `?` over characters, case-insensitive.
fn glob(pat: &[char], s: &[char]) -> bool {
    let (mut p, mut i, mut star, mut mark) = (0, 0, None, 0);
    while i < s.len() {
        if p < pat.len() && (pat[p] == '?' || pat[p] == s[i]) {
            p += 1;
            i += 1;
        } else if p < pat.len() && pat[p] == '*' {
            star = Some(p);
            mark = i;
            p += 1;
        } else if let Some(sp) = star {
            p = sp + 1;
            mark += 1;
            i = mark;
        } else {
            return false;
        }
    }
    pat[p..].iter().all(|&c| c == '*')
}

fn lower(s: &str) -> Vec<char> {
    s.trim().to_lowercase().chars().collect()
}

/// The columns a selector picks: a comma list of globs over the names, `role:NAME` (`reference` is the role `t0`) or `time:MINUTES`; the last two need the samples document.
fn select(sel: &str, names: &[String], cols: &[Col], has_meta: bool) -> R<Vec<usize>> {
    let mut hit = vec![false; names.len()];
    for tok in sel.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        let low = tok.to_lowercase();
        if let Some(role) = low.strip_prefix("role:") {
            if !has_meta {
                return user("err.compare.samples", "role: and time: selectors need --samples");
            }
            let role = if role == "reference" { "t0" } else { role };
            (0..names.len()).filter(|&i| cols[i].role.as_deref() == Some(role)).for_each(|i| hit[i] = true);
        } else if let Some(t) = low.strip_prefix("time:") {
            if !has_meta {
                return user("err.compare.samples", "role: and time: selectors need --samples");
            }
            let Ok(t) = t.trim().parse::<f64>() else { return user("err.compare.param", &format!("time: needs a number of minutes, got '{tok}'")) };
            (0..names.len()).filter(|&i| cols[i].time == Some(t)).for_each(|i| hit[i] = true);
        } else {
            let pat = lower(tok);
            (0..names.len()).filter(|&i| glob(&pat, &lower(&names[i]))).for_each(|i| hit[i] = true);
        }
    }
    let found: Vec<usize> = (0..names.len()).filter(|&i| hit[i]).collect();
    if found.is_empty() {
        let list = names.iter().take(40).cloned().collect::<Vec<_>>().join(", ");
        let more = if names.len() > 40 { format!(", ... ({} columns)", names.len()) } else { String::new() };
        return user("err.compare.selector", &format!("no column matches '{sel}'; the columns are: {list}{more}"));
    }
    Ok(found)
}

/// The time of every column: from the samples document, else parsed from the names by the one name parser of `sample table` (`samples::role_and_minutes`: no series, no replicate logic).
fn columns(inp: &CmpIn) -> Vec<Col> {
    match inp.meta {
        Some(meta) => meta
            .iter()
            .map(|m| match m {
                Some(c) => Col { time: c.time, origin: if c.time.is_some() { "samples" } else { "none" }, role: c.role.clone() },
                None => Col { time: None, origin: "none", role: None },
            })
            .collect(),
        None => {
            (0..inp.names.len())
                .map(|i| {
                    let time = crate::samples::role_and_minutes(&inp.names[i]).1;
                    Col { time, origin: if time.is_some() { "name" } else { "none" }, role: None }
                })
                .collect()
        }
    }
}

fn plan_of(inp: &CmpIn, o: &CmpOpts) -> R<Plan> {
    let mut choices = Vec::new();
    let quantity = o.quantity.clone().unwrap_or_else(|| "area".into());
    if quantity != "area" && quantity != "height" {
        return user("err.compare.param", "quantity must be 'area' or 'height'");
    }
    let (factor, floor, rho_min_v) = (o.factor.unwrap_or(3.0), o.floor.unwrap_or(FLOOR), o.rho_min.unwrap_or(0.5));
    if factor.is_nan() || factor <= 0.0 || floor.is_nan() || floor < 0.0 || rho_min_v.is_nan() || !(0.0..=1.0).contains(&rho_min_v) || o.pseudo.is_some_and(|p| p.is_nan() || p < 0.0) {
        return user("err.compare.param", "factor must be > 0, the floor and the pseudocount >= 0 and rho_min between 0 and 1");
    }
    for (key, given, v, why) in [
        ("quantity", o.quantity.is_some(), text("area"), "area unless --quantity says height"),
        ("factor", o.factor.is_some(), num(3.0), "the factor of the reference difference"),
        ("floor", o.floor.is_some(), num(FLOOR), "smallest estimated pseudocount, in the unit of the quantity"),
        ("rho_min", o.rho_min.is_some(), num(0.5), "smallest Spearman rho of a rise or a fall"),
    ] {
        if !given {
            choices.push(choice(key, v, &format!("default; {why}")));
        }
    }
    choices.push(choice("reference_statistic", text("max"), "default; the reference level is the maximum of the reference columns"));
    let cols = columns(inp);
    let has_meta = inp.meta.is_some();
    let reference = select(&o.ref_sel, inp.names, &cols, has_meta)?;
    let blank = select(&o.blank_sel, inp.names, &cols, has_meta)?;
    if let Some(&i) = reference.iter().find(|i| blank.contains(i)) {
        return user("err.compare.overlap", &format!("the column '{}' is in both --ref and --blank", inp.names[i]));
    }
    let treated: Vec<usize> = (0..inp.names.len()).filter(|i| !reference.contains(i) && !blank.contains(i)).collect();
    if treated.is_empty() {
        return user("err.stat.ref.notreated", "no treated column besides the reference and the blank");
    }
    for (i, c) in cols.iter().enumerate() {
        choices.push(choice(&format!("time:{}", inp.names[i]), opt(c.time), c.origin));
    }
    choices.push(choice("class_rules", text(RULES), "fixed rules; the first that matches decides and nothing is removed"));
    Ok(Plan { cols, reference, blank, treated, quantity, factor, floor, rho_min: rho_min_v, choices })
}

fn names_of(inp: &CmpIn, v: &[usize]) -> Value {
    Value::Arr(v.iter().map(|&i| text(&inp.names[i])).collect())
}

fn table_of(inp: &CmpIn) -> Value {
    Obj::new().put("path", text(inp.path)).put("sha256", text(inp.sha256)).i("rows", inp.matrix.len() as i64).i("columns", inp.names.len() as i64).done()
}

/// `compare-plan/1`: the columns of every role with their times, the parameters and the choices; nothing is computed.
pub fn plan(inp: &CmpIn, o: &CmpOpts) -> R<Value> {
    check(inp)?;
    let p = plan_of(inp, o)?;
    let col = |v: &[usize]| {
        Value::Arr(v.iter().map(|&i| Obj::new().put("name", text(&inp.names[i])).put("time_min", opt(p.cols[i].time)).put("time_origin", text(p.cols[i].origin)).done()).collect())
    };
    let roles = Obj::new()
        .put("reference", Obj::new().put("selector", text(&o.ref_sel)).put("columns", col(&p.reference)).done())
        .put("blank", Obj::new().put("selector", text(&o.blank_sel)).put("columns", col(&p.blank)).done())
        .put("treated", Obj::new().put("columns", col(&p.treated)).done())
        .done();
    let pseudo = o.pseudo.map_or(text("auto"), num);
    let params = Obj::new().put("quantity", text(&p.quantity)).n("factor", p.factor).put("pseudocount", pseudo).n("floor", p.floor).n("rho_min", p.rho_min).done();
    Ok(Obj::new().put("schema", text("compare-plan/1")).put("table", table_of(inp)).put("roles", roles).put("params", params).put("choices", Value::Arr(p.choices)).done())
}

fn check(inp: &CmpIn) -> R<()> {
    let n = inp.matrix.len();
    let wide = inp.matrix.iter().any(|r| r.len() != inp.names.len());
    if inp.ids.len() != n || inp.mz.len() != n || inp.rt.len() != n || wide || inp.meta.is_some_and(|m| m.len() != inp.names.len()) {
        return bad("the table, the ids and the column names must agree");
    }
    Ok(())
}

fn ratio(x: f64, y: f64, pc: f64) -> f64 {
    (x + pc) / (y + pc)
}

/// The class of one feature and why (rules in `RULES`).
#[allow(clippy::too_many_arguments)]
fn classify(max: f64, pc: f64, r_ref: f64, r_blank: f64, absent: bool, shape: Option<&str>, blank_max: f64, blank_rho: Option<f64>, factor: f64, rho_min: f64) -> (&'static str, Vec<&'static str>) {
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
    let sel = |s: &str, v: &[usize]| Obj::new().put("selector", text(s)).put("columns", names_of(inp, v)).done();
    let roles_v = Obj::new()
        .put("reference", sel(&o.ref_sel, &p.reference))
        .put("blank", sel(&o.blank_sel, &p.blank))
        .put("treated", Obj::new().put("columns", names_of(inp, &p.treated)).done())
        .done();
    let params = Obj::new().put("quantity", text(&p.quantity)).n("factor", p.factor).n("pseudocount", pc).put("pseudocount_origin", text(origin)).n("floor", p.floor).n("rho_min", p.rho_min).done();
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

/// `compare/1` of `inp` under `o`; the rows keep the order of the table, so the bytes are stable.
pub fn compare(inp: &CmpIn, o: &CmpOpts) -> R<Value> {
    check(inp)?;
    let p = plan_of(inp, o)?;
    let n = inp.names.len();
    let mut roles = vec![String::new(); n];
    // the reference columns are "dark" (never a blank and never treated); the blank columns are "blank"; the pseudocount reads the blanks
    p.reference.iter().for_each(|&i| roles[i] = "dark".into());
    p.blank.iter().for_each(|&i| roles[i] = "blank".into());
    p.treated.iter().for_each(|&i| roles[i] = "sample".into());
    let times: Vec<Option<f64>> = p.cols.iter().map(|c| c.time).collect();
    let refcols: Vec<i64> = p.reference.iter().map(|&i| i as i64).collect();
    let ropts = RefOpts { factor: p.factor, pseudo: o.pseudo, floor: p.floor, rho_min: p.rho_min, ..RefOpts::default() };
    let rin = RefIn { matrix: inp.matrix, roles: &roles, times: &times, groups: None, reference: Some(&refcols) };
    let diff = reference_difference(&rin, &ropts)?;
    let pc = diff.get("params").and_then(|v| v.get("pseudo")).and_then(Value::f64).unwrap_or(p.floor);
    let origin = diff.get("params").and_then(|v| v.get("pseudo_source")).and_then(Value::str).unwrap_or("floor").to_string();
    let timed: Vec<usize> = p.blank.iter().copied().filter(|&i| times[i].is_some()).collect();
    let btimes: Vec<f64> = timed.iter().map(|&i| times[i].unwrap_or(0.0)).collect();
    let mut counts = [0i64; 5];
    let mut rows = Vec::with_capacity(inp.matrix.len());
    for (f, d) in diff.get("rows").map(Value::arr).unwrap_or(&[]).iter().enumerate() {
        let get = |k: &str| d.get(k).and_then(Value::f64).unwrap_or(0.0);
        let (ref_level, max_treated) = (get("ref_level"), get("treated_max"));
        let a = &inp.matrix[f];
        let blank_max = p.blank.iter().fold(0.0f64, |m, &i| m.max(a[i]));
        let by: Vec<f64> = timed.iter().map(|&i| a[i]).collect();
        let blank_rho = if btimes.len() >= 3 { spearman(&btimes, &by) } else { None };
        let (r_ref, r_blank) = (ratio(max_treated, ref_level, pc), ratio(max_treated, blank_max, pc));
        let absent = d.get("absent_in_reference").and_then(Value::bool).unwrap_or(false);
        let shape = d.get("shape").and_then(Value::str);
        let (class, why) = classify(max_treated, pc, r_ref, r_blank, absent, shape, blank_max, blank_rho, p.factor, p.rho_min);
        counts[CLASSES.iter().position(|c| *c == class).unwrap_or(3)] += 1;
        let file = d.get("max_file").and_then(Value::f64).map_or(Value::Null, |i| text(&inp.names[i as usize]));
        rows.push(
            Obj::new()
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
                .done(),
        );
    }
    let (roles_v, params, choices) = header(inp, o, &p, pc, &origin);
    let cnt = CLASSES.iter().zip(counts).fold(Obj::new(), |o, (c, k)| o.put(c, int(k))).done();
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
