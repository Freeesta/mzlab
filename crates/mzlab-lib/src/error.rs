//! Errors carry an i18n key and parameters, never a text: the page translates them.

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

    /// The file is damaged (bad checksum, impossible offsets or counts): the page offers to index it again.
    pub fn corrupt(what: &'static str) -> Self {
        Error::new("lib.err.corrupt").with("what", what)
    }

    /// The file was written by a newer version of the program.
    pub fn version(found: u16, need: u16) -> Self {
        Error::new("lib.err.version")
            .with("found", found)
            .with("need", need)
    }

    /// A large allocation could not be reserved (the WASM memory is limited): never a panic.
    pub fn memory_budget() -> Self {
        Error::new("error.memoryBudget")
    }

    /// A limit of the format (spectra, peaks, sizes) would be exceeded.
    pub fn too_large(what: &'static str) -> Self {
        Error::new("lib.err.tooLarge").with("what", what)
    }

    /// JSON `{"error_key":…,"params":{…}}` as the other WASM bridges give it.
    pub fn to_json(&self) -> String {
        let mut p = serde_json::Map::new();
        for (k, v) in &self.params {
            p.insert((*k).to_string(), serde_json::Value::String(v.clone()));
        }
        serde_json::json!({ "error_key": self.key, "params": p }).to_string()
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
