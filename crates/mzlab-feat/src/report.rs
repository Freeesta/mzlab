//! The report of a table (`report/1`, JSON, numbers only): what the series is, what the view kept and the quality control. It is the text of the
//! header of the exports and what the page shows; it holds no verdict. The SHA-256 of the table is given by the caller (the one of the `.mzfeat`).

use mzlab_core::json::Out;

use crate::aligned::Aligned;
use crate::qc;
use crate::samples::{self, Sample};
use crate::view::View;

pub const SCHEMA: &str = "report/1";

/// The `report/1` document of a table under a view (None = the whole table, areas as measured).
pub fn to_json(t: &Aligned, samples: &[Sample], view: Option<&View>, table_sha256: &str) -> String {
    let mut j = Out::default();
    j.raw("{\"schema\":")
        .str(SCHEMA)
        .raw(",\"crate\":")
        .str(env!("CARGO_PKG_VERSION"));
    j.raw(",\"tabella_sha256\":")
        .str(table_sha256)
        .raw(",\"campioni_sha256\":")
        .str(&samples::digest(samples));
    j.raw(",\"righe\":")
        .int(t.len() as i64)
        .raw(",\"file\":")
        .int(t.n_files as i64);
    let all = qc::per_file(t);
    let sum = |f: fn(&qc::FileQc) -> usize| all.iter().map(f).sum::<usize>() as i64;
    j.raw(",\"celle\":{\"rilevate\":")
        .int(sum(|q| q.detected))
        .raw(",\"mirate\":")
        .int(sum(|q| q.targeted));
    j.raw(",\"riempite\":")
        .int(sum(|q| q.filled))
        .raw(",\"assenti\":")
        .int(sum(|q| q.absent))
        .raw("}");
    j.raw(",\"vista\":");
    match view {
        Some(v) => j
            .raw("{\"parametri\":")
            .raw(&v.spec_json())
            .raw(",\"conteggi\":")
            .raw(&v.counts_json())
            .raw("}"),
        None => j.raw("null"),
    };
    j.raw(",\"qc\":")
        .raw(&qc::to_json(t, samples, view))
        .raw("}");
    j.0
}

/// The lines (`# ...`) that open an export: program, table, view and samples, so the file says what it comes from.
pub fn header_lines(
    t: &Aligned,
    samples: &[Sample],
    view: Option<&View>,
    table_sha256: &str,
) -> Vec<String> {
    let mut v = vec![
        format!(
            "# Tabui feature table, mzlab-feat {}",
            env!("CARGO_PKG_VERSION")
        ),
        format!(
            "# table SHA-256 {table_sha256} · {} rows × {} files",
            t.len(),
            t.n_files
        ),
        format!("# samples SHA-256 {}", samples::digest(samples)),
    ];
    if let Some(w) = view {
        v.push(format!("# view {} · {} rows kept", w.spec_json(), w.len()));
    }
    v
}
