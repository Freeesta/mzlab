//! The steps of the bridge on any `ReadAt`, shared with the native example `tabella`, so the browser and the desktop write the same bytes.
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;
use mzlab_feat::export::{self, Ctx};
use mzlab_feat::export_ms;
use mzlab_feat::ms2spec;
use mzlab_feat::qc;
use mzlab_feat::report;
use mzlab_feat::riga;
use mzlab_feat::samples::{self, Sample};
use mzlab_feat::store::read_table_from;
use mzlab_feat::table::Aligned;
use mzlab_feat::view::{self, View};

/// The table and the samples; the samples must be one per file.
fn load<R: ReadAt>(r: &mut R, size: u64, samples_json: &str) -> Result<(Aligned, Vec<Sample>)> {
    let (t, _) = read_table_from(r, size)?;
    let s = samples::from_json(samples_json.as_bytes())?;
    if s.len() != t.n_files {
        return Err(Error::new("error.badView"));
    }
    Ok((t, s))
}

/// The view of the parameters `spec` (`{"normalizza":..., "filtra":[...]}`); an empty text is no view.
fn view_of(t: &Aligned, s: &[Sample], spec: &str) -> Result<Option<View>> {
    if spec.trim().is_empty() {
        return Ok(None);
    }
    view::from_spec(t, s, spec.as_bytes()).map(Some)
}

fn ctx<'a>(t: &'a Aligned, s: &'a [Sample], v: Option<&'a View>, sha: &'a str, polarity: i32) -> Ctx<'a> {
    Ctx { table: t, samples: s, view: v, table_sha256: sha, polarity: if polarity < 0 { -1 } else { 1 } }
}

/// The view of a table (an empty `spec` keeps every row, areas as measured): `{"digest", "righe":[id of every row kept], "fattori":[per file], "conteggi":{...}}`.
pub fn view_json<R: ReadAt>(r: &mut R, size: u64, samples_json: &str, spec: &str) -> Result<String> {
    let (t, s) = load(r, size, samples_json)?;
    let spec = if spec.trim().is_empty() { "{}" } else { spec };
    let v = view::from_spec(&t, &s, spec.as_bytes())?;
    let mut o = Out::default();
    o.raw("{\"digest\":").str(&v.digest(&t)).raw(",\"righe\":[");
    for (i, &row) in v.rows.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).int(i64::from(t.id(row as usize)));
    }
    o.raw("],\"fattori\":").floats(&v.factors);
    o.raw(",\"conteggi\":").raw(&v.counts_json()).raw("}");
    Ok(o.0)
}

/// The `report/1` of a table under the view `spec` (empty = the whole table).
pub fn report_json<R: ReadAt>(r: &mut R, size: u64, samples_json: &str, spec: &str, table_sha256: &str) -> Result<String> {
    let (t, s) = load(r, size, samples_json)?;
    let v = view_of(&t, &s, spec)?;
    Ok(report::to_json(&t, &s, v.as_ref(), table_sha256))
}

/// The `tptab/1` of a table: the rows of the JSON list `rows_json` (indices of the table), or, empty, the rows of the view `tp/1`, with their numbers and the controls.
pub fn tp_table_json<R: ReadAt>(r: &mut R, size: u64, samples_json: &str, rows_json: &str) -> Result<String> {
    let (t, s) = load(r, size, samples_json)?;
    let rows: Option<Vec<usize>> = if rows_json.trim().is_empty() {
        None
    } else {
        let v = mzlab_core::json::parse(rows_json.as_bytes())?;
        Some(v.arr().iter().filter_map(|x| x.f64()).map(|x| x as usize).collect())
    };
    mzlab_feat::tp::table_json(&t, &s, rows.as_deref())
}

/// The `campioni/1` that `propose` makes from the names of the files: `{"files":[{"name":..,"sha256":..}, ...]}` (type, time, replicate and group are read from the names).
pub fn samples_json(files_json: &str) -> Result<String> {
    let v = mzlab_core::json::parse(files_json.as_bytes())?;
    let files: Vec<(String, String)> = v
        .get("files")
        .map(|a| a.arr())
        .unwrap_or_default()
        .iter()
        .map(|f| (f.get("name").and_then(|x| x.str()).unwrap_or("").to_string(), f.get("sha256").and_then(|x| x.str()).unwrap_or("").to_string()))
        .collect();
    Ok(samples::to_json(&samples::propose(&files)))
}

/// The `qc/1` of a table under the view `spec` (empty = the whole table): per file the cells and the total, per condition the agreement of the replicates, the standard.
pub fn qc_json<R: ReadAt>(r: &mut R, size: u64, samples_json: &str, spec: &str) -> Result<String> {
    let (t, s) = load(r, size, samples_json)?;
    let v = view_of(&t, &s, spec)?;
    Ok(qc::to_json(&t, &s, v.as_ref()))
}

/// A table file: `kind` is `full` (CSV full for mzmine), `full_mzmine` (the same with mzmine's words for the filled and targeted cells), `quant` (FBMN),
/// `csv_semicolon` (the sheet of features as text with `;`) or `xlsx_rows` (the same sheet as JSON rows for the page to zip).
pub fn table_file<R: ReadAt>(r: &mut R, size: u64, samples_json: &str, spec: &str, table_sha256: &str, polarity: i32, kind: &str) -> Result<String> {
    let (t, s) = load(r, size, samples_json)?;
    let v = view_of(&t, &s, spec)?;
    let c = ctx(&t, &s, v.as_ref(), table_sha256, polarity);
    match kind {
        "full" => Ok(export::csv_full(&c, false)),
        "full_mzmine" => Ok(export::csv_full(&c, true)),
        "quant" => Ok(export::quant(&c)),
        "csv_semicolon" => Ok(export::csv_semicolon(&c)),
        "xlsx_rows" => Ok(export::xlsx_rows(&c)),
        _ => Err(Error::new("error.badView")),
    }
}

/// An MS2 file: `kind` is `mgf` (FBMN), `mgf_unit` (the same with the note of unit resolution; there is no `sirius` version) or `sirius` (`.ms`). `open(k)` gives the index of file `k`, which holds the best MS2 of some rows.
#[allow(clippy::too_many_arguments)]
pub fn ms2_file<R: ReadAt, I: ReadAt>(
    r: &mut R,
    size: u64,
    samples_json: &str,
    spec: &str,
    table_sha256: &str,
    polarity: i32,
    kind: &str,
    open: impl FnMut(usize) -> Result<Index<I>>,
) -> Result<String> {
    let (t, s) = load(r, size, samples_json)?;
    let v = view_of(&t, &s, spec)?;
    let c = ctx(&t, &s, v.as_ref(), table_sha256, polarity);
    let sp = ms2spec::spectra(&c, open)?;
    match kind {
        "mgf" => Ok(export_ms::mgf(&c, &sp)),
        "mgf_unit" => Ok(export_ms::mgf_unit(&c, &sp)),
        "sirius" => Ok(export_ms::sirius(&c, &sp)),
        _ => Err(Error::new("error.badView")),
    }
}

/// The `riga/1` of the row with `id` (`error.rowNotFound` when the table has none).
pub fn row_json<R: ReadAt>(r: &mut R, size: u64, samples_json: &str, id: u32) -> Result<String> {
    let (t, s) = load(r, size, samples_json)?;
    riga::row(&t, &s, id)
}

/// The `riga_tracce/1` of the row with `id`, read from the index of every file (`open(k)`), within `ppm` of its m/z.
pub fn row_traces_json<R: ReadAt, I: ReadAt>(r: &mut R, size: u64, samples_json: &str, id: u32, ppm: f64, open: impl FnMut(usize) -> Result<Index<I>>) -> Result<String> {
    let (t, s) = load(r, size, samples_json)?;
    riga::traces(&t, &s, id, open, ppm)
}

/// `campioni/2`: `op` is `proponi`, `valida`, `da_legacy` or `per_tp` (`mzlab_core::campioni::route`).
pub fn campioni_json(op: &str, input: &str) -> Result<String> {
    mzlab_core::campioni::route(op, input)
}
