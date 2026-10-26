//! The checks `campioni/3` adds to `valida`: free columns (`missingValue`, `typeMismatch`), the key of a file (`duplicateKey`) and the two
//! polarities of a physical sample (`pairMissing`, `pairMismatch`). Stable codes; the pages put the words.

use super::{idcampione, valida, Avviso, Campione, Tabella};

fn push(
    out: &mut Vec<Avviso>,
    code: &'static str,
    severity: &'static str,
    i: usize,
    c: &Campione,
    column: &str,
) {
    out.push(Avviso {
        code,
        severity,
        file: Some(i),
        series: c.series.clone(),
        column: column.to_string(),
    });
}

/// `typeMismatch` on `tempo` (error) when the ID has a time part that is not a number, `unknownCondition` (warning, column `serie`) when
/// the ID has the shape `MAT-COND-Ttt` with a condition the series does not use.
fn ids(out: &mut Vec<Avviso>, cs: &[Campione]) {
    for (i, c) in cs.iter().enumerate() {
        let info = idcampione::leggi(&c.sample_id);
        if info.bad_time {
            push(out, "typeMismatch", "error", i, c, "tempo");
        }
        if info.unknown_cond {
            push(out, "unknownCondition", "warning", i, c, "serie");
        }
    }
}

/// `duplicateKey` (error) on every file after the first with the same SHA-256 (when known) or the same name, and the same sample inside it.
fn duplicates(out: &mut Vec<Avviso>, cs: &[Campione]) {
    for (i, c) in cs.iter().enumerate() {
        let same = |o: &Campione| {
            o.sub == c.sub
                && if c.sha256.is_empty() || o.sha256.is_empty() {
                    o.file == c.file && !c.file.is_empty()
                } else {
                    o.sha256 == c.sha256
                }
        };
        if cs[..i].iter().any(same) {
            push(out, "duplicateKey", "error", i, c, "");
        }
    }
}

fn columns(out: &mut Vec<Avviso>, t: &Tabella) {
    for col in &t.colonne {
        for (i, c) in t.campioni.iter().enumerate() {
            match c.extra_of(&col.name) {
                None if col.required => push(out, "missingValue", "error", i, c, &col.name),
                Some(v) if !col.accepts(v) => push(out, "typeMismatch", "error", i, c, &col.name),
                _ => {}
            }
        }
    }
}

/// The files of one `campione_id` with a polarity: `pairMissing` (warning, on the first) when only one polarity is there, `pairMismatch`
/// (warning, on the first file of the other polarity) when positive and negative differ in time, role or condition.
fn pairs(out: &mut Vec<Avviso>, cs: &[Campione]) {
    let mut ids: Vec<&str> = Vec::new();
    for c in cs
        .iter()
        .filter(|c| !c.sample_id.is_empty() && c.polarity.is_some())
    {
        if !ids.contains(&c.sample_id.as_str()) {
            ids.push(&c.sample_id);
        }
    }
    for id in ids {
        let of = |p: i8| {
            cs.iter()
                .position(|c| c.sample_id == id && c.polarity == Some(p))
        };
        match (of(1), of(-1)) {
            (Some(a), Some(b)) => {
                let (x, y) = (&cs[a], &cs[b]);
                if x.time_min != y.time_min || x.role() != y.role() || x.series != y.series {
                    let i = a.max(b);
                    push(out, "pairMismatch", "warning", i, &cs[i], "");
                }
            }
            (Some(i), None) | (None, Some(i)) => push(out, "pairMissing", "warning", i, &cs[i], ""),
            (None, None) => {}
        }
    }
}

/// The findings of `valida` plus the ones of `campioni/3`, in that order.
pub fn valida3(t: &Tabella) -> Vec<Avviso> {
    let mut out = valida(&t.campioni);
    duplicates(&mut out, &t.campioni);
    columns(&mut out, t);
    ids(&mut out, &t.campioni);
    pairs(&mut out, &t.campioni);
    out
}
