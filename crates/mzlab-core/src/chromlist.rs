//! The chromatogram list of an mzML as the `.mzidx` keeps it (`run.chromatograms`, `docs/agenti/mzidx.md`): one row per `Chrom`,
//! `[id, start, end, kind, q1, q3, ce, name, dwell, n]`, in file order (the position is `Chrom::index`). `Index::open` hands it to the
//! `Layout`, so opening the file does not scan the tail for it.
use crate::json::{Out, Value};
use crate::mzmlhead::Chrom;

/// `,"chromatograms":[...]` for the `run` object; nothing for a file with none.
pub fn run_part(chroms: &[Chrom]) -> String {
    if chroms.is_empty() {
        return String::new();
    }
    let mut o = Out::default();
    o.raw(",\"chromatograms\":[");
    for (i, c) in chroms.iter().enumerate() {
        o.raw(if i > 0 { ",[" } else { "[" })
            .str(&c.id)
            .raw(",")
            .int(c.start as i64)
            .raw(",")
            .int(c.end as i64)
            .raw(",")
            .str(&c.kind)
            .raw(",")
            .opt(c.q1)
            .raw(",")
            .opt(c.q3)
            .raw(",")
            .opt(c.ce)
            .raw(",")
            .str(&c.name)
            .raw(",")
            .opt(c.dwell)
            .raw(",");
        match c.n {
            Some(n) => o.int(n),
            None => o.raw("null"),
        };
        o.raw("]");
    }
    o.raw("]");
    o.0
}

/// The list in `run`, `None` when the index has none (written before the list was kept, or a file with no chromatograms) or a row is not one.
pub fn of_run(run: &Value) -> Option<Vec<Chrom>> {
    let rows = run.get("chromatograms")?.arr();
    let mut out = Vec::with_capacity(rows.len());
    for (index, r) in rows.iter().enumerate() {
        let num = |k: usize| r.at(k).and_then(Value::f64);
        out.push(Chrom {
            index,
            id: r.at(0)?.str()?.to_string(),
            start: num(1)? as u64,
            end: num(2)? as u64,
            kind: r.at(3)?.str()?.to_string(),
            q1: num(4),
            q3: num(5),
            ce: num(6),
            name: r.at(7)?.str()?.to_string(),
            dwell: num(8),
            n: num(9).map(|v| v as i64),
        });
    }
    (!out.is_empty()).then_some(out)
}
