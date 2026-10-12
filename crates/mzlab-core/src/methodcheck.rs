//! `check_against` and `method_kind` of `mzlab/reader/methodinfo.py`: a method (`.dam`) against what an mzML says about its own acquisition.

use crate::json::{fmt_g, Out, Value};
use crate::methodinfo::{Kind, Method};

/// A text of a row: plain, or an i18n message `{"key", "params"}`.
pub enum Cell {
    S(String),
    M(&'static str, Vec<(&'static str, Param)>),
}

pub enum Param {
    S(String),
    I(i64),
}

pub struct Row {
    pub what: Cell,
    pub method: Cell,
    pub data: Cell,
    pub status: &'static str,
}

fn msg(key: &'static str) -> Cell {
    Cell::M(key, vec![])
}

fn s(x: impl Into<String>) -> Cell {
    Cell::S(x.into())
}

fn transition(q1: f64, q3: f64) -> Cell {
    Cell::M("chk.transition", vec![("q1", Param::S(fmt_g(q1))), ("q3", Param::S(fmt_g(q3)))])
}

impl Cell {
    fn write(&self, o: &mut Out) {
        match self {
            Cell::S(t) => {
                o.str(t);
            }
            Cell::M(k, ps) => {
                o.raw("{\"key\":").str(k).raw(",\"params\":{");
                for (i, (n, p)) in ps.iter().enumerate() {
                    o.raw(if i > 0 { "," } else { "" }).str(n).raw(":");
                    match p {
                        Param::S(t) => o.str(t),
                        Param::I(v) => o.int(*v),
                    };
                }
                o.raw("}}");
            }
        }
    }
}

pub fn write_rows(rows: &[Row], o: &mut Out) {
    o.raw("[");
    for (i, r) in rows.iter().enumerate() {
        o.raw(if i > 0 { ",{\"what\":" } else { "{\"what\":" });
        r.what.write(o);
        o.raw(",\"method\":");
        r.method.write(o);
        o.raw(",\"data\":");
        r.data.write(o);
        o.raw(",\"status\":").str(r.status).raw("}");
    }
    o.raw("]");
}

fn nums(v: Option<&Value>) -> Vec<f64> {
    v.map(|a| a.arr().iter().filter_map(Value::f64).collect()).unwrap_or_default()
}

fn truthy_num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::f64).filter(|x| *x != 0.0)
}

fn mrm_rows(rows: &mut Vec<Row>, m_tr: &[&crate::methodinfo::Transition], data: &Value) {
    let d_tr: Vec<&Value> = data.get("transitions").map(|a| a.arr().iter().collect()).unwrap_or_default();
    let mut used = vec![false; d_tr.len()];
    let g = |x: &Value, k: &str| x.get(k).and_then(Value::f64);
    for t in m_tr {
        let hit = (0..d_tr.len()).find(|&i| match (g(d_tr[i], "q1"), g(d_tr[i], "q3")) {
            (Some(a), Some(b)) => !used[i] && (a - t.q1).abs() <= 0.2 && (b - t.q3).abs() <= 0.2,
            _ => false,
        });
        let mtxt = match t.ce {
            Some(ce) => format!("CE {} eV, dwell {} ms", fmt_g(ce), fmt_g(t.dwell)),
            None => format!("dwell {} ms", fmt_g(t.dwell)),
        };
        let Some(h) = hit else {
            rows.push(Row { what: transition(t.q1, t.q3), method: s(mtxt), data: msg("chk.absentFile"), status: "diff" });
            continue;
        };
        used[h] = true;
        let x = d_tr[h];
        let (ce, dwell) = (g(x, "ce"), g(x, "dwell").map(|d| if d < 5.0 { d * 1000.0 } else { d }));
        let ok = ce.is_none_or(|c| t.ce.is_none_or(|m| (c - m).abs() < 0.5)) && dwell.is_none_or(|d| (d - t.dwell).abs() < 1.0);
        let got = match (ce, dwell) {
            (Some(c), Some(d)) => s(format!("CE {} eV, dwell {} ms", fmt_g(c), fmt_g(d))),
            _ => msg("chk.present"),
        };
        rows.push(Row { what: transition(t.q1, t.q3), method: s(mtxt), data: got, status: if ok { "ok" } else { "diff" } });
    }
    for (i, x) in d_tr.iter().enumerate() {
        if let (false, Some(q1)) = (used[i], g(x, "q1")) {
            rows.push(Row { what: transition(q1, g(x, "q3").unwrap_or(f64::NAN)), method: msg("chk.absentMethod"), data: msg("chk.presentFile"), status: "diff" });
        }
    }
}

/// Rows `{what, method, data, status}` (`ok`, `diff`, `na`) comparing the method `lab` with `data`, the `Item.method()` of the file.
pub fn check_against(data: &Value, lab: &Method) -> Vec<Row> {
    let mut rows = Vec::new();
    if lab.experiments.is_empty() {
        return rows;
    }
    let kind = data.get("kind").and_then(Value::str).unwrap_or("");
    let m_mrm = lab.experiments.iter().all(|e| matches!(e.kind, Kind::Mrm(_)));
    let d_name = match kind {
        "full" => msg("chk.type.full"),
        "ms2" => msg("chk.type.ms2"),
        "mrm" => s("MRM"),
        _ => msg("chk.type.unknown"),
    };
    let ok = (kind == "mrm") == m_mrm && ["full", "ms2", "mrm"].contains(&kind);
    rows.push(Row { what: msg("chk.type"), method: if m_mrm { s("MRM") } else { msg("chk.type.scan") }, data: d_name, status: if ok { "ok" } else { "diff" } });
    if kind == "mrm" && m_mrm {
        let m_tr: Vec<_> = lab.experiments.iter().flat_map(|e| if let Kind::Mrm(t) = &e.kind { t.iter().collect() } else { vec![] }).collect();
        mrm_rows(&mut rows, &m_tr, data);
    } else if (kind == "full" || kind == "ms2") && !m_mrm {
        let rngs: Vec<Option<[f64; 2]>> = lab.experiments.iter().map(|e| if let Kind::Scan { range, .. } = &e.kind { *range } else { None }).collect();
        let win = data.get("scan_window").filter(|w| !w.is_null()).map(|w| nums(Some(w)));
        let all = rngs.iter().all(Option::is_some);
        let w = win.as_ref().filter(|w| w.len() >= 2);
        if let (Some(w), true) = (w, all) {
            let lo = rngs.iter().flatten().map(|r| r[0]).fold(f64::INFINITY, f64::min);
            let hi = rngs.iter().flatten().map(|r| r[1]).fold(f64::NEG_INFINITY, f64::max);
            let ok = (lo - w[0]).abs() <= 1.0 && (hi - w[1]).abs() <= 1.0;
            rows.push(Row { what: msg("chk.massRange"), method: s(format!("{}-{}", fmt_g(lo), fmt_g(hi))), data: s(format!("{}-{}", fmt_g(w[0]), fmt_g(w[1]))), status: if ok { "ok" } else { "diff" } });
        } else if let Some(w) = w {
            rows.push(Row { what: msg("chk.massRange"), method: msg("chk.notDecodable"), data: s(format!("{}-{}", fmt_g(w[0]), fmt_g(w[1]))), status: "na" });
        }
        if kind == "ms2" {
            let mut ces: Vec<f64> = lab.experiments.iter().flat_map(|e| if let Kind::Scan { ce, .. } = &e.kind { ce.clone() } else { vec![] }).collect();
            ces.sort_by(f64::total_cmp);
            ces.dedup();
            let mut dce = nums(data.get("ce"));
            dce.sort_by(f64::total_cmp);
            if !ces.is_empty() && !dce.is_empty() {
                let ok = ces.len() == dce.len() && ces.iter().zip(&dce).all(|(a, b)| (a - b).abs() < 0.5);
                let j = |v: &[f64]| v.iter().map(|x| fmt_g(*x)).collect::<Vec<_>>().join(", ");
                rows.push(Row { what: msg("chk.ce"), method: s(j(&ces)), data: s(j(&dce)), status: if ok { "ok" } else { "diff" } });
            }
            let prec = nums(data.get("precursors"));
            if !prec.is_empty() {
                let list = prec.iter().map(|x| fmt_g(*x)).collect::<Vec<_>>().join(", ");
                rows.push(Row {
                    what: msg("chk.experiments"),
                    method: s(lab.experiments.len().to_string()),
                    data: Cell::M("chk.precursorsN", vec![("n", Param::I(prec.len() as i64)), ("list", Param::S(list))]),
                    status: if prec.len() == lab.experiments.len() { "ok" } else { "diff" },
                });
            }
        }
    }
    if let Some(lc) = &lab.lc {
        if let (Some(rt), Some(mx)) = (lc.run_time.filter(|r| *r != 0.0), truthy_num(data.get("rt_max"))) {
            rows.push(Row {
                what: msg("chk.runTime"),
                method: s(fmt_g(rt)),
                data: Cell::M("chk.reachesTo", vec![("t", Param::S(format!("{mx:.1}")))]),
                status: if mx <= rt + 0.1 { "ok" } else { "diff" },
            });
        }
        let has = data.get("pda").and_then(Value::bool).unwrap_or(false);
        rows.push(Row { what: msg("chk.pda"), method: msg("chk.pdaOn"), data: msg(if has { "chk.pdaSignal" } else { "chk.pdaNone" }), status: if has { "ok" } else { "diff" } });
    }
    rows
}

fn word_at(b: &[u8], i: usize, len: usize) -> bool {
    let w = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    (i == 0 || !w(b[i - 1])) && (i + len >= b.len() || !w(b[i + len]))
}

/// `\bword\b` anywhere in `name`.
fn has_word(name: &str, word: &str) -> bool {
    name.match_indices(word).any(|(i, _)| word_at(name.as_bytes(), i, word.len()))
}

/// Experiment type a method is for: `mrm` (decoded), `ms2` or `full` (only the NAME says), `None` when it cannot be said.
pub fn method_kind(lab: &Method) -> Option<&'static str> {
    if !lab.experiments.is_empty() && lab.experiments.iter().all(|e| matches!(e.kind, Kind::Mrm(_))) {
        return Some("mrm");
    }
    let n = lab.name.to_lowercase();
    let epi = n.match_indices("epi").any(|(i, _)| i + 3 >= n.len() || !(n.as_bytes()[i + 3].is_ascii_alphanumeric() || n.as_bytes()[i + 3] == b'_'));
    if ["ms2", "msms", "ms-ms", "product", "prodotti"].iter().any(|k| n.contains(k)) || epi {
        return Some("ms2");
    }
    if n.contains("full") || n.contains("scan") || ["q1", "ems", "ms1"].iter().any(|k| has_word(&n, k)) {
        return Some("full");
    }
    None
}
