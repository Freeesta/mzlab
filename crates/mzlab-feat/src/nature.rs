//! What the files of a series say of themselves: the polarity of their MS1 scans and whether they are unit resolution. `tp.generate`, `tp.link@2` and
//! `tp.dig_lr` use it for what the plan leaves unwritten (`polarity`, `nominal`), the same rule in `flow run` (`mzlab-cli`) and in the browser (the proofs .wasm).

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
mod tests {
    use super::*;

    #[test]
    fn negative_hr_positive_lr_and_mixed_series() {
        let neg_hr = of_facts([(-1, true), (-1, true)]);
        assert_eq!(
            neg_hr,
            Nature {
                polarity: Some(-1),
                unit: false
            }
        );
        let pos_lr = of_facts([(1, false), (1, false), (1, false)]);
        assert_eq!(
            pos_lr,
            Nature {
                polarity: Some(1),
                unit: true
            }
        );
        // an even mix of polarities decides none; one high-resolution file makes the series not unit
        let mixed = of_facts([(1, false), (-1, true)]);
        assert_eq!(
            mixed,
            Nature {
                polarity: None,
                unit: false
            }
        );
        // the majority decides, a file of unknown polarity counts for none
        assert_eq!(
            of_facts([(-1, false), (-1, false), (1, false), (0, false)]).polarity,
            Some(-1)
        );
        assert_eq!(
            of_facts([]),
            Nature {
                polarity: None,
                unit: false
            }
        );
    }

    #[test]
    fn the_plan_wins_and_the_detection_closes_the_polarity() {
        let n = Nature {
            polarity: Some(-1),
            unit: true,
        };
        let names = |v: Vec<(String, Value, Value)>| v.into_iter().map(|e| e.0).collect::<Vec<_>>();
        assert_eq!(names(estimates(&|_| false, &n)), ["polarity", "nominal"]);
        assert_eq!(names(estimates(&|k| k == "polarity", &n)), ["nominal"]);
        assert!(estimates(&|_| true, &n).is_empty());
        assert_eq!(open_polarity(None, &n), Some(-1));
        assert_eq!(open_polarity(Some(0), &n), Some(-1));
        assert_eq!(open_polarity(Some(1), &n), None);
        assert_eq!(
            estimates_json(&estimates(&|_| false, &n)),
            r#"[{"name":"polarity","value":-1,"proof":"scan polarity of the files"},{"name":"nominal","value":true,"proof":"unit resolution of the files"}]"#
        );
    }
}
