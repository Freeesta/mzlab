//! The subagents of the data assistant as data: `data/assist/assist-ruoli-1.json` says which prompt, model, effort and output limit each role has, and the
//! prompt files are exact copies of the blocks of `docs/studi/assistente-dati/prompt/`. Nothing about a model is written in the code. Used by the agent.

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};

pub const ROLES: &str = include_str!("../data/assist/assist-ruoli-1.json");

/// The prompt files by name, in the order of `data/assist/`.
pub const PROMPTS: [(&str, &str); 8] = [
    (
        "01_sistema.md",
        include_str!("../data/assist/01_sistema.md"),
    ),
    ("02_router.md", include_str!("../data/assist/02_router.md")),
    (
        "03_metadati.md",
        include_str!("../data/assist/03_metadati.md"),
    ),
    (
        "04_cromatografia.md",
        include_str!("../data/assist/04_cromatografia.md"),
    ),
    (
        "05_parametri.md",
        include_str!("../data/assist/05_parametri.md"),
    ),
    ("06_piano.md", include_str!("../data/assist/06_piano.md")),
    (
        "07_interprete.md",
        include_str!("../data/assist/07_interprete.md"),
    ),
    (
        "08_grafici.md",
        include_str!("../data/assist/08_grafici.md"),
    ),
];

/// One subagent: what it is asked, by which model, and what it may answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Role {
    pub role: String,
    pub prompt: &'static str,
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

/// The text of a prompt file by name.
pub fn prompt(file: &str) -> Option<&'static str> {
    PROMPTS.iter().find(|(n, _)| *n == file).map(|(_, t)| *t)
}

/// SHA-256 of a prompt as it is sent (recorded in the log `llmlog/1`).
pub fn prompt_digest(file: &str) -> Option<String> {
    prompt(file).map(|t| sha256_hex(t.trim_end().as_bytes()))
}

/// The roles of `text` (an `assist-ruoli/1`); every prompt file must exist and no role may repeat.
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
            prompt: prompt(&file).ok_or_else(bad)?,
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

/// The roles of the program (`ROLES`).
pub fn builtin() -> Result<Vec<Role>> {
    roles(ROLES)
}
