//! `gruppi/1` as text (keys in English, fixed order) and the input of the module as a JSON document:
//! `{"table": {n_rows, n_files, rows, area, height, state, apex_rt}, "campioni": campioni/2, "opts": {...}}` (the matrices as lists of rows).

use super::{build, CondOut, GroupOpts, GroupTable, GroupsOut, RowMeta, RowOut};
use mzlab_core::campioni::{self, Campione};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};
use std::fmt::Write;

/// `Out::num`, `int` and `opt` without a temporary string per number (the table has millions of them).
trait Fast {
    fn fnum(&mut self, v: f64) -> &mut Self;
    fn fint(&mut self, v: i64) -> &mut Self;
    fn fopt(&mut self, v: Option<f64>) -> &mut Self;
    fn ffloats(&mut self, a: &[f64]) -> &mut Self;
}

impl Fast for Out {
    fn fnum(&mut self, v: f64) -> &mut Self {
        if v.is_finite() {
            let _ = write!(self.0, "{v}");
        } else {
            self.0.push_str("null");
        }
        self
    }
    fn fint(&mut self, v: i64) -> &mut Self {
        let _ = write!(self.0, "{v}");
        self
    }
    fn fopt(&mut self, v: Option<f64>) -> &mut Self {
        match v {
            Some(x) => self.fnum(x),
            None => self.raw("null"),
        }
    }
    fn ffloats(&mut self, a: &[f64]) -> &mut Self {
        self.0.push('[');
        for (i, &v) in a.iter().enumerate() {
            if i > 0 {
                self.0.push(',');
            }
            self.fnum(v);
        }
        self.0.push(']');
        self
    }
}

/// The text of `gruppi/1` for a table, the files and the options.
pub fn gruppi_json(tb: &GroupTable, cs: &[Campione], opts: &GroupOpts) -> Result<String> {
    Ok(to_json(&build(tb, cs, opts)?))
}

/// Builds the groups from the JSON input described in the module header and returns `gruppi/1`.
pub fn run_json(input: &[u8]) -> Result<String> {
    let (tb, cs, opts) = parse_input(input)?;
    gruppi_json(&tb, &cs, &opts)
}

/// Six decimals, cheaply (the ratios are for people; `Out::rounded` formats and parses).
fn r6(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

fn strs(o: &mut Out, v: &[String]) {
    o.raw("[");
    for (i, s) in v.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(s);
    }
    o.raw("]");
}

fn ints(o: &mut Out, v: &[usize]) {
    o.raw("[");
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.fint(*x as i64);
    }
    o.raw("]");
}

fn opt_str(o: &mut Out, v: Option<&str>) {
    match v {
        Some(s) => o.str(s),
        None => o.raw("null"),
    };
}

fn cond_json(o: &mut Out, c: &CondOut) {
    o.raw("{\"id\":").str(&c.id).raw(",\"factors\":{");
    for (i, (k, v)) in c.factors.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(k).raw(":").str(v);
    }
    o.raw("},\"files\":");
    ints(o, &c.files);
    o.raw(",\"times\":").ffloats(&c.times);
    o.raw(",\"replicates\":").fint(c.replicates as i64);
    o.raw(",\"controls\":{\"t0\":");
    ints(o, &c.controls.t0);
    o.raw(",\"blank\":");
    ints(o, &c.controls.blank);
    o.raw(",\"std\":");
    ints(o, &c.controls.std);
    o.raw(",\"dark\":");
    ints(o, &c.controls.dark);
    o.raw("},\"parent\":{\"k\":").fopt(c.parent.k);
    o.raw(",\"half_life\":").fopt(c.parent.half_life);
    o.raw(",\"remaining\":").fopt(c.parent.remaining);
    o.raw("}}");
}

fn row_json(o: &mut Out, r: &RowOut) {
    let m = &r.meta;
    o.raw("{\"row\":").fint(m.id as i64);
    o.raw(",\"tp_name\":").str(&m.tp_name);
    o.raw(",\"mz\":").fnum(m.mz);
    o.raw(",\"polarity\":").str(&m.polarity);
    o.raw(",\"rt\":").fnum(m.rt);
    o.raw(",\"neutral_formula\":");
    opt_str(o, m.neutral_formula.as_deref());
    o.raw(",\"ppm\":").fopt(m.ppm);
    o.raw(",\"ion_formula\":");
    opt_str(o, m.ion_formula.as_deref());
    o.raw(",\"other_polarity\":")
        .fopt(m.other_polarity.map(f64::from));
    o.raw(",\"present_in\":");
    strs(o, &r.present_in);
    o.raw(",\"only_in\":");
    opt_str(o, r.only_in.as_deref());
    o.raw(",\"in_all\":")
        .raw(if r.in_all { "true" } else { "false" });
    o.raw(",\"cond\":[");
    for (i, c) in r.cond.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        cell_json(o, c);
    }
    o.raw("]}");
}

fn cell_json(o: &mut Out, c: &super::CondRow) {
    o.raw("{\"id\":").str(&c.id);
    o.raw(",\"present\":")
        .raw(if c.present { "true" } else { "false" });
    o.raw(",\"n_det\":").fint(c.n_det as i64);
    o.raw(",\"n_rep\":").fint(c.n_rep as i64);
    o.raw(",\"area\":").fnum(c.area);
    o.raw(",\"area_sd\":").fopt(c.area_sd);
    o.raw(",\"cv\":");
    o.fopt(c.cv.map(r6));
    o.raw(",\"ratio_ref\":");
    o.fopt(c.ratio_ref.map(r6));
    o.raw(",\"shape\":");
    opt_str(o, c.shape);
    o.raw(",\"t_max\":").fopt(c.t_max);
    o.raw(",\"t_half_rise\":").fopt(c.t_half_rise);
    o.raw(",\"rate_rel\":");
    o.fopt(c.rate_rel.map(r6));
    o.raw(",\"kin\":[");
    for (i, p) in c.kin.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.raw("{\"t\":").fnum(p.t).raw(",\"area\":").fnum(p.area);
        o.raw(",\"sd\":")
            .fopt(p.sd)
            .raw(",\"n\":")
            .fint(p.n as i64)
            .raw("}");
    }
    o.raw("],\"prev\":");
    match &c.prev {
        None => {
            o.raw("null");
        }
        Some(p) => {
            o.raw("{\"absent_t0\":").str(p.absent_t0);
            o.raw(",\"absent_blank\":").str(p.absent_blank);
            o.raw(",\"in_parent_std\":").str(p.in_parent_std);
            o.raw(",\"above_min\":").str(p.above_min);
            o.raw(",\"ms2\":").str(p.ms2);
            o.raw(",\"verdict\":").str(p.verdict).raw("}");
        }
    }
    o.raw("}");
}

/// Writes the groups in the fixed order of the schema.
pub fn to_json(g: &GroupsOut) -> String {
    let mut o = Out::default();
    o.raw("{\"format\":\"gruppi\",\"version\":1,\"reference\":");
    opt_str(&mut o, g.reference.as_deref());
    o.raw(",\"norm\":\"raw\",\"conditions\":[");
    for (i, c) in g.conditions.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        cond_json(&mut o, c);
    }
    o.raw("],\"rows\":[");
    for (i, r) in g.rows.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        row_json(&mut o, r);
    }
    o.raw("],\"sets\":[");
    for (i, s) in g.sets.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.raw("{\"conditions\":");
        strs(&mut o, &s.conditions);
        o.raw(",\"rows\":[");
        for (j, r) in s.rows.iter().enumerate() {
            if j > 0 {
                o.raw(",");
            }
            o.fint(*r as i64);
        }
        o.raw("]}");
    }
    o.raw("],\"warnings\":[");
    for (i, w) in g.warnings.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.raw("{\"code\":").str(w.code).raw(",\"condition\":");
        o.str(&w.condition).raw("}");
    }
    o.raw("]}");
    o.0
}

// ---- input ----

fn bad(name: &'static str, v: impl ToString) -> Error {
    Error::new("error.gruppi.badOpts").with(name, v)
}

fn size(what: &str) -> Error {
    Error::new("error.gruppi.sizeMismatch").with("what", what)
}

/// Options from a JSON object; missing keys keep their default, wrong types are `error.gruppi.badOpts`.
fn parse_opts(v: &Value) -> Result<GroupOpts> {
    let mut o = GroupOpts::default();
    let num = |k: &'static str, into: &mut f64| -> Result<()> {
        match v.get(k) {
            None => Ok(()),
            Some(x) => x.f64().map(|n| *into = n).ok_or_else(|| bad(k, "type")),
        }
    };
    num("min_rep_frac", &mut o.min_rep_frac)?;
    num("min_area", &mut o.min_area)?;
    num("ctrl_ratio", &mut o.ctrl_ratio)?;
    num("shape_drop", &mut o.shape_drop)?;
    match v.get("reference") {
        None | Some(Value::Null) => {}
        Some(x) => o.reference = Some(x.str().ok_or_else(|| bad("reference", "type"))?.to_string()),
    }
    if let Some(x) = v.get("norm") {
        o.norm = x.str().ok_or_else(|| bad("norm", "type"))?.to_string();
    }
    if let Some(Value::Obj(fs)) = v.get("factors") {
        for (cond, kv) in fs {
            o.factors.push((cond.clone(), strings(cond, kv)?));
        }
    } else if v.get("factors").is_some_and(|x| !x.is_null()) {
        return Err(bad("factors", "type"));
    }
    Ok(o)
}

/// An object of strings: anything else is a bad option.
fn strings(cond: &str, kv: &Value) -> Result<Vec<(String, String)>> {
    let Value::Obj(items) = kv else {
        return Err(bad("factors", cond));
    };
    items
        .iter()
        .map(|(k, x)| match x.str() {
            Some(s) => Ok((k.clone(), s.to_string())),
            None => Err(bad("factors", cond)),
        })
        .collect()
}

fn meta(v: &Value) -> Result<RowMeta> {
    let id = v
        .get("row")
        .or_else(|| v.get("id"))
        .and_then(Value::f64)
        .ok_or_else(|| size("rows"))?;
    let num = |k: &str| v.get(k).and_then(Value::f64);
    let text = |k: &str| v.get(k).and_then(Value::str).map(str::to_string);
    Ok(RowMeta {
        id: id as u32,
        tp_name: text("tp_name").unwrap_or_default(),
        mz: num("mz").unwrap_or(0.0),
        polarity: text("polarity").unwrap_or_default(),
        rt: num("rt").unwrap_or(0.0),
        neutral_formula: text("neutral_formula"),
        ppm: num("ppm"),
        ion_formula: text("ion_formula"),
        is_parent: v.get("is_parent").and_then(Value::bool).unwrap_or(false),
        ms2_files: v
            .get("ms2_files")
            .map(|a| {
                a.arr()
                    .iter()
                    .filter_map(Value::f64)
                    .map(|x| x as usize)
                    .collect()
            })
            .unwrap_or_default(),
        other_polarity: num("other_polarity").map(|x| x as u32),
    })
}

/// A matrix written as a list of rows, flattened row by row.
fn matrix(v: &Value, key: &str) -> Result<Vec<f64>> {
    let rows = v.get(key).ok_or_else(|| size(key))?.arr();
    let mut out = Vec::new();
    for r in rows {
        for x in r.arr() {
            out.push(x.f64().ok_or_else(|| size(key))?);
        }
    }
    Ok(out)
}

fn parse_table(t: &Value) -> Result<GroupTable> {
    let dim = |k: &str| {
        t.get(k)
            .and_then(Value::f64)
            .map(|x| x as usize)
            .ok_or_else(|| size(k))
    };
    let state: Vec<u8> = matrix(t, "state")?.into_iter().map(|x| x as u8).collect();
    let apex_rt: Vec<f32> = matrix(t, "apex_rt")?
        .into_iter()
        .map(|x| x as f32)
        .collect();
    Ok(GroupTable {
        n_rows: dim("n_rows")?,
        n_files: dim("n_files")?,
        area: matrix(t, "area")?,
        height: matrix(t, "height")?,
        state,
        apex_rt,
        rows: t
            .get("rows")
            .map(Value::arr)
            .unwrap_or(&[])
            .iter()
            .map(meta)
            .collect::<Result<_>>()?,
    })
}

/// Reads the JSON input of `run_json`.
pub fn parse_input(input: &[u8]) -> Result<(GroupTable, Vec<Campione>, GroupOpts)> {
    let doc = parse(input)?;
    let tb = parse_table(doc.get("table").ok_or_else(|| size("table"))?)?;
    let mut cs_text = Out::default();
    dump(
        &mut cs_text,
        doc.get("campioni").ok_or_else(|| size("campioni"))?,
    );
    let cs = campioni::from_json(cs_text.0.as_bytes())?;
    let opts = match doc.get("opts") {
        Some(v) => parse_opts(v)?,
        None => GroupOpts::default(),
    };
    Ok((tb, cs, opts))
}

/// A parsed value back to compact text (to hand `campioni/2` to its own reader).
fn dump(o: &mut Out, v: &Value) {
    match v {
        Value::Null => {
            o.raw("null");
        }
        Value::Bool(b) => {
            o.raw(if *b { "true" } else { "false" });
        }
        Value::Num(n) => {
            o.fnum(*n);
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
                dump(o, x);
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
                dump(o, x);
            }
            o.raw("}");
        }
    }
}
