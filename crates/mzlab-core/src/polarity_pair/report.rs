//! The `pair/1` document: the pairs with the mass error, the RT difference and the rule of each, the rows left without a partner, the parameters, the
//! adducts used and every automatic choice, plus the same pairs as CSV (unmatched rows last, rule `unmatched`).

use super::pairs::Matched;
use super::params::Params;
use super::table::Row;
use crate::json::Value;
use crate::jsonb::{text, write, Obj};

fn round(v: f64, d: i32) -> f64 {
    let f = 10f64.powi(d);
    (v * f).round() / f
}

fn fix(v: f64, d: usize) -> String {
    let s = format!("{v:.d$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

fn cell(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn row_value(r: &Row) -> Value {
    Obj::new()
        .put("id", text(&r.id))
        .n("mz", round(r.mz, 5))
        .n("rt", round(r.rt, 4))
        .done()
}

fn adducts_value(list: &[super::params::Adduct]) -> Value {
    Value::Arr(
        list.iter()
            .map(|a| {
                Obj::new()
                    .put("name", text(&a.name))
                    .n("k", a.k)
                    .n("shift", round(a.shift, 6))
                    .n("z", a.z)
                    .done()
            })
            .collect(),
    )
}

fn choice(what: &str, detail: Vec<(&str, Value)>) -> Value {
    detail
        .into_iter()
        .fold(Obj::new().put("what", text(what)), |o, (k, v)| o.put(k, v))
        .done()
}

pub fn document(pos: &[Row], neg: &[Row], m: &Matched, p: &Params) -> String {
    let names = |l: &[super::params::Adduct]| {
        l.iter()
            .map(|a| a.name.clone())
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut csv =
        String::from("pos_id,neg_id,rule,neutral_mass,err_da,err_ppm,rt_pos,rt_neg,drt,mutual\n");
    let mut pairs = Vec::new();
    for x in &m.pairs {
        let rule = format!(
            "{} / {}",
            p.pos[x.pos_adduct].name, p.neg[x.neg_adduct].name
        );
        let (a, b) = (&pos[x.pos], &neg[x.neg]);
        csv += &format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            cell(&a.id),
            cell(&b.id),
            cell(&rule),
            fix(x.neutral(), 5),
            fix(x.err_da(), 5),
            fix(x.err_ppm(), 2),
            fix(a.rt, 4),
            fix(b.rt, 4),
            fix(x.drt, 4),
            x.mutual
        );
        pairs.push(
            Obj::new()
                .put("pos", text(&a.id))
                .put("neg", text(&b.id))
                .put("rule", text(&rule))
                .n("neutral_mass", round(x.neutral(), 5))
                .n("err_da", round(x.err_da(), 5))
                .n("err_ppm", round(x.err_ppm(), 2))
                .n("rt_pos", round(a.rt, 4))
                .n("rt_neg", round(b.rt, 4))
                .n("drt", round(x.drt, 4))
                .n("dist", round(x.dist, 3))
                .b("mutual", x.mutual)
                .i("pos_partners", x.pos_n as i64)
                .i("neg_partners", x.neg_n as i64)
                .done(),
        );
    }
    for &i in &m.pos_free {
        csv += &format!(
            "{},,unmatched,,,,{},,,\n",
            cell(&pos[i].id),
            fix(pos[i].rt, 4)
        );
    }
    for &i in &m.neg_free {
        csv += &format!(
            ",{},unmatched,,,,,{},,\n",
            cell(&neg[i].id),
            fix(neg[i].rt, 4)
        );
    }
    let free = |rows: &[Row], idx: &[usize]| {
        Value::Arr(idx.iter().map(|&i| row_value(&rows[i])).collect())
    };
    let params = Obj::new()
        .n("rt_tol", p.rt_tol)
        .n("mass_tol", p.mass_tol)
        .put("mass_unit", text(p.unit_name()))
        .put("pos_adducts", text(&names(&p.pos)))
        .put("neg_adducts", text(&names(&p.neg)))
        .done();
    let counts = Obj::new()
        .i("pos_rows", pos.len() as i64)
        .i("neg_rows", neg.len() as i64)
        .i("pairs", m.pairs.len() as i64)
        .i("mutual", m.pairs.iter().filter(|x| x.mutual).count() as i64)
        .i("pos_unmatched", m.pos_free.len() as i64)
        .i("neg_unmatched", m.neg_free.len() as i64)
        .done();
    let choices = vec![
        choice("rule", vec![("text", text("pair when |RT(neg) - RT(pos)| <= rt_tol and |M(neg) - M(pos)| <= mass_tol; M = (mz * z - shift) / k of the adduct"))]),
        choice("adducts", vec![("side", text("pos")), ("list", adducts_value(&p.pos))]),
        choice("adducts", vec![("side", text("neg")), ("list", adducts_value(&p.neg))]),
        choice("mass_error", vec![("basis", text("neutral mass of the pair (mean of the two), ppm or Da as mass_unit")), ("sign", text("negative minus positive"))]),
        choice("mutual", vec![("basis", text("nearest partner of both rows by the mass and RT errors scaled by their tolerances; every other pair within both tolerances is kept"))]),
    ];
    write(
        &Obj::new()
            .put("schema", text(super::SCHEMA))
            .put("params", params)
            .put("counts", counts)
            .put("pairs", Value::Arr(pairs))
            .put(
                "unmatched",
                Obj::new()
                    .put("pos", free(pos, &m.pos_free))
                    .put("neg", free(neg, &m.neg_free))
                    .done(),
            )
            .put("choices", Value::Arr(choices))
            .put("csv", text(&csv))
            .done(),
    )
}
