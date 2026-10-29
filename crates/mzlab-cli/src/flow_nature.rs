//! What the files of a series say of themselves: the polarity of their MS1 scans and whether they are unit resolution. `tp.generate` and `tp.link@2`
//! use it for what the plan leaves unwritten (`polarity`, `nominal`), so a negative or a unit-resolution series needs no setting by hand.

use std::path::PathBuf;

use mzlab_core::flow::Job;
use mzlab_core::json::Value;
use mzlab_feat::facts::file_facts;

use crate::flow_base::open;

/// `Some(±1)` when the MS1 scans of the files lean one way, None for no files or an even mix; `unit` is true when every file is unit resolution.
pub struct Nature {
    pub polarity: Option<i8>,
    pub unit: bool,
}

pub fn of_files(paths: &[PathBuf]) -> Nature {
    let (mut sum, mut unit) = (0i64, !paths.is_empty());
    for p in paths {
        if let Ok(ix) = open(p) {
            sum += i64::from(file_facts(&ix).polarity);
            unit &= !ix.hr1();
        }
    }
    Nature {
        polarity: (sum != 0).then(|| sum.signum() as i8),
        unit,
    }
}

/// Whether the instance of the plan writes the parameter (a value the descriptor fills in is not a choice of the plan).
pub fn written(job: &Job, name: &str) -> bool {
    job.instance.params.iter().any(|(k, _)| k == name)
}

/// The estimate record of a parameter the plan left unwritten and the files decided (name, value, what it was read from).
pub fn estimate(name: &str, value: Value, from: &str) -> (String, Value, Value) {
    (name.to_string(), value, Value::Str(from.into()))
}

/// The estimates of `tp.generate`: the polarity and the unit resolution of the files, when the plan does not write them.
pub fn estimates(job: &Job, nature: &Nature) -> Vec<(String, Value, Value)> {
    let mut v = Vec::new();
    if let Some(p) = nature.polarity.filter(|_| !written(job, "polarity")) {
        v.push(estimate(
            "polarity",
            Value::Num(f64::from(p)),
            "scan polarity of the files",
        ));
    }
    if nature.unit && !written(job, "nominal") {
        v.push(estimate(
            "nominal",
            Value::Bool(true),
            "unit resolution of the files",
        ));
    }
    v
}
