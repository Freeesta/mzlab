//! The scan table as the JSON rows of `SCAN_COLUMNS` (what the `.mzidx` stores), written straight from the rows: `Table::rows_json` for a table that stays, `Table::drain_json` when
//! the rows can be freed as they are written.

use super::{Row, Table};
use crate::json::Out;

/// Columns 21 (type) and 22 (mobility `[value, unit]`) only for the scans that have one, so an index without any stays as it was.
fn tail_columns(o: &mut Out, r: &Row) {
    if r.kind.is_none() && r.mob.is_none() {
        return;
    }
    o.raw(",");
    match r.kind {
        Some(k) => o.str(k),
        None => o.raw("null"),
    };
    if let Some((v, u)) = r.mob {
        o.raw(",[");
        py_float(o, v);
        o.raw(",").str(u).raw("]");
    }
}

fn py_float(o: &mut Out, v: f64) {
    if !v.is_finite() {
        o.raw("null");
        return;
    }
    let s = format!("{v:?}");
    if let Some((m, e)) = s.split_once('e') {
        let n: i32 = e.parse().unwrap_or(0);
        o.raw(&format!(
            "{m}e{}{:02}",
            if n < 0 { '-' } else { '+' },
            n.abs()
        ));
    } else {
        o.raw(&s);
    }
}

fn opt(o: &mut Out, v: Option<f64>) {
    match v {
        Some(x) => py_float(o, x),
        None => {
            o.raw("null");
        }
    }
}

/// Size of the pieces in which `drain_json` writes.
const PIECE: usize = 1 << 20;
/// Room kept above `PIECE` for the row that crosses it, so that a piece never doubles.
const SLACK: usize = 1 << 16;

impl Table {
    /// The rows of `SCAN_COLUMNS` as compact JSON (`json.dumps(rows, separators=(",", ":"))`); a row of a vendor file (`k` set) starts with
    /// `[membro,offset,lunghezza]` or `null`, and `k` (version 2, `docs/agenti/mzidx.md`).
    pub fn rows_json(&self) -> String {
        let mut o = Out::default();
        o.raw("[");
        for (i, r) in self.rows.iter().enumerate() {
            o.raw(if i > 0 { ",[" } else { "[" });
            write_row(&mut o, r);
        }
        o.raw("]");
        o.0
    }

    /// `rows_json` that frees each row as soon as it is written, and the four columns the index builder needs
    /// (levels, polarities, RT, TIC): the rows and their JSON never coexist whole, which is a third of the peak for a run of 100000 scans.
    /// The JSON is written in pieces of about `PIECE` bytes and joined at the end, when the rows are gone: a vector that doubles would hold the old and
    /// the new buffer together at the point where it is largest.
    pub fn drain_json(&mut self) -> (String, Columns) {
        let rows = std::mem::take(&mut self.rows);
        let n = rows.len();
        let mut c = Columns::default();
        c.level.reserve_exact(n);
        c.polarity.reserve_exact(n);
        c.rt.reserve_exact(n);
        c.tic.reserve_exact(n);
        let mut pieces: Vec<String> = Vec::new();
        let mut o = Out(String::with_capacity(PIECE + SLACK));
        o.raw("[");
        let mut total = 1;
        for (i, r) in rows.into_iter().enumerate() {
            if o.0.len() >= PIECE {
                total += o.0.len();
                pieces.push(std::mem::replace(
                    &mut o.0,
                    String::with_capacity(PIECE + SLACK),
                ));
            }
            o.raw(if i > 0 { ",[" } else { "[" });
            write_row(&mut o, &r);
            c.level.push(r.level);
            c.polarity.push(r.polarity);
            c.rt.push(r.rt);
            c.tic.push(r.tic);
        }
        total += o.0.len() + 1;
        pieces.push(o.0);
        let mut all = String::with_capacity(total);
        for p in pieces {
            all.push_str(&p);
        }
        all.push(']');
        (all, c)
    }
}

/// The columns of the scan table that the index builder reads (`mzidx::ScanTable`).
#[derive(Debug, Default)]
pub struct Columns {
    pub level: Vec<u8>,
    pub polarity: Vec<i8>,
    pub rt: Vec<f64>,
    pub tic: Vec<f64>,
}

/// One row as the JSON array of `SCAN_COLUMNS` (without the separator before it).
fn write_row(o: &mut Out, r: &Row) {
    write_where(o, r);
    o.raw(",").int(r.level as i64).raw(",");
    py_float(o, r.rt);
    o.raw(",").int(r.polarity as i64).raw(",");
    py_float(o, r.tic);
    o.raw(",");
    opt(o, r.d().precursor);
    o.raw(",");
    opt(o, r.d().ce);
    o.raw(",")
        .str(&r.filter)
        .raw(if r.profile { ",true," } else { ",false," });
    match r.parent {
        Some(p) => o.int(p as i64),
        None => o.raw("null"),
    };
    o.raw(",");
    match r.d().iso {
        Some((a, b)) => {
            o.raw("[");
            py_float(o, a);
            o.raw(",");
            py_float(o, b);
            o.raw("]");
        }
        None => {
            o.raw("null");
        }
    }
    o.raw(",");
    match &r.d().act {
        Some(a) => o.str(a),
        None => o.raw("null"),
    };
    o.raw(",");
    opt(o, r.res);
    o.raw(",").str(r.an).raw(",");
    opt(o, r.d().pint);
    o.raw(",");
    match &r.d().path {
        Some(p) => {
            o.raw("[");
            for (j, (m, a, c)) in p.iter().enumerate() {
                if j > 0 {
                    o.raw(",");
                }
                o.raw("[");
                py_float(o, *m);
                o.raw(",").str(a).raw(",");
                py_float(o, *c);
                o.raw("]");
            }
            o.raw("]");
        }
        None => {
            o.raw("null");
        }
    }
    write_precs(o, r);
    match r.d().charge {
        Some(c) => o.int(c),
        None => o.raw("null"),
    };
    o.raw(",").str(&r.native);
    tail_columns(o, r);
    o.raw("]");
}

/// The precursors of a row as the JSON list of objects, followed by the comma that goes before the next column.
fn write_precs(o: &mut Out, r: &Row) {
    o.raw(",[");
    for (j, e) in r.d().precs.iter().enumerate() {
        if j > 0 {
            o.raw(",");
        }
        o.raw("{\"target\":");
        opt(o, e.target);
        o.raw(",\"lo\":");
        opt(o, e.lo);
        o.raw(",\"hi\":");
        opt(o, e.hi);
        o.raw(",\"selected\":");
        opt(o, e.selected);
        o.raw(",\"charge\":");
        match e.charge {
            Some(c) => o.int(c),
            None => o.raw("null"),
        };
        o.raw(",\"ref\":");
        match &e.refer {
            Some(s) => o.str(s),
            None => o.raw("null"),
        };
        o.raw(",\"act\":");
        match e.act {
            Some(s) => o.str(s),
            None => o.raw("null"),
        };
        o.raw(",\"ce\":");
        opt(o, e.ce);
        o.raw(",\"pint\":");
        opt(o, e.pint);
        o.raw("}");
    }
    o.raw("],");
}

/// Where the scan lies: the span of the mzML, or the place in a vendor file set.
fn write_where(o: &mut Out, r: &Row) {
    match r.dove.as_deref().map(|d| (d.k, d.posizione)) {
        None => o.int(r.start as i64).raw(",").int(r.end as i64),
        Some((k, None)) => o.raw("null,").int(k as i64),
        Some((k, Some(p))) => {
            let (m, n) = (i64::from(p.membro), i64::from(p.lunghezza));
            o.raw("[")
                .int(m)
                .raw(",")
                .int(p.offset as i64)
                .raw(",")
                .int(n)
                .raw("],")
                .int(k as i64)
        }
    };
}
