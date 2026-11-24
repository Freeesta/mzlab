//! The `union/1` document: the target list (CSV for `quant.targets` and objects with the rows each came from), the series with what the RT map did, the
//! conflicts, the targets dropped by `min_series` and every automatic choice.

use super::merge::{Merged, Target};
use super::params::Params;
use super::read::Series;
use crate::json::Value;
use crate::jsonb::{int, num, text, write, Obj};

fn round(v: f64, d: i32) -> f64 {
    let f = 10f64.powi(d);
    (v * f).round() / f
}

fn pol_rank(p: Option<i8>) -> u8 {
    match p {
        None => 0,
        Some(-1) => 1,
        Some(_) => 2,
    }
}

fn pol_text(p: Option<i8>) -> &'static str {
    match p {
        None => "",
        Some(-1) => "-",
        Some(_) => "+",
    }
}

fn pol_value(p: Option<i8>) -> Value {
    if p.is_none() {
        Value::Null
    } else {
        text(pol_text(p))
    }
}

/// A target ready to be written.
struct Done<'a> {
    t: &'a Target,
    mz: f64,
    rt: f64,
    lo: f64,
    hi: f64,
    clamped: bool,
    series: usize,
}

fn finish<'a>(t: &'a Target, series: &[Series], p: &Params) -> Done<'a> {
    let n = t.members.len() as f64;
    let rows = || t.members.iter().map(|m| &series[m.series].rows[m.row]);
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for r in rows() {
        let (a, b) = r.window.unwrap_or((r.rt - p.rt_tol, r.rt + p.rt_tol));
        lo = lo.min(a);
        hi = hi.max(b);
    }
    let (lo, hi) = (round(lo - p.pad, 4), round(hi + p.pad, 4));
    Done {
        t,
        mz: round(rows().map(|r| r.mz).sum::<f64>() / n, 5),
        rt: round(t.members.iter().map(|m| m.rt).sum::<f64>() / n, 4),
        lo: lo.max(0.0),
        hi,
        clamped: lo < 0.0,
        series: t.members.len(),
    }
}

fn members(d: &Done, series: &[Series]) -> Value {
    let first = &series[d.t.members[0].series].rows[d.t.members[0].row];
    Value::Arr(
        d.t.members
            .iter()
            .map(|m| {
                let r = &series[m.series].rows[m.row];
                Obj::new()
                    .put("series", text(&series[m.series].name))
                    .put("id", text(&r.id))
                    .put("line", int(r.line as i64))
                    .n("mz", round(r.mz, 5))
                    .n("rt", round(r.rt, 4))
                    .n("rt_warped", round(m.rt, 4))
                    .n("delta_mz", round(r.mz - first.mz, 5))
                    .n("delta_rt", round(m.rt - d.t.rt0, 4))
                    .done()
            })
            .collect(),
    )
}

fn fix(v: f64, d: usize) -> String {
    let s = format!("{v:.d$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// The ids and the rows of the conflicts: kept targets of one polarity that stand within `da` and `rt_tol` of each other (they could not merge).
fn conflicts(kept: &[(String, &Done)], p: &Params) -> Vec<Value> {
    let mut out = Vec::new();
    for (i, (a, x)) in kept.iter().enumerate() {
        for (b, y) in kept[i + 1..]
            .iter()
            .take_while(|(_, y)| y.t.polarity == x.t.polarity && y.mz - x.mz <= p.da)
        {
            if (y.rt - x.rt).abs() <= p.rt_tol {
                out.push(
                    Obj::new()
                        .put("a", text(a))
                        .put("b", text(b))
                        .n("delta_mz", round(y.mz - x.mz, 5))
                        .n("delta_rt", round(y.rt - x.rt, 4))
                        .done(),
                );
            }
        }
    }
    out
}

fn choice(what: &str, series: &str, detail: Vec<(&str, Value)>) -> Value {
    let o = Obj::new()
        .put("what", text(what))
        .put("series", text(series));
    detail.into_iter().fold(o, |o, (k, v)| o.put(k, v)).done()
}

fn series_info(series: &[Series], m: &Merged, p: &Params, choices: &mut Vec<Value>) -> Value {
    let mut out = Vec::new();
    for (k, s) in series.iter().enumerate() {
        let joined = m
            .targets
            .iter()
            .flat_map(|t| t.members.iter().skip(1))
            .filter(|x| x.series == k)
            .count();
        let warp = &m.warps[k];
        let mut w = Obj::new().put(
            "method",
            text(if k == 0 { "reference" } else { p.warp_name() }),
        );
        if k > 0 && p.warp == super::params::Warp::Piecewise {
            let anchors = warp.curve.as_ref().map_or(0, |c| c.anchors);
            w = w
                .i("anchors", anchors as i64)
                .b("applied", warp.curve.is_some());
            if let Some(c) = &warp.curve {
                let (med, max) = c.size();
                w = w
                    .n("median_shift", round(med, 4))
                    .n("max_shift", round(max, 4));
                choices.push(choice(
                    "rt_warp",
                    &s.name,
                    vec![
                        ("anchors", int(anchors as i64)),
                        ("max_shift", num(round(max, 4))),
                    ],
                ));
            } else {
                choices.push(choice(
                    "rt_warp_skipped",
                    &s.name,
                    vec![("anchors", int(anchors as i64))],
                ));
            }
        }
        if s.windowless > 0 {
            choices.push(choice(
                "window_from_rt_tol",
                &s.name,
                vec![("rows", int(s.windowless as i64))],
            ));
        }
        if s.polarity_set > 0 {
            choices.push(choice(
                "polarity_from_param",
                &s.name,
                vec![
                    ("rows", int(s.polarity_set as i64)),
                    ("value", text(pol_text(p.polarity))),
                ],
            ));
        }
        out.push(
            Obj::new()
                .put("name", text(&s.name))
                .i("rows", s.rows.len() as i64)
                .i("joined", joined as i64)
                .i("new", (s.rows.len() - joined) as i64)
                .put("warp", w.done())
                .done(),
        );
    }
    Value::Arr(out)
}

fn params_value(p: &Params) -> Value {
    Obj::new()
        .n("da", p.da)
        .n("rt_tol", p.rt_tol)
        .n("rt_gap", p.rt_gap)
        .put("rt_warp", text(p.warp_name()))
        .i("min_series", p.min_series as i64)
        .n("pad", p.pad)
        .put("polarity", text(pol_text(p.polarity)))
        .done()
}

/// The target rows of the CSV and of the document, in the order of `named`.
fn target_rows(named: &[(String, &Done)], series: &[Series]) -> (String, Vec<Value>) {
    let mut csv = String::from("id,mz,rt_min,rt_max,polarity,role\n");
    let mut targets = Vec::new();
    for (id, d) in named {
        csv += &format!(
            "{id},{},{},{},{},target\n",
            fix(d.mz, 5),
            fix(d.lo, 4),
            fix(d.hi, 4),
            pol_text(d.t.polarity)
        );
        targets.push(
            Obj::new()
                .put("id", text(id))
                .n("mz", d.mz)
                .n("rt", d.rt)
                .n("rt_min", d.lo)
                .n("rt_max", d.hi)
                .put("polarity", pol_value(d.t.polarity))
                .i("series", d.series as i64)
                .put("members", members(d, series))
                .done(),
        );
    }
    (csv, targets)
}

fn dropped_value(dropped: &[&Done], series: &[Series]) -> Vec<Value> {
    dropped
        .iter()
        .map(|d| {
            Obj::new()
                .n("mz", d.mz)
                .n("rt", d.rt)
                .put("polarity", pol_value(d.t.polarity))
                .i("series", d.series as i64)
                .put("reason", text("min_series"))
                .put("members", members(d, series))
                .done()
        })
        .collect()
}

/// Targets in output order: polarity, m/z, RT, then the first member (a total order, so the bytes never depend on the input order).
fn sorted<'a>(m: &'a Merged, series: &[Series], p: &Params) -> Vec<Done<'a>> {
    let mut all: Vec<Done> = m.targets.iter().map(|t| finish(t, series, p)).collect();
    crate::sortc::slice_by(&mut all, |a, b| {
        pol_rank(a.t.polarity)
            .cmp(&pol_rank(b.t.polarity))
            .then(a.mz.total_cmp(&b.mz))
            .then(a.rt.total_cmp(&b.rt))
            .then(
                (a.t.members[0].series, a.t.members[0].row)
                    .cmp(&(b.t.members[0].series, b.t.members[0].row)),
            )
    });
    all
}

pub fn document(series: &[Series], m: &Merged, p: &Params) -> String {
    let all = sorted(m, series, p);
    let (kept, dropped): (Vec<&Done>, Vec<&Done>) =
        all.iter().partition(|d| d.series >= p.min_series);
    let width = kept.len().to_string().len().max(4);
    let named: Vec<(String, &Done)> = kept
        .iter()
        .enumerate()
        .map(|(i, d)| (format!("U{:0width$}", i + 1), *d))
        .collect();
    let mut choices = Vec::new();
    let series_v = series_info(series, m, p, &mut choices);
    let (csv, targets) = target_rows(&named, series);
    let clamped = named.iter().filter(|(_, d)| d.clamped).count();
    if clamped > 0 {
        choices.push(choice(
            "rt_min_clamped",
            "",
            vec![("targets", int(clamped as i64))],
        ));
    }
    let drop_v = dropped_value(&dropped, series);
    let conf = conflicts(&named, p);
    let rows_in: usize = series.iter().map(|s| s.rows.len()).sum();
    let counts = Obj::new()
        .i("rows_in", rows_in as i64)
        .i("targets", named.len() as i64)
        .i(
            "merged",
            named.iter().filter(|(_, d)| d.series > 1).count() as i64,
        )
        .i("dropped", drop_v.len() as i64)
        .i("conflicts", conf.len() as i64)
        .done();
    write(
        &Obj::new()
            .put("schema", text(super::SCHEMA))
            .put("params", params_value(p))
            .put("counts", counts)
            .put("series", series_v)
            .put("targets", Value::Arr(targets))
            .put("conflicts", Value::Arr(conf))
            .put("dropped", Value::Arr(drop_v))
            .put("choices", Value::Arr(choices))
            .put("csv", text(&csv))
            .done(),
    )
}
