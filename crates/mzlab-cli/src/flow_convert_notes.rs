//! What `convert.mzml` puts in the note of the file of the run record (`prov.json`, `report.json`, `flowrun/1`): the flagged centroids, the spectrum-type
//! decision and a partial read.
use mzlab_core::json::Value;
use mzlab_core::readers::Losses;

use crate::flow::Made;

/// The centroids of our own peak picking flagged merged and saturated in the note of the file (`--report` of `convert`); a merged one raises
/// `warn.centroid.merged`, a notice and not an answer.
pub(crate) fn note_flagged(made: &mut Made, (merged, saturated): (usize, usize)) {
    let counts = [
        ("merged".to_string(), Value::Num(merged as f64)),
        ("saturated".to_string(), Value::Num(saturated as f64)),
    ];
    match &mut made.file {
        Some(Value::Obj(o)) => o.extend(counts),
        _ => made.file = Some(Value::Obj(counts.to_vec())),
    }
    if merged > 0 {
        made.warnings.push("warn.centroid.merged".into());
    }
}

/// The spectrum-type decision in the note of the file, so `prov.json` and `--report` carry it.
pub(crate) fn note_spectrum(made: &mut Made, text: String) {
    let item = ("spectrum_type".to_string(), Value::Str(text));
    match &mut made.file {
        Some(Value::Obj(o)) => o.push(item),
        _ => made.file = Some(Value::Obj(vec![item])),
    }
}

/// A partial read in the record of the run (prov.json, report.json, flowrun/1): its cause among the warnings, its counts as the note of the file.
pub(crate) fn note_losses(made: &mut Made, lost: &Losses) {
    if lost.partial() {
        made.warnings.push(lost.cause.into());
        made.file = Some(lost.note());
    }
}
