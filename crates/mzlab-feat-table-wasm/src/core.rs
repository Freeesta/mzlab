//! The steps of the bridge on any `ReadAt`, shared with the native example `tabella`, so the browser and the desktop write the same bytes.
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;
use mzlab_feat::export::{self, Ctx};
use mzlab_feat::export_ms;
use mzlab_feat::ms2spec;
use mzlab_feat::report;
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

/// A table file: `kind` is `full` (CSV full for mzmine), `full_mzmine` (the same with mzmine's words for the filled and targeted cells) or `quant` (FBMN).
pub fn table_file<R: ReadAt>(r: &mut R, size: u64, samples_json: &str, spec: &str, table_sha256: &str, polarity: i32, kind: &str) -> Result<String> {
    let (t, s) = load(r, size, samples_json)?;
    let v = view_of(&t, &s, spec)?;
    let c = ctx(&t, &s, v.as_ref(), table_sha256, polarity);
    match kind {
        "full" => Ok(export::csv_full(&c, false)),
        "full_mzmine" => Ok(export::csv_full(&c, true)),
        "quant" => Ok(export::quant(&c)),
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
