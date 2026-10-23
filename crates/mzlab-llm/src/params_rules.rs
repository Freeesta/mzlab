//! `proposta-param/1`: the parameters of mzFlow proposed by deterministic rules (`R-*` of `docs/studi/assistente-dati/03-parametri.md` §5) from an
//! `ispezione/1` and the answers of the user. Every proposal carries its rule, signals, confidence, `range_ok` against the real `step/1` limits and the
//! place of its proof; a signal that is absent gives a `missing` question, never an invented value. Used by `mzlab-cli llm` and the `llm` module.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::step::{find, Kind, Param, Step};

use crate::params_rules_det as det;
use crate::params_rules_tab as tab;
use crate::tools::value;

/// One parameter: `(step, algorithm or "", name)`.
pub type Target = (&'static str, &'static str, &'static str);

/// Rule id → the parameters it sets as `(step, algorithm or "", name)`; the single source for the rules, the checks against the registry and the docs.
pub const TARGETS: &[(&str, &[Target])] = &[
    (
        "R-pol-1",
        &[
            ("features.estimate", "", "polarity"),
            ("features.detect", "", "polarity"),
            ("features.table", "", "polarity"),
        ],
    ),
    (
        "R-axis-1",
        &[
            ("features.estimate", "", "axis"),
            ("features.detect", "", "axis"),
        ],
    ),
    ("R-ppm-1", PPM),
    ("R-ppm-2", PPM),
    (
        "R-da-1",
        &[("tp.dig_lr", "", "tol_da"), ("spectra.ms2", "", "tol_da")],
    ),
    (
        "R-floor-1",
        &[
            ("features.detect", "", "floor"),
            ("features.gapfill", "", "floor"),
        ],
    ),
    (
        "R-height-1",
        &[
            ("features.detect", "", "min_height"),
            ("table.filter", "", "min_height"),
        ],
    ),
    ("R-snr-1", &[("features.detect", "centwave", "snr")]),
    (
        "R-width-1",
        &[("features.detect", "centwave", "peak_width_s")],
    ),
    (
        "R-pts-1",
        &[
            ("features.detect", "masstrace", "min_points"),
            ("features.gapfill", "", "min_points"),
        ],
    ),
    (
        "R-gap-1",
        &[
            ("features.detect", "masstrace", "max_gap"),
            ("features.gapfill", "", "max_gap"),
        ],
    ),
    ("R-rt-1", RT),
    ("R-rt-2", RT),
    ("R-warp-1", &[("features.align", "", "rt_warp")]),
    ("R-win-1", &[("tp.dig_lr", "", "rt_min")]),
    ("R-blank-1", &[("table.filter", "", "blank_ratio")]),
    (
        "R-qc-1",
        &[
            ("table.filter", "", "qc_cv_max"),
            ("table.filter", "", "qc_min_files"),
        ],
    ),
    ("R-det-1", &[("table.filter", "", "min_detected")]),
    ("R-ms2-1", &[("table.filter", "", "require_ms2")]),
    (
        "R-drift-1",
        &[("table.drift", "", "method"), ("table.drift", "", "span")],
    ),
    (
        "R-iso-1",
        &[("features.ms2_link", "", "default_half_window")],
    ),
    (
        "R-cent-1",
        &[
            ("spectra.centroid", "", "fwhm"),
            ("spectra.centroid", "", "threshold"),
        ],
    ),
    (
        "R-form-1",
        &[
            ("formula.ms1", "", "tol_ppm"),
            ("formula.ms2", "", "ppm_ion"),
        ],
    ),
    ("R-form-2", &[("formula.ms1", "", "elements")]),
];

const PPM: &[Target] = &[
    ("features.detect", "centwave", "ppm"),
    ("features.align", "", "ppm"),
    ("features.gapfill", "", "ppm"),
];
const RT: &[Target] = &[
    ("features.align", "", "rt_gap"),
    ("features.gapfill", "", "rt_tol"),
    ("features.table", "", "rt_tol"),
];

/// How sure a proposal is (`03-parametri.md` §1): `low` is shown as a question, not as a proposal.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Conf {
    High,
    Medium,
    Low,
}

impl Conf {
    fn name(self) -> &'static str {
        match self {
            Conf::High => "high",
            Conf::Medium => "medium",
            Conf::Low => "low",
        }
    }
}

/// The signals of the series, read from the `ispezione/1` of the reference file (the first readable file that is not a blank) and its `series`.
pub(crate) struct Sig<'a> {
    pub file: &'a Value,
    pub series: &'a Value,
    pub alias: String,
    pub hr: Option<bool>,
}

impl Sig<'_> {
    pub fn num(&self, path: &[&str]) -> Option<f64> {
        let mut v = self.file;
        for k in path {
            v = v.get(k)?;
        }
        v.f64().filter(|x| x.is_finite())
    }

    pub fn texts(&self, key: &str) -> Vec<&str> {
        self.file
            .get(key)
            .map_or(&[][..], Value::arr)
            .iter()
            .filter_map(Value::str)
            .collect()
    }

    pub fn role_count(&self, role: &str) -> f64 {
        self.series
            .get("roles")
            .and_then(|r| r.get(role))
            .and_then(Value::f64)
            .unwrap_or(0.0)
    }
}

/// What the rules collect: proposals, questions and warnings.
pub(crate) struct Acc<'a> {
    pub registry: &'a [Step],
    pub proposals: Vec<Value>,
    pub missing: Vec<Value>,
    pub warnings: Vec<String>,
    pub alias: String,
}

fn obj(items: Vec<(&str, Value)>) -> Value {
    Value::Obj(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn text(s: &str) -> Value {
    Value::Str(s.to_string())
}

fn param<'a>(step: &'a Step, alg: &str, name: &str) -> Option<&'a Param> {
    if alg.is_empty() {
        step.param(name)
    } else {
        step.algorithm_param(alg, name)
    }
}

impl Acc<'_> {
    /// One proposal for every parameter called `name` among the targets of `rule`.
    pub fn emit(
        &mut self,
        rule: &str,
        name: &str,
        v: Value,
        signals: &[String],
        conf: Conf,
        proof: &str,
    ) {
        let targets = TARGETS
            .iter()
            .find(|t| t.0 == rule)
            .map_or(&[][..], |t| t.1);
        for (step_id, alg, pname) in targets.iter().filter(|t| t.2 == name) {
            let Some(step) = self.registry.iter().find(|s| s.id == *step_id) else {
                continue;
            };
            let Some(p) = param(step, alg, pname) else {
                continue;
            };
            let (value, range_ok) = fit(p, v.clone());
            let mut items = vec![("step", Value::Str(format!("{}@{}", step.id, step.version)))];
            if !alg.is_empty() {
                items.push(("algorithm", text(alg)));
            }
            items.extend([
                ("name", text(pname)),
                ("value", value),
                ("rule", text(rule)),
                (
                    "signals",
                    Value::Arr(signals.iter().map(|s| text(s)).collect()),
                ),
                ("confidence", text(conf.name())),
                ("range_ok", Value::Bool(range_ok)),
                (
                    "proof",
                    Value::Str(format!("ispezione:{}:{}", self.alias, proof)),
                ),
            ]);
            self.proposals.push(obj(items));
        }
    }

    pub fn ask(&mut self, signal: &str, asks: &str) {
        if !self
            .missing
            .iter()
            .any(|m| m.get("signal").and_then(Value::str) == Some(signal))
        {
            self.missing
                .push(obj(vec![("signal", text(signal)), ("asks", text(asks))]));
        }
    }
}

/// `name=value` for the list of signals of a proposal.
pub(crate) fn sg(name: &str, x: f64) -> String {
    format!("{name}={}", crate::context::sig4(x))
}

/// The ispezione/1 of the sources → the proposals; `answers` is `{objective?, identify?, weak_signals?, elements?, groups?}`, `registry` is `flow::steps()`.
pub fn propose(inspection: &Value, answers: &Value, registry: &[Step]) -> Result<String> {
    if inspection.get("schema").and_then(Value::str) != Some("ispezione/1") {
        return Err(Error::new("err.llm.params.input"));
    }
    let files = inspection.get("files").map_or(&[][..], Value::arr);
    let ok = |f: &&Value| f.get("status").and_then(Value::str) == Some("ok");
    let reference = files
        .iter()
        .filter(ok)
        .find(|f| f.get("role").and_then(Value::str) != Some("blank"))
        .or_else(|| files.iter().find(ok));
    let null = Value::Null;
    let series = inspection.get("series").unwrap_or(&null);
    let mut acc = Acc {
        registry,
        proposals: vec![],
        missing: vec![],
        warnings: vec![],
        alias: String::new(),
    };
    if let Some(file) = reference {
        let class = file.get("class").and_then(Value::str);
        let sig = Sig {
            file,
            series,
            alias: file
                .get("alias")
                .and_then(Value::str)
                .unwrap_or("f1")
                .to_string(),
            hr: match class {
                Some("high") => Some(true),
                Some("unit") => Some(false),
                _ => None,
            },
        };
        acc.alias = sig.alias.clone();
        det::run(&sig, answers, &mut acc);
        tab::run(&sig, answers, &mut acc);
    } else {
        acc.ask("files", "Q-files");
    }
    let mut o = Out::default();
    o.raw("{\"schema\":\"proposta-param/1\",\"series\":\"s1\",\"proposals\":");
    value(&mut o, &Value::Arr(acc.proposals));
    o.raw(",\"missing\":");
    value(&mut o, &Value::Arr(acc.missing));
    o.raw(",\"warnings\":");
    value(
        &mut o,
        &Value::Arr(acc.warnings.iter().map(|w| text(w)).collect()),
    );
    o.raw("}");
    Ok(o.0)
}

/// `propose` for text inputs.
pub fn propose_text(inspection: &str, answers: &str, registry: &[Step]) -> Result<String> {
    let i = parse(inspection.as_bytes()).map_err(|_| Error::new("err.llm.params.input"))?;
    let a = if answers.trim().is_empty() {
        Value::Null
    } else {
        parse(answers.as_bytes()).map_err(|_| Error::new("err.llm.params.input"))?
    };
    propose(&i, &a, registry)
}

/// The descriptor of a target in `registry`, for the checks that every name of `TARGETS` exists.
pub fn target_param<'a>(
    registry: &'a [Step],
    step: &str,
    alg: &str,
    name: &str,
) -> Option<&'a Param> {
    param(find(registry, step)?, alg, name)
}

/// `value` brought inside the limits of `p`; false when it had to be cut (or is not one of the choices).
pub(crate) fn fit(p: &Param, v: Value) -> (Value, bool) {
    let cut = |x: f64, lo: Option<f64>| {
        let y = x
            .max(lo.or(p.min).unwrap_or(f64::NEG_INFINITY))
            .min(p.max.unwrap_or(f64::INFINITY));
        (y, y == x)
    };
    match (&p.kind, &v) {
        (Kind::Choice(list), _) => {
            let ok = list.contains(&v);
            (v, ok)
        }
        (Kind::Pair(lo), Value::Arr(a)) if a.len() == 2 => {
            let (x, k1) = cut(a[0].f64().unwrap_or(0.0), Some(*lo));
            let (y, k2) = cut(a[1].f64().unwrap_or(0.0), Some(x));
            (Value::Arr(vec![Value::Num(x), Value::Num(y)]), k1 && k2)
        }
        (Kind::Integer, Value::Num(x)) => {
            let (y, ok) = cut(x.round(), None);
            (Value::Num(y), ok && y == *x)
        }
        (_, Value::Num(x)) => {
            let (y, ok) = cut(*x, None);
            (Value::Num(y), ok)
        }
        _ => (v, true),
    }
}
