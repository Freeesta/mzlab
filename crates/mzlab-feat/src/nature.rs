//! What the files of a series say of themselves: the polarity of their MS1 scans and whether they are unit resolution. `tp.generate`, `tp.link@2` and
//! `tp.dig_lr` use it for what the plan leaves unwritten (`polarity`, `nominal`), the same rule in `flow run` (`mzlab-cli`) and in the browser (the proofs .wasm);
//! `features.align` refuses a series that mixes them (`one_kind_of`).

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{write_value, Out, Value};

use crate::facts::file_facts;

/// `Some(±1)` when the MS1 scans of the files lean one way, None for no files or an even mix; `unit` is true when every file is unit resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nature {
    pub polarity: Option<i8>,
    pub unit: bool,
}

/// The nature of a series from the (polarity of the MS1 scans, high resolution) of each file.
pub fn of_facts(files: impl IntoIterator<Item = (i8, bool)>) -> Nature {
    let (mut sum, mut unit, mut any) = (0i64, true, false);
    for (polarity, hr) in files {
        sum += i64::from(polarity);
        unit &= !hr;
        any = true;
    }
    Nature {
        polarity: (sum != 0).then(|| sum.signum() as i8),
        unit: unit && any,
    }
}

/// The nature of the series of `indexes`.
pub fn of_indexes<R: ReadAt>(indexes: &[Index<R>]) -> Nature {
    of_facts(indexes.iter().map(|ix| (file_facts(ix).polarity, ix.hr1())))
}

/// One table holds one polarity and one resolution: `err.flow.align.polarity` names the positive and the negative files, `err.flow.align.resolution`
/// the unit and the high-resolution ones (a file of tied or unknown polarity takes no side). `files` = (name, polarity of the MS1 scans, high resolution).
pub fn one_kind(files: &[(String, i8, bool)]) -> Result<()> {
    let names = |keep: &dyn Fn(i8, bool) -> bool| {
        let v: Vec<&str> = files
            .iter()
            .filter(|f| keep(f.1, f.2))
            .map(|f| f.0.as_str())
            .collect();
        v.join(", ")
    };
    let (pos, neg) = (names(&|p, _| p > 0), names(&|p, _| p < 0));
    if !pos.is_empty() && !neg.is_empty() {
        return Err(Error::new("err.flow.align.polarity")
            .with("positive", pos)
            .with("negative", neg));
    }
    let (unit, high) = (names(&|_, hr| !hr), names(&|_, hr| hr));
    if !unit.is_empty() && !high.is_empty() {
        return Err(Error::new("err.flow.align.resolution")
            .with("unit", unit)
            .with("high", high));
    }
    Ok(())
}

/// `one_kind` of the series of `indexes`, each named after the file it was made from; `features.align` of `flow run` and of the browser call it.
pub fn one_kind_of<R: ReadAt>(indexes: &[Index<R>]) -> Result<()> {
    let files: Vec<(String, i8, bool)> = indexes
        .iter()
        .map(|ix| {
            let name = ix.source.get("name").and_then(Value::str).unwrap_or("");
            (name.to_string(), file_facts(ix).polarity, ix.hr1())
        })
        .collect();
    one_kind(&files)
}

/// The polarity of the link or the Dig when the detection took both (or no) polarities: the files', so a negative series is not read as positive.
pub fn open_polarity(detect: Option<i8>, nature: &Nature) -> Option<i8> {
    detect
        .is_none_or(|p| p == 0)
        .then_some(nature.polarity)
        .flatten()
}

/// The estimate record of a parameter the plan left unwritten and the files decided (name, value, what it was read from).
pub fn estimate(name: &str, value: Value, from: &str) -> (String, Value, Value) {
    (name.to_string(), value, Value::Str(from.into()))
}

/// The estimate of the polarity the files decide.
pub fn polarity_estimate(p: i8) -> (String, Value, Value) {
    estimate(
        "polarity",
        Value::Num(f64::from(p)),
        "scan polarity of the files",
    )
}

/// The estimates of `tp.generate`: the polarity and the unit resolution of the files, for the parameters the plan does not write (`written`).
pub fn estimates(written: &dyn Fn(&str) -> bool, nature: &Nature) -> Vec<(String, Value, Value)> {
    let mut v = Vec::new();
    if let Some(p) = nature.polarity.filter(|_| !written("polarity")) {
        v.push(polarity_estimate(p));
    }
    if nature.unit && !written("nominal") {
        v.push(estimate(
            "nominal",
            Value::Bool(true),
            "unit resolution of the files",
        ));
    }
    v
}

/// The estimates as the browser runner reads them: `[{"name","value","proof"}]`.
pub fn estimates_json(list: &[(String, Value, Value)]) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (k, (name, value, proof)) in list.iter().enumerate() {
        if k > 0 {
            o.raw(",");
        }
        o.raw("{\"name\":").str(name).raw(",\"value\":");
        write_value(&mut o, value);
        o.raw(",\"proof\":");
        write_value(&mut o, proof);
        o.raw("}");
    }
    o.raw("]");
    o.0
}

#[cfg(test)]
#[path = "nature_tests.rs"]
mod tests;
