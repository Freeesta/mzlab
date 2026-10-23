//! The subagents of the data assistant as data: `data/assist/assist-ruoli-1.json` says which prompt, model, effort and output limit each role has, and the
//! prompt files (fetched by the page, never part of the program) are exact copies of the blocks of `docs/studi/assistente-dati/prompt/`. Nothing about a model is written in the code. Used by the agent.

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};

/// One subagent: what it is asked, by which model, and what it may answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Role {
    pub role: String,
    pub prompt_file: String,
    pub model: String,
    pub effort: String,
    pub max_tokens: u32,
    pub output_schema: String,
    pub input_sources: Vec<String>,
}

fn bad() -> Error {
    Error::new("err.llm.assist.roles")
}

fn text(v: &Value, key: &str) -> Result<String> {
    v.get(key)
        .and_then(Value::str)
        .map(String::from)
        .ok_or_else(bad)
}

/// The prompt files the caller has read (the page fetches them from `static/llm/assist/`; the program carries none).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Prompts(Vec<(String, String)>);

/// The prompts a caller hands over as `(file name, text)`.
pub fn with_prompts(files: &[(&str, &str)]) -> Prompts {
    Prompts(
        files
            .iter()
            .map(|(n, t)| (n.to_string(), t.to_string()))
            .collect(),
    )
}

impl Prompts {
    /// The text of a prompt file by name.
    pub fn prompt(&self, file: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(n, _)| n == file)
            .map(|(_, t)| t.as_str())
    }

    /// SHA-256 of a prompt as it is sent (recorded in the log `llmlog/1`).
    pub fn digest(&self, file: &str) -> Option<String> {
        self.prompt(file)
            .map(|t| sha256_hex(t.trim_end().as_bytes()))
    }

    /// Every role must have its prompt file.
    pub fn check(&self, roles: &[Role]) -> Result<()> {
        match roles.iter().all(|r| self.prompt(&r.prompt_file).is_some()) {
            true => Ok(()),
            false => Err(bad()),
        }
    }
}

/// The roles of `text_json` (an `assist-ruoli/1`); no role may repeat. The prompt files are checked by `Prompts::check`.
pub fn roles(text_json: &str) -> Result<Vec<Role>> {
    let doc = parse(text_json.as_bytes())?;
    if doc.get("schema").and_then(Value::str) != Some("assist-ruoli/1") {
        return Err(bad());
    }
    let mut out: Vec<Role> = Vec::new();
    for r in doc.get("roles").map_or(&[][..], Value::arr) {
        let file = text(r, "prompt")?;
        let role = Role {
            role: text(r, "role")?,
            prompt_file: file,
            model: text(r, "model")?,
            effort: text(r, "effort")?,
            max_tokens: r.get("max_tokens").and_then(Value::f64).ok_or_else(bad)? as u32,
            output_schema: text(r, "output_schema")?,
            input_sources: r
                .get("input_sources")
                .map_or(&[][..], Value::arr)
                .iter()
                .filter_map(|s| s.str().map(String::from))
                .collect(),
        };
        if out.iter().any(|o| o.role == role.role) {
            return Err(bad());
        }
        out.push(role);
    }
    Ok(out)
}
