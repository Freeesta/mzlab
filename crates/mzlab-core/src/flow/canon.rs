//! Canonical form of an instance (rule 1 of the study): the algorithm default, then the preset, then the differences written in the file, as JSON with sorted
//! keys and numbers in the shortest form that reads back equal. The fingerprint `params_sha256` leaves out the parameters that do not change the result.

use super::Instance;
use crate::budget::sha256_hex;
use crate::error::{Error, Result};
use crate::json::{Out, Value};
use crate::step::{Algorithm, Param, Preset, Step};

/// The parameters of an instance with every value present, and what ran: the algorithm (`id`, `version`) and the preset.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub algorithm: Option<(String, u32)>,
    pub preset: Option<String>,
    /// Sorted by key.
    pub params: Vec<(String, Value)>,
    /// Names of the parameters that do not change the result (memory budget): shown, but not part of the fingerprint.
    pub noresult: Vec<String>,
}

fn write(o: &mut Out, v: &Value) {
    match v {
        Value::Null => {
            o.raw("null");
        }
        Value::Bool(b) => {
            o.raw(if *b { "true" } else { "false" });
        }
        // `{}` is the shortest text that reads back equal; -0 is written as 0
        Value::Num(n) => {
            o.num(if *n == 0.0 { 0.0 } else { *n });
        }
        Value::Str(s) => {
            o.str(s);
        }
        Value::Arr(a) => {
            o.raw("[");
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    o.raw(",");
                }
                write(o, x);
            }
            o.raw("]");
        }
        Value::Obj(kv) => object(o, kv.iter().map(|(k, x)| (k.as_str(), x)).collect()),
    }
}

fn object(o: &mut Out, mut kv: Vec<(&str, &Value)>) {
    crate::sortc::slice_by(&mut kv, |a, b| a.0.cmp(b.0));
    o.raw("{");
    for (i, (k, x)) in kv.into_iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(k).raw(":");
        write(o, x);
    }
    o.raw("}");
}

/// The canonical JSON text of one value.
pub(super) fn value_text(v: &Value) -> String {
    let mut o = Out::default();
    write(&mut o, v);
    o.0
}

impl Resolved {
    /// The canonical JSON text of every parameter.
    pub fn text(&self) -> String {
        self.text_without(&[])
    }
    fn text_without(&self, skip: &[String]) -> String {
        let mut o = Out::default();
        object(
            &mut o,
            self.params
                .iter()
                .filter(|(k, _)| !skip.contains(k))
                .map(|(k, v)| (k.as_str(), v))
                .collect(),
        );
        o.0
    }
    /// SHA-256 of the canonical text without the `noresult` parameters: the same result, the same fingerprint.
    pub fn params_sha256(&self) -> String {
        sha256_hex(self.text_without(&self.noresult).as_bytes())
    }
}

/// The key of the cache: (SHA-256 of every input, `id@version`, `id@version` of the algorithm, `params_sha256`). The same key gives the same result.
pub fn cache_key(
    inputs: &[String],
    step: &str,
    algorithm: Option<&str>,
    params_sha256: &str,
) -> String {
    let text = format!(
        "{}\n{step}\n{}\n{params_sha256}",
        inputs.join(","),
        algorithm.unwrap_or("")
    );
    sha256_hex(text.as_bytes())
}

/// The preset and the algorithm an instance means: the preset written or `default_preset`; the algorithm written, else the one of the preset, else
/// `default_algorithm`. An unknown name is an error.
pub(super) fn effective<'a>(
    inst: &Instance,
    step: &'a Step,
) -> Result<(Option<&'a Algorithm>, Option<&'a Preset>)> {
    let preset_id = inst.preset.as_deref().or(step.default_preset);
    let preset = match preset_id {
        Some(id) => Some(
            step.presets
                .iter()
                .find(|p| p.id == id)
                .ok_or_else(|| Error::new("err.flow.preset.unknown").with("preset", id))?,
        ),
        None => None,
    };
    let algorithm_id = inst
        .algorithm
        .as_deref()
        .or(preset.and_then(|p| p.algorithm))
        .or(step.default_algorithm);
    let algorithm = match algorithm_id {
        Some(id) => Some(
            step.algorithms
                .iter()
                .find(|a| a.id == id)
                .ok_or_else(|| Error::new("err.flow.algorithm.unknown").with("algorithm", id))?,
        ),
        None => None,
    };
    // a preset taken only because it is the default does not follow an algorithm it was not made for
    let preset = preset.filter(|p| {
        inst.preset.is_some() || p.algorithm.is_none() || p.algorithm == algorithm.map(|a| a.id)
    });
    Ok((algorithm, preset))
}

fn set(params: &mut Vec<(String, Value)>, key: &str, v: Value) {
    match params.iter_mut().find(|(k, _)| k == key) {
        Some(slot) => slot.1 = v,
        None => params.push((key.to_string(), v)),
    }
}

/// The parameters an instance can be given: those of the step and those of its algorithm.
pub(super) fn declared<'a>(
    step: &'a Step,
    algorithm: Option<&'a Algorithm>,
) -> impl Iterator<Item = &'a Param> {
    step.params
        .iter()
        .chain(algorithm.into_iter().flat_map(|a| a.params.iter()))
}

/// Resolves an instance: defaults, then the preset, then the file. A parameter with no default that nobody sets (an estimate, an optional rule) is absent.
pub fn canonical(inst: &Instance, step: &Step) -> Result<Resolved> {
    let (algorithm, preset) = effective(inst, step)?;
    let mut params: Vec<(String, Value)> = Vec::new();
    for p in declared(step, algorithm) {
        if let Some(d) = &p.default {
            set(&mut params, p.name, d.clone());
        }
    }
    if let Some(p) = preset {
        for (k, v) in &p.params {
            set(&mut params, k, v.clone());
        }
    }
    for (k, v) in &inst.params {
        set(&mut params, k, v.clone());
    }
    crate::sortc::slice_by(&mut params, |a, b| a.0.cmp(&b.0));
    Ok(Resolved {
        algorithm: algorithm.map(|a| (a.id.to_string(), a.version)),
        preset: preset.map(|p| p.id.to_string()),
        noresult: declared(step, algorithm)
            .filter(|p| p.noresult)
            .map(|p| p.name.to_string())
            .collect(),
        params,
    })
}
