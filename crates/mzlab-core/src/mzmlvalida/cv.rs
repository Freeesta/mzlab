//! The PSI-MS vocabulary the validator checks against: `data/psims-cv.json`, generated from `psi-ms.obo` by `tools/psims_tabella.py` (CC BY 4.0,
//! declared in `LICENZE-TERZI.md`). Loaded once on first use.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::json::{self, Value};

/// The groups of terms the validator asks "is this term a ...?" about (the roots are in the generator).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Class {
    SpectrumType = 1,
    SpectrumRepr = 2,
    ArrayType = 4,
    BinaryType = 8,
    Compression = 16,
    ChromType = 32,
}

const CLASSES: [(&str, Class); 6] = [
    ("spectrum_type", Class::SpectrumType),
    ("spectrum_repr", Class::SpectrumRepr),
    ("array_type", Class::ArrayType),
    ("binary_type", Class::BinaryType),
    ("compression", Class::Compression),
    ("chrom_type", Class::ChromType),
];

pub(super) struct Cv {
    /// accession -> (name, obsolete)
    terms: HashMap<String, (String, bool)>,
    /// accession -> the units its `has_units` allows
    units: HashMap<String, Vec<String>>,
    unit_names: HashMap<String, String>,
    class: HashMap<String, u8>,
}

/// The table of the repository. A broken table is a bug of the build, not of the file being checked.
pub(super) fn get() -> &'static Cv {
    static CV: OnceLock<Cv> = OnceLock::new();
    CV.get_or_init(|| {
        let v = json::parse(include_bytes!("../../data/psims-cv.json")).unwrap_or(Value::Null);
        Cv::from_json(&v)
    })
}

fn strings(v: &Value) -> Vec<String> {
    v.arr()
        .iter()
        .filter_map(|s| s.str().map(String::from))
        .collect()
}

impl Cv {
    fn from_json(v: &Value) -> Cv {
        let mut cv = Cv {
            terms: HashMap::new(),
            units: HashMap::new(),
            unit_names: HashMap::new(),
            class: HashMap::new(),
        };
        for t in v.get("terms").map_or(&[][..], Value::arr) {
            if let (Some(a), Some(n)) = (t.at(0).and_then(Value::str), t.at(1).and_then(Value::str))
            {
                let obsolete = t.at(2).and_then(Value::f64) == Some(1.0);
                cv.terms.insert(a.into(), (n.into(), obsolete));
            }
        }
        if let Some(Value::Obj(o)) = v.get("units") {
            cv.units = o.iter().map(|(k, u)| (k.clone(), strings(u))).collect();
        }
        if let Some(Value::Obj(o)) = v.get("unit_names") {
            cv.unit_names = o
                .iter()
                .filter_map(|(k, n)| Some((k.clone(), n.str()?.to_string())))
                .collect();
        }
        for (key, class) in CLASSES {
            for a in strings(
                v.get(key)
                    .or_else(|| v.get("classes").and_then(|c| c.get(key)))
                    .unwrap_or(&Value::Null),
            ) {
                *cv.class.entry(a).or_insert(0) |= class as u8;
            }
        }
        cv
    }

    /// (name, obsolete) of an `MS:` term.
    pub fn term(&self, acc: &str) -> Option<(&str, bool)> {
        self.terms.get(acc).map(|(n, o)| (n.as_str(), *o))
    }

    /// The name of a unit: an `MS:` term, or a `UO:` unit that some term of the table allows.
    pub fn unit_name(&self, acc: &str) -> Option<&str> {
        self.terms
            .get(acc)
            .map(|t| t.0.as_str())
            .or_else(|| self.unit_names.get(acc).map(String::as_str))
    }

    /// The units the term allows (empty: the table says nothing).
    pub fn units(&self, acc: &str) -> &[String] {
        self.units.get(acc).map_or(&[], Vec::as_slice)
    }

    pub fn is(&self, acc: &str, class: Class) -> bool {
        self.class.get(acc).is_some_and(|b| b & class as u8 != 0)
    }

    pub fn len(&self) -> usize {
        self.terms.len()
    }
}
