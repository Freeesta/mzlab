//! What the `.mzidx` of a file says about how it was acquired, for the `columns` of the meta of a `.mzfeat` (the exports describe each sample with it).

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{Out, Value};

/// Facts of one input file, read from its index (no peak is read).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileFacts {
    /// Polarity of most MS1 scans: 1, -1, or 0 when unknown or tied.
    pub polarity: i8,
    /// Mean resolving power declared by the MS1 scans; 0 when none declares it.
    pub resolution: f64,
    /// `ms1` (no MS2), `dda`, `dia` (most MS2 scans isolate a window wider than 5 m/z) or `mrm` (chromatograms and no MS1).
    pub acquisition: String,
    /// Analyzer named by most MS1 scans (`an`), empty when none does.
    pub instrument: String,
}

/// An isolation window wider than this (m/z) is a DIA window.
const DIA_WIDTH: f64 = 5.0;

/// The facts of a file from its index.
pub fn file_facts<R: ReadAt>(index: &Index<R>) -> FileFacts {
    let (mut sign, mut res, mut nres, mut ms1, mut ms2) = (0i64, 0.0f64, 0u32, 0u32, 0u32);
    let mut an: Vec<(&str, u32)> = Vec::new();
    let mut wide = 0u32;
    for s in &index.scans {
        if s.level == 1 {
            ms1 += 1;
            sign += i64::from(s.polarity.signum());
            if let Some(x) = s.res {
                res += x;
                nres += 1;
            }
            if let Some(a) = s.an.as_deref() {
                match an.iter_mut().find(|(n, _)| *n == a) {
                    Some(e) => e.1 += 1,
                    None => an.push((a, 1)),
                }
            }
        } else if s.level >= 2 {
            ms2 += 1;
            wide += u32::from(s.iso.is_some_and(|(a, b)| b - a > DIA_WIDTH));
        }
    }
    let chroms = index.run.get("n_chromatograms").and_then(Value::f64);
    let acquisition = if ms1 == 0 && chroms.is_some_and(|c| c > 0.0) {
        "mrm"
    } else if ms2 == 0 {
        "ms1"
    } else if wide * 2 > ms2 {
        "dia"
    } else {
        "dda"
    };
    FileFacts {
        polarity: sign.signum() as i8,
        resolution: if nres > 0 { res / f64::from(nres) } else { 0.0 },
        acquisition: acquisition.to_string(),
        instrument: an
            .iter()
            .max_by_key(|e| e.1)
            .map_or("", |e| e.0)
            .to_string(),
    }
}

impl FileFacts {
    /// The object of the meta `columns`, in a fixed order.
    pub fn write(&self, j: &mut Out) {
        j.raw("{\"polarity\":")
            .int(i64::from(self.polarity))
            .raw(",\"resolution\":")
            .int(self.resolution.round() as i64)
            .raw(",\"acquisition\":")
            .str(&self.acquisition)
            .raw(",\"instrument\":")
            .str(&self.instrument)
            .raw("}");
    }

    /// Reads back what `write` wrote (missing fields are the default).
    pub fn from_json(v: &Value) -> FileFacts {
        let text = |k: &str| v.get(k).and_then(Value::str).unwrap_or("").to_string();
        FileFacts {
            polarity: v.get("polarity").and_then(Value::f64).unwrap_or(0.0) as i8,
            resolution: v.get("resolution").and_then(Value::f64).unwrap_or(0.0),
            acquisition: text("acquisition"),
            instrument: text("instrument"),
        }
    }
}
