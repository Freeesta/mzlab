//! The windows of `spectra.match` / `table.match` that follow the resolution of the query when the plan writes none (the same Rust in the browser and in `flow run`).

use mzlab_core::json::Value;

/// Fragment window (Da) of a unit-resolution query that the plan did not write: the tolerance of the unit MGF (`export_ms::mgf_unit`), a little under a mass unit.
pub const UNIT_FRAG_DA: f64 = 0.5;
/// Precursor window (Da) of a unit-resolution query that the plan did not write: the 0.7 FWHM peak width of the 3200 QTRAP quadrupoles.
pub const UNIT_PRECURSOR_DA: f64 = 0.7;
/// Fragment window (Da) of a query that is not unit resolution and a plan that did not write one, the GNPS convention.
const HR_FRAG_DA: f64 = 0.02;

/// A query written by the unit-resolution export carries the note `# unit resolution` at its top (`export_ms::mgf_unit`; `table.match` writes it for unit-resolution files).
pub fn is_unit_query(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(2048)];
    head.windows(17).any(|w| w == b"# unit resolution")
}

/// The parameters with the windows the plan left at 0 (`tol_da`, and `precursor_da` for a unit-resolution query) set to the values that fit the resolution of the query.
pub fn follow(params: &[(String, Value)], unit: bool) -> Vec<(String, Value)> {
    let mut out: Vec<(String, Value)> = params.to_vec();
    let mut set = |name: &str, x: f64| {
        if let Some(v) = out
            .iter_mut()
            .find(|(k, v)| k == name && v.f64() == Some(0.0))
        {
            v.1 = Value::Num(x);
        }
    };
    set("tol_da", if unit { UNIT_FRAG_DA } else { HR_FRAG_DA });
    if unit {
        set("precursor_da", UNIT_PRECURSOR_DA);
    }
    out
}
