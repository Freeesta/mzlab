//! The files of mzFinder as `api.classify` names them (`guess_sample`, `file_kind`, `classify`), read from the
//! `.mzidx` of each file: label, treatment time and type guessed from the name, kind of the unit-resolution path and kind of the high-resolution path.

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;

/// What the name of a file says: label (the stem), time in minutes, type (`sample`, `blank`, `standard`, `control`).
#[derive(Debug, Clone, PartialEq)]
pub struct Guess {
    pub label: String,
    pub time: Option<f64>,
    pub kind: &'static str,
}

/// `Path(name).stem`: the last component without its last suffix.
pub fn stem(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    match base.rfind('.') {
        Some(i) if i > 0 && i + 1 < base.len() => base[..i].to_string(),
        _ => base.to_string(),
    }
}

/// `guess_sample`: label, time (min) and type from a file name ('blank', 't30', 'STD_12ppm', '120min', 'dark'), read by the one parser of the names.
pub fn guess(name: &str) -> Guess {
    let label = stem(name);
    let (role, minutes) = mzlab_core::campioni::ruolo_e_minuti(name);
    let kind = match role {
        "blank" => "blank",
        "dark" => "control",
        "std" | "calibration" => "standard",
        _ => "sample",
    };
    let time = if kind == "sample" { minutes } else { None };
    Guess { label, time, kind }
}

/// `file_kind` (unit-resolution path) and `hr_engine.classify` of an index: (`full`|`ms2`|`mrm`|`empty`, `hr`|`msn`|`other`, number of scans).
/// The `.mzidx` keeps the number of chromatograms but not their ids: a file with no spectra to speak of and more than one chromatogram (the TIC and
/// at least one transition) is `mrm`.
pub fn kinds<R: ReadAt>(ix: &Index<R>) -> (&'static str, &'static str, usize) {
    let n1 = ix.scans.iter().filter(|s| s.level == 1).count();
    let n2 = ix.scans.iter().filter(|s| s.level == 2).count();
    let chroms = ix
        .run
        .get("n_chromatograms")
        .and_then(|v| v.f64())
        .unwrap_or(0.0);
    let kind = if n1 >= 20 && n1 >= n2 {
        "full"
    } else if n2 >= 5 {
        "ms2"
    } else if chroms > 1.0 {
        "mrm"
    } else {
        "empty"
    };
    let max_level = ix.scans.iter().map(|s| s.level).max().unwrap_or(1);
    let hr = if max_level >= 3 {
        "msn"
    } else if ix.hr1() && n1 > 0 {
        "hr"
    } else {
        "other"
    };
    (kind, hr, ix.scans.len())
}

/// One item of `api.classify`: `{name, kind, hr_kind, type, time, label, scans}`, or the error item when the index cannot be read.
pub fn item_json<R: ReadAt>(o: &mut Out, name: &str, ix: std::result::Result<&Index<R>, String>) {
    let g = guess(name);
    o.raw("{\"name\":").str(name);
    match ix {
        Ok(ix) => {
            let (kind, hr, n) = kinds(ix);
            o.raw(",\"kind\":").str(kind).raw(",\"hr_kind\":").str(hr);
            o.raw(",\"type\":")
                .str(g.kind)
                .raw(",\"time\":")
                .opt(g.time);
            o.raw(",\"label\":")
                .str(&g.label)
                .raw(",\"scans\":")
                .int(n as i64);
        }
        Err(e) => {
            let e: String = e.chars().take(160).collect();
            o.raw(",\"kind\":\"error\",\"error\":").str(&e);
            o.raw(",\"type\":\"sample\",\"time\":null,\"label\":")
                .str(name);
        }
    }
    o.raw("}");
}
