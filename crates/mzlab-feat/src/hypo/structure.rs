//! What the page knows of the structure of the parent, as the JSON of `grafo/1` (`chem::grafo::analyse_json`, which runs in its own `.wasm`) with an optional
//! `"profile"`: `{"profile":"fotocatalisi","formula":"C13H24N4O3S","groups":[..],"fragments":[..]}`. Other keys are ignored; empty text = nothing known.

use mzlab_core::elements;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Value};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Structure {
    /// Profile of the rules (`None` = the default of the file).
    pub profile: Option<String>,
    /// Formula of the structure: it must be the formula of the parent.
    pub formula: Option<String>,
    /// Functional groups; `None` = the structure is not known.
    pub groups: Option<Vec<String>>,
    pub fragments: Vec<String>,
}

fn strings(v: Option<&Value>) -> Option<Vec<String>> {
    v.map(|a| a.arr().iter().filter_map(|x| x.str().map(String::from)).collect())
}

impl Structure {
    pub fn parse(text: &str) -> Result<Structure> {
        if text.trim().is_empty() {
            return Ok(Structure::default());
        }
        let v = json::parse(text.as_bytes()).map_err(|_| Error::new("error.hypo.structure"))?;
        let s = |k: &str| v.get(k).and_then(Value::str).filter(|x| !x.is_empty()).map(String::from);
        Ok(Structure { profile: s("profile"), formula: s("formula"), groups: strings(v.get("groups")), fragments: strings(v.get("fragments")).unwrap_or_default() })
    }

    /// The structure has to be the parent: another formula is refused with both.
    pub fn check_parent(&self, parent: &str) -> Result<()> {
        let Some(f) = &self.formula else { return Ok(()) };
        let norm = |t: &str| elements::parse_formula(t).map(|x| elements::fmt(&x));
        let (a, b) = (norm(f)?, norm(parent)?);
        if a != b {
            return Err(Error::new("error.hypo.structureMismatch").with("structure", a).with("parent", b));
        }
        Ok(())
    }
}
