//! What the `.mzidx` of a file says about how it was acquired, for the `columns` of the meta of a `.mzfeat` (the exports describe each sample with it).

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{Out, Value};

/// Facts of one input file, read from its index (no peak is read).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileFacts {
    /// Polarity of most MS1 scans: 1, -1, or 0 when unknown or tied.
    pub polarity: i8,
    /// Median resolving power declared by the MS1 scans; 0 when none declares it.
    pub resolution: f64,
    /// `ms1` (no MS2), `dda`, `dia` (few distinct isolation centres for many MS2 scans) or `mrm` (chromatograms and no MS1).
    pub acquisition: String,
    /// Analyzer named by most MS1 scans (`an`), empty when none does.
    pub instrument: String,
}

/// Fewer distinct isolation centres than this share of the MS2 scans means a fixed list of windows (DIA).
const DIA_SHARE: f64 = 0.1;

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v.get(v.len() / 2).copied().unwrap_or(0.0)
}

/// The facts of a file from its index.
pub fn file_facts<R: ReadAt>(index: &Index<R>) -> FileFacts {
    let ms1 = || index.scans.iter().filter(|s| s.level == 1);
    let sign: i64 = ms1().map(|s| i64::from(s.polarity.signum())).sum();
    let mut names: Vec<&str> = ms1().filter_map(|s| s.an.as_deref()).collect();
    names.sort_unstable();
    let top = |names: &[&str]| {
        names
            .chunk_by(|a, b| a == b)
            .max_by_key(|g| g.len())
            .map_or("", |g| g[0])
            .to_string()
    };
    let ms2: Vec<f64> = index
        .scans
        .iter()
        .filter(|s| s.level >= 2)
        .map(|s| s.prec(0.0))
        .collect();
    let mut centres: Vec<i64> = ms2.iter().map(|c| (c * 10.0).round() as i64).collect();
    centres.sort_unstable();
    centres.dedup();
    let chroms = index
        .run
        .get("n_chromatograms")
        .and_then(Value::f64)
        .unwrap_or(0.0);
    let acquisition = if ms1().next().is_none() && chroms > 0.0 {
        "mrm"
    } else if ms2.is_empty() {
        "ms1"
    } else if (centres.len() as f64) < DIA_SHARE * ms2.len() as f64 {
        "dia"
    } else {
        "dda"
    };
    FileFacts {
        polarity: sign.signum() as i8,
        resolution: median(ms1().filter_map(|s| s.res).collect()),
        acquisition: acquisition.to_string(),
        instrument: top(&names),
    }
}

impl FileFacts {
    /// The object of the meta `columns`, in a fixed order.
    pub fn write(&self, j: &mut Out) {
        j.raw("{\"polarity\":")
            .int(i64::from(self.polarity))
            .raw(",\"resolution\":")
            .num(self.resolution)
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
