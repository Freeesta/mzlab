//! The plan of `table compare`: which columns are the reference, the blank and the treated, the time of each column and the parameters taken. Used by
//! `compare/1` (`statcmp::compare`) and by `compare-plan/1` (`plan`, which computes nothing).

use crate::glob::glob;
use crate::json::Value;
use crate::jsonb::{num, opt, text, Obj};
use crate::statcmp::{CmpIn, CmpOpts, RULES};
use crate::statistics::{bad, user, R};
use crate::statref::FLOOR;

pub(crate) struct Col {
    pub(crate) time: Option<f64>,
    pub(crate) origin: &'static str,
    pub(crate) role: Option<String>,
}

pub(crate) struct Plan {
    pub(crate) cols: Vec<Col>,
    pub(crate) reference: Vec<usize>,
    pub(crate) blank: Vec<usize>,
    pub(crate) treated: Vec<usize>,
    pub(crate) quantity: String,
    pub(crate) factor: f64,
    pub(crate) floor: f64,
    pub(crate) rho_min: f64,
    pub(crate) choices: Vec<Value>,
}

pub(crate) fn choice(key: &str, value: Value, reason: &str) -> Value {
    Obj::new()
        .put("key", text(key))
        .put("value", value)
        .put("reason", text(reason))
        .done()
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
                return user(
                    "err.compare.samples",
                    "role: and time: selectors need --samples",
                );
            }
            let role = if role == "reference" { "t0" } else { role };
            (0..names.len())
                .filter(|&i| cols[i].role.as_deref() == Some(role))
                .for_each(|i| hit[i] = true);
        } else if let Some(t) = low.strip_prefix("time:") {
            if !has_meta {
                return user(
                    "err.compare.samples",
                    "role: and time: selectors need --samples",
                );
            }
            let Ok(t) = t.trim().parse::<f64>() else {
                return user(
                    "err.compare.param",
                    &format!("time: needs a number of minutes, got '{tok}'"),
                );
            };
            (0..names.len())
                .filter(|&i| cols[i].time == Some(t))
                .for_each(|i| hit[i] = true);
        } else {
            let pat = lower(tok);
            (0..names.len())
                .filter(|&i| glob(&pat, &lower(&names[i])))
                .for_each(|i| hit[i] = true);
        }
    }
    let found: Vec<usize> = (0..names.len()).filter(|&i| hit[i]).collect();
    if found.is_empty() {
        let list = names
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let more = if names.len() > 40 {
            format!(", ... ({} columns)", names.len())
        } else {
            String::new()
        };
        return user(
            "err.compare.selector",
            &format!("no column matches '{sel}'; the columns are: {list}{more}"),
        );
    }
    Ok(found)
}

/// The time of every column: from the samples document, else parsed from the names by the one name parser of `sample table` (`samples::role_and_minutes`: no series, no replicate logic).
fn columns(inp: &CmpIn) -> Vec<Col> {
    match inp.meta {
        Some(meta) => meta
            .iter()
            .map(|m| match m {
                Some(c) => Col {
                    time: c.time,
                    origin: if c.time.is_some() { "samples" } else { "none" },
                    role: c.role.clone(),
                },
                None => Col {
                    time: None,
                    origin: "none",
                    role: None,
                },
            })
            .collect(),
        None => (0..inp.names.len())
            .map(|i| {
                let time = crate::samples::role_and_minutes(&inp.names[i]).1;
                Col {
                    time,
                    origin: if time.is_some() { "name" } else { "none" },
                    role: None,
                }
            })
            .collect(),
    }
}

/// The numbers of the run: the ones given and the defaults taken (each default is a `choices` entry).
struct Params {
    quantity: String,
    factor: f64,
    floor: f64,
    rho_min: f64,
}

fn params_of(o: &CmpOpts) -> R<(Params, Vec<Value>)> {
    let quantity = o.quantity.clone().unwrap_or_else(|| "area".into());
    if quantity != "area" && quantity != "height" {
        return user("err.compare.param", "quantity must be 'area' or 'height'");
    }
    let (factor, floor, rho_min) = (
        o.factor.unwrap_or(3.0),
        o.floor.unwrap_or(FLOOR),
        o.rho_min.unwrap_or(0.5),
    );
    if factor.is_nan()
        || factor <= 0.0
        || floor.is_nan()
        || floor < 0.0
        || rho_min.is_nan()
        || !(0.0..=1.0).contains(&rho_min)
        || o.pseudo.is_some_and(|p| p.is_nan() || p < 0.0)
    {
        return user(
            "err.compare.param",
            "factor must be > 0, the floor and the pseudocount >= 0 and rho_min between 0 and 1",
        );
    }
    let mut choices = Vec::new();
    for (key, given, v, why) in [
        (
            "quantity",
            o.quantity.is_some(),
            text("area"),
            "area unless --quantity says height",
        ),
        (
            "factor",
            o.factor.is_some(),
            num(3.0),
            "the factor of the reference difference",
        ),
        (
            "floor",
            o.floor.is_some(),
            num(FLOOR),
            "smallest estimated pseudocount, in the unit of the quantity",
        ),
        (
            "rho_min",
            o.rho_min.is_some(),
            num(0.5),
            "smallest Spearman rho of a rise or a fall",
        ),
    ] {
        if !given {
            choices.push(choice(key, v, &format!("default; {why}")));
        }
    }
    let why = "default; the reference level is the maximum of the reference columns";
    choices.push(choice("reference_statistic", text("max"), why));
    Ok((
        Params {
            quantity,
            factor,
            floor,
            rho_min,
        },
        choices,
    ))
}

pub(crate) fn plan_of(inp: &CmpIn, o: &CmpOpts) -> R<Plan> {
    let (par, mut choices) = params_of(o)?;
    // two columns with one name (selectors ignore the case, so so does this) cannot be told apart by any selector: the table was made from several
    // samples of one file under the file name
    let low: Vec<String> = inp.names.iter().map(|n| n.to_lowercase()).collect();
    if let Some((i, n)) = (0..low.len()).find_map(|i| {
        let n = low[i].as_str();
        (low[..i].iter().any(|o| o == n)).then_some((i, inp.names[i].as_str()))
    }) {
        let all = low.iter().filter(|o| **o == low[i]).count();
        let text = format!(
            "the name '{n}' is on {all} columns (the second is column {})",
            i + 1
        );
        return user("err.compare.duplicate", &text);
    }
    let cols = columns(inp);
    let has_meta = inp.meta.is_some();
    let reference = select(&o.ref_sel, inp.names, &cols, has_meta)?;
    let blank = select(&o.blank_sel, inp.names, &cols, has_meta)?;
    if let Some(&i) = reference.iter().find(|i| blank.contains(i)) {
        let text = format!("the column '{}' is in both --ref and --blank", inp.names[i]);
        return user("err.compare.overlap", &text);
    }
    let treated: Vec<usize> = (0..inp.names.len())
        .filter(|i| !reference.contains(i) && !blank.contains(i))
        .collect();
    if treated.is_empty() {
        return user(
            "err.stat.ref.notreated",
            "no treated column besides the reference and the blank",
        );
    }
    for (i, c) in cols.iter().enumerate() {
        choices.push(choice(
            &format!("time:{}", inp.names[i]),
            opt(c.time),
            c.origin,
        ));
    }
    let why = "fixed rules; the first that matches decides and nothing is removed";
    choices.push(choice("class_rules", text(RULES), why));
    Ok(Plan {
        cols,
        reference,
        blank,
        treated,
        quantity: par.quantity,
        factor: par.factor,
        floor: par.floor,
        rho_min: par.rho_min,
        choices,
    })
}

pub(crate) fn names_of(inp: &CmpIn, v: &[usize]) -> Value {
    Value::Arr(v.iter().map(|&i| text(&inp.names[i])).collect())
}

pub(crate) fn table_of(inp: &CmpIn) -> Value {
    Obj::new()
        .put("path", text(inp.path))
        .put("sha256", text(inp.sha256))
        .i("rows", inp.matrix.len() as i64)
        .i("columns", inp.names.len() as i64)
        .done()
}

/// `compare-plan/1`: the columns of every role with their times, the parameters and the choices; nothing is computed.
pub fn plan(inp: &CmpIn, o: &CmpOpts) -> R<Value> {
    check(inp)?;
    let p = plan_of(inp, o)?;
    let col = |v: &[usize]| {
        Value::Arr(
            v.iter()
                .map(|&i| {
                    Obj::new()
                        .put("name", text(&inp.names[i]))
                        .put("time_min", opt(p.cols[i].time))
                        .put("time_origin", text(p.cols[i].origin))
                        .done()
                })
                .collect(),
        )
    };
    let roles = Obj::new()
        .put(
            "reference",
            Obj::new()
                .put("selector", text(&o.ref_sel))
                .put("columns", col(&p.reference))
                .done(),
        )
        .put(
            "blank",
            Obj::new()
                .put("selector", text(&o.blank_sel))
                .put("columns", col(&p.blank))
                .done(),
        )
        .put("treated", Obj::new().put("columns", col(&p.treated)).done())
        .done();
    let pseudo = o.pseudo.map_or(text("auto"), num);
    let params = Obj::new()
        .put("quantity", text(&p.quantity))
        .n("factor", p.factor)
        .put("pseudocount", pseudo)
        .n("floor", p.floor)
        .n("rho_min", p.rho_min)
        .done();
    Ok(Obj::new()
        .put("schema", text("compare-plan/1"))
        .put("table", table_of(inp))
        .put("roles", roles)
        .put("params", params)
        .put("choices", Value::Arr(p.choices))
        .done())
}

pub(crate) fn check(inp: &CmpIn) -> R<()> {
    let n = inp.matrix.len();
    let wide = inp.matrix.iter().any(|r| r.len() != inp.names.len());
    if inp.ids.len() != n
        || inp.mz.len() != n
        || inp.rt.len() != n
        || wide
        || inp.meta.is_some_and(|m| m.len() != inp.names.len())
    {
        return bad("the table, the ids and the column names must agree");
    }
    Ok(())
}
