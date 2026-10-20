//! Self-description of a step of a workflow, `step/1` (`docs/studi/workflow/contract.schema.json`): plain data written next to the code that runs the
//! step, one descriptor per step and version, exported to JSON by `json::registry_json`. Nothing here runs a step or changes a result.
//! `core_steps` holds the steps owned by this crate; `mzlab_feat::steps::registry` adds the others and is the whole list.

mod core_steps;
pub mod json;

pub use core_steps::core_steps;

use crate::json::Value;

/// The typed data that travels between steps (`dataType` of the schema).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Raw,
    Mzml,
    Mzidx,
    Xic,
    Peaks,
    Rtmap,
    Table,
    Samples,
    Design,
    Spectra,
    Library,
    Formulas,
    Candidates,
    Annot,
    Tp,
    Structure,
    Transform,
    Comparison,
    Qc,
    Report,
}

impl DataType {
    pub const ALL: [DataType; 20] = [
        DataType::Raw,
        DataType::Mzml,
        DataType::Mzidx,
        DataType::Xic,
        DataType::Peaks,
        DataType::Rtmap,
        DataType::Table,
        DataType::Samples,
        DataType::Design,
        DataType::Spectra,
        DataType::Library,
        DataType::Formulas,
        DataType::Candidates,
        DataType::Annot,
        DataType::Tp,
        DataType::Structure,
        DataType::Transform,
        DataType::Comparison,
        DataType::Qc,
        DataType::Report,
    ];
    /// The type called `name` in a file (`raw`, `mzidx`, ...).
    pub fn from_name(name: &str) -> Option<DataType> {
        DataType::ALL.into_iter().find(|t| t.name() == name)
    }
    pub fn name(self) -> &'static str {
        match self {
            DataType::Raw => "raw",
            DataType::Mzml => "mzml",
            DataType::Mzidx => "mzidx",
            DataType::Xic => "xic",
            DataType::Peaks => "peaks",
            DataType::Rtmap => "rtmap",
            DataType::Table => "table",
            DataType::Samples => "samples",
            DataType::Design => "design",
            DataType::Spectra => "spectra",
            DataType::Library => "library",
            DataType::Formulas => "formulas",
            DataType::Candidates => "candidates",
            DataType::Annot => "annot",
            DataType::Tp => "tp",
            DataType::Structure => "structure",
            DataType::Transform => "transform",
            DataType::Comparison => "comparison",
            DataType::Qc => "qc",
            DataType::Report => "report",
        }
    }
}

/// Facets of the data a port reads or a preset fits: resolution (`hr`, `lr`), polarity (1, -1, 0 = all), MS levels. Empty = no constraint.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facets {
    pub resolution: Vec<&'static str>,
    pub polarity: Vec<i8>,
    pub ms_levels: Vec<u32>,
}

/// An input or output of a step.
#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub name: &'static str,
    pub ty: DataType,
    /// A list, one per file of the series.
    pub many: bool,
    pub optional: bool,
    pub title: &'static str,
    /// Other types an input takes besides `ty`.
    pub accepts: Vec<DataType>,
    pub requires: Facets,
}

impl Port {
    pub fn new(name: &'static str, ty: DataType, title: &'static str) -> Port {
        Port {
            name,
            ty,
            many: false,
            optional: false,
            title,
            accepts: Vec::new(),
            requires: Facets::default(),
        }
    }
    pub fn many(mut self) -> Port {
        self.many = true;
        self
    }
    pub fn accepts(mut self, t: &[DataType]) -> Port {
        self.accepts = t.to_vec();
        self
    }
    pub fn requires(mut self, f: Facets) -> Port {
        self.requires = f;
        self
    }
}

/// The JSON Schema type of a parameter.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Number,
    Integer,
    Bool,
    /// Free text (a list of element ranges, a path).
    Text,
    /// Two numbers, the second not below the first (a range), each at least the `f64`.
    Pair(f64),
    /// One of these values (numbers or strings).
    Choice(Vec<Value>),
}

/// One parameter of a step or an algorithm (a property of its `paramSchema`).
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: &'static str,
    pub title: &'static str,
    pub kind: Kind,
    /// None: no default, the step estimates the value or the rule is off.
    pub default: Option<Value>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub unit: Option<&'static str>,
    pub doc: Option<&'static str>,
    /// Hidden behind «More» in the form.
    pub advanced: bool,
    /// Absent value = measured by the step, which writes the value and its proof.
    pub estimate: bool,
    /// Does not change the result (memory budget): left out of the fingerprint.
    pub noresult: bool,
}

impl Param {
    fn new(name: &'static str, title: &'static str, kind: Kind) -> Param {
        Param {
            name,
            title,
            kind,
            default: None,
            min: None,
            max: None,
            unit: None,
            doc: None,
            advanced: false,
            estimate: false,
            noresult: false,
        }
    }
    pub fn number(name: &'static str, title: &'static str) -> Param {
        Param::new(name, title, Kind::Number)
    }
    pub fn integer(name: &'static str, title: &'static str) -> Param {
        Param::new(name, title, Kind::Integer)
    }
    pub fn boolean(name: &'static str, title: &'static str) -> Param {
        Param::new(name, title, Kind::Bool)
    }
    pub fn choice(name: &'static str, title: &'static str, values: &[&str]) -> Param {
        let v = values
            .iter()
            .map(|s| Value::Str((*s).to_string()))
            .collect();
        Param::new(name, title, Kind::Choice(v))
    }
    pub fn text(name: &'static str, title: &'static str) -> Param {
        Param::new(name, title, Kind::Text)
    }
    pub fn pair(name: &'static str, title: &'static str, min: f64) -> Param {
        Param::new(name, title, Kind::Pair(min))
    }
    pub fn default_pair(mut self, a: f64, b: f64) -> Param {
        self.default = Some(Value::Arr(vec![Value::Num(a), Value::Num(b)]));
        self
    }
    pub fn choice_num(name: &'static str, title: &'static str, values: &[f64]) -> Param {
        Param::new(
            name,
            title,
            Kind::Choice(values.iter().map(|v| Value::Num(*v)).collect()),
        )
    }
}

/// A paper an algorithm or step derives from; the DOI is resolved before it is written here.
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub doi: &'static str,
    /// First author + year + first word of the title, lower case.
    pub citation_key: &'static str,
    pub note: &'static str,
}

impl Param {
    pub fn default_num(mut self, v: f64) -> Param {
        self.default = Some(Value::Num(v));
        self
    }
    pub fn default_bool(mut self, v: bool) -> Param {
        self.default = Some(Value::Bool(v));
        self
    }
    pub fn default_str(mut self, v: &str) -> Param {
        self.default = Some(Value::Str(v.to_string()));
        self
    }
    pub fn min(mut self, v: f64) -> Param {
        self.min = Some(v);
        self
    }
    pub fn max(mut self, v: f64) -> Param {
        self.max = Some(v);
        self
    }
    pub fn unit(mut self, u: &'static str) -> Param {
        self.unit = Some(u);
        self
    }
    pub fn doc(mut self, d: &'static str) -> Param {
        self.doc = Some(d);
        self
    }
    pub fn advanced(mut self) -> Param {
        self.advanced = true;
        self
    }
    pub fn estimate(mut self) -> Param {
        self.estimate = true;
        self
    }
    pub fn noresult(mut self) -> Param {
        self.noresult = true;
        self
    }
    /// The default as a number (None when absent or not a number).
    pub fn default_f64(&self) -> Option<f64> {
        self.default.as_ref().and_then(Value::f64)
    }
}

/// When an algorithm fits the data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Applicability {
    pub resolution: Vec<&'static str>,
    pub spectrum_mode: Vec<&'static str>,
    /// Range of chromatographic peak widths (FWHM, seconds) where the method is known to work.
    pub peak_fwhm_s: Option<(f64, f64)>,
    pub ms_levels: Vec<u32>,
    /// Inputs the method cannot work without (`library`, `ms2`, `internal_standards`, `blank`, `reference_sample`).
    pub needs: Vec<&'static str>,
    pub note: &'static str,
}

/// Whether an algorithm can be chosen today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Available,
    Planned,
}

/// An interchangeable method of a step, with the same ports.
#[derive(Debug, Clone, PartialEq)]
pub struct Algorithm {
    pub id: &'static str,
    /// Raised whenever this algorithm can give a different output.
    pub version: u32,
    pub title: &'static str,
    pub doc: &'static str,
    pub references: Vec<Reference>,
    pub applicability: Applicability,
    /// Parameters of this algorithm only.
    pub params: Vec<Param>,
    pub status: Status,
}

/// A named set of parameter values for one algorithm.
#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub id: &'static str,
    pub title: &'static str,
    pub when: Facets,
    pub params: Vec<(&'static str, Value)>,
    /// The measure or constant that justifies the values.
    pub evidence: &'static str,
    pub algorithm: Option<&'static str>,
}

/// Where a step runs: the `.wasm` module with its size ceiling, and/or native.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunsIn {
    pub wasm: Option<&'static str>,
    pub wasm_ceiling_bytes: Option<u64>,
    pub native: bool,
}

/// The descriptor `step/1` of one step.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub id: &'static str,
    /// Raised whenever the same inputs and parameters can give a different output.
    pub version: u32,
    pub title: &'static str,
    pub doc: &'static str,
    pub cv: Option<&'static str>,
    pub package: &'static str,
    pub ins: Vec<Port>,
    pub outs: Vec<Port>,
    /// Parameters common to every algorithm.
    pub params: Vec<Param>,
    pub runs_in: RunsIn,
    pub deterministic: bool,
    pub errors: Vec<&'static str>,
    pub algorithms: Vec<Algorithm>,
    pub default_algorithm: Option<&'static str>,
    pub per_item: bool,
    /// A registered conversion: the plan check proposes it when a wire joins `from` to `to`.
    pub converts: Option<(DataType, DataType)>,
    pub cost: Option<f64>,
    pub memory_hint_bytes: Option<u64>,
    pub presets: Vec<Preset>,
    pub default_preset: Option<&'static str>,
    pub references: Vec<Reference>,
}

impl Step {
    /// A core step with the fields that every step has; the rest is filled by the caller.
    pub fn core(id: &'static str, title: &'static str, doc: &'static str) -> Step {
        Step {
            id,
            version: 1,
            title,
            doc,
            cv: None,
            package: "core",
            ins: Vec::new(),
            outs: Vec::new(),
            params: Vec::new(),
            runs_in: RunsIn::default(),
            deterministic: true,
            errors: Vec::new(),
            algorithms: Vec::new(),
            default_algorithm: None,
            per_item: false,
            converts: None,
            cost: None,
            memory_hint_bytes: None,
            presets: Vec::new(),
            default_preset: None,
            references: Vec::new(),
        }
    }

    /// The common parameter `name`, if the step has it.
    pub fn param(&self, name: &str) -> Option<&Param> {
        self.params.iter().find(|p| p.name == name)
    }

    /// The parameter `name` of algorithm `algorithm`.
    pub fn algorithm_param(&self, algorithm: &str, name: &str) -> Option<&Param> {
        let a = self.algorithms.iter().find(|a| a.id == algorithm)?;
        a.params.iter().find(|p| p.name == name)
    }
}

/// The descriptor with identifier `id` in `registry`.
pub fn find<'a>(registry: &'a [Step], id: &str) -> Option<&'a Step> {
    registry.iter().find(|s| s.id == id)
}
