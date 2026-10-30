//! Errors carry an i18n key and parameters, never a text: the page translates them (as the Python engine does).

use std::fmt;

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

    pub fn with(mut self, name: &'static str, value: impl ToString) -> Self {
        self.params.push((name, value.to_string()));
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
