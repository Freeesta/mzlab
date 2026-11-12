//! Errors carry an i18n key and parameters, never a text: the page translates them (as the Python engine does).

use std::fmt;
use std::sync::{Mutex, PoisonError};

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub key: &'static str,
    pub params: Vec<(&'static str, String)>,
}

impl Error {
    pub fn new(key: &'static str) -> Self {
        Error {
            key,
            params: Vec::new(),
        }
    }

    pub fn with(self, name: &'static str, value: impl ToString) -> Self {
        self.param(name, value.to_string())
    }

    // one body for every value type: the generic `with` adds only its `to_string` to the size-capped `.wasm`
    fn param(mut self, name: &'static str, value: String) -> Self {
        self.params.push((name, value));
        self
    }

    /// The error as the JSON text every bridge sends (`{"error_key":..,"params":{..}}`); strings go through `json::Out::str`, so control characters become valid `\u0001`.
    // `#[inline]` so each bridge compiles it at its own opt-level ("z" for the size-capped `.wasm`), not the core's 3.
    #[inline]
    pub fn to_json(&self) -> String {
        let mut o = crate::json::Out::default();
        o.raw("{\"error_key\":").str(self.key).raw(",\"params\":{");
        for (i, (k, v)) in self.params.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
        }
        o.raw("}}");
        o.0
    }

    /// A large allocation that could not be reserved (the WASM memory is limited): never a panic.
    pub fn memory_budget() -> Self {
        Error::new("error.memoryBudget")
    }

    /// A part of a file (`what`, engine words) above `bytes`, a limit of the program that no memory budget moves: the index pool retries only `error.memoryBudget`.
    pub fn fixed_limit(what: &str, bytes: u64) -> Self {
        Error::new("error.fixedLimit")
            .with("what", what)
            .with("limit_mb", bytes.div_ceil(1 << 20))
    }

    /// A helper thread that stopped without its answer (a panic): `detail` names the work, never the memory.
    pub fn work_stopped(detail: &str) -> Self {
        Error::new("error.workStopped").with("detail", detail)
    }
}

/// `s` as a `&'static str`, leaked once per distinct text: the error keys, parameter names and descriptor texts read at run time (job reports,
/// step registries) come from a small vocabulary, so the memory kept is bounded by it, not by how many reports or registry reads arrive.
/// A plain list: the vocabulary is small, and a set map would add ~3 KB to every .wasm that links the engine.
pub fn intern(s: &str) -> &'static str {
    static SEEN: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut seen = SEEN.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(&k) = seen.iter().find(|&&k| k == s) {
        return k;
    }
    let k: &'static str = String::leak(s.to_owned());
    seen.push(k);
    k
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.key)?;
        for (k, v) in &self.params {
            write!(f, " {k}={v}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interned_texts_are_kept_once() {
        let a = intern(&String::from("err.test.interned"));
        let b = intern(&String::from("err.test.interned"));
        assert_eq!(a, "err.test.interned");
        assert!(
            std::ptr::eq(a, b),
            "the second call must not leak a new copy"
        );
    }

    #[test]
    fn to_json_is_valid_json_for_control_characters() {
        let e = Error::new("error.x").with("name", "a\u{1}b\"c\\d\n\u{200b}é");
        let v = crate::json::parse(e.to_json().as_bytes()).expect("valid JSON");
        assert_eq!(v.get("error_key").and_then(|k| k.str()), Some("error.x"));
        assert_eq!(
            v.get("params")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.str()),
            Some("a\u{1}b\"c\\d\n\u{200b}é")
        );
    }
}
