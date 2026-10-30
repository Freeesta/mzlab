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

    pub fn with(mut self, name: &'static str, value: impl ToString) -> Self {
        self.params.push((name, value.to_string()));
        self
    }

    /// A large allocation that could not be reserved (the WASM memory is limited): never a panic.
    pub fn memory_budget() -> Self {
        Error::new("error.memoryBudget")
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
    let k: &'static str = Box::leak(s.to_string().into_boxed_str());
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
    #[test]
    fn interned_texts_are_kept_once() {
        let a = super::intern(&String::from("err.test.interned"));
        let b = super::intern(&String::from("err.test.interned"));
        assert_eq!(a, "err.test.interned");
        assert!(
            std::ptr::eq(a, b),
            "the second call must not leak a new copy"
        );
    }
}
