//! The route `flow/run` of `rpc`: runs a `mzflow/2` plan of the graphical Flow with the engine of `mzlab-cli` (`flow::run_plan`), the one behind
//! `cargo run -p mzlab-cli --example flow`. The files the plan names are the ones registered with the app (they stay where they are: the run sees them
//! through links in a staging folder); the outputs go to a folder of the system temp dir that the next run replaces.

use std::path::Path;

use mzlab_core::flow::parse;
use mzlab_core::{Error, Result};
use serde_json::{json, Value};

use crate::engine::State;
use crate::flow_stage::stage_inputs;
use crate::rpc::{text, Answer};

/// A link to `from` named `to`: a symbolic link where the system has them, else a hard link, else a copy.
pub(crate) fn link(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    if std::os::unix::fs::symlink(from, to).is_ok() {
        return Ok(());
    }
    std::fs::hard_link(from, to).or_else(|_| std::fs::copy(from, to).map(|_| ()))
}

/// The largest output file the page may read back, in bytes.
const READ_MAX: u64 = 4 << 20;

impl State {
    /// `{name}` -> the text of the output file `name` of the last run (a plain file name, at most 4 MB); with `encoding: "base64"` it answers `{"base64": "..."}`, the exact bytes of a binary file.
    pub(crate) fn flow_read(&self, p: &Value) -> Result<Answer> {
        let name = crate::store::clean(p["name"].as_str().unwrap_or(""))?;
        let dir = std::env::temp_dir().join("mzlab-flow").join("out");
        text(read_out(
            &dir,
            name,
            p["encoding"].as_str() == Some("base64"),
        )?)
    }

    /// `{plan: "<mzflow/2 text>", fresh?: bool, files?: {name: text}}` (`files`: inputs the page holds as text, the sample lists, which need no registering; `llm`: `{log_sha256, turn, provider, model, level}` when the assistant launched the run) -> `{prov: "<prov/1 text>", dir: "<folder of the outputs>"}`; the steps of the last runs whose key is
    /// unchanged are taken again (`cached` in `prov/1`) unless `fresh`. A file the plan names that is not registered is `err.flow.input.missing`.
    pub(crate) fn flow_run(&self, p: &Value) -> Result<Answer> {
        let plan_text = p["plan"]
            .as_str()
            .ok_or_else(|| Error::new("err.flow.json"))?;
        let plan = parse(plan_text.as_bytes())?;
        let root = std::env::temp_dir().join("mzlab-flow");
        let (inputs, work) = (root.join("in"), root.join("out"));
        // the outputs stay: their cache gives back the steps whose key did not change (`fresh` recomputes all)
        let _ = std::fs::remove_dir_all(&inputs);
        for d in [&inputs, &work] {
            std::fs::create_dir_all(d).map_err(|e| crate::rpc::io_error(&e))?;
        }
        let names: Vec<&str> = plan.inputs.iter().map(|f| f.name.as_str()).collect();
        // a maker's file whose mzML the background queue has already made is staged as that mzML
        let held = |name: &str| {
            self.conv_ready(name).or_else(|| {
                self.store
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get(name)
                    .map(|e| e.path)
            })
        };
        stage_inputs(&names, &p["files"], &held, &inputs)?;
        let fresh = p["fresh"].as_bool().unwrap_or(false);
        let workers = std::thread::available_parallelism()
            .map_or(1, |n| n.get().saturating_sub(1).clamp(1, 4));
        let done = crate::flow_progress::start();
        let (_, prov) = mzlab_cli::flow::run_plan_notify(
            plan_text.as_bytes(),
            &inputs,
            &work,
            workers,
            None,
            Some(fresh),
            Some(done),
        )?;
        // a run launched by the assistant carries `llm` in its `prov/1`
        let prov = match llm_ref(&p["llm"]) {
            Some(l) => mzlab_core::flow::prov_with_llm(&prov, &l),
            None => prov,
        };
        // the folder is a run folder for `flow/report`: prov.json beside the files of the steps
        std::fs::write(work.join("prov.json"), &prov).map_err(|e| crate::rpc::io_error(&e))?;
        text(json!({ "prov": prov, "dir": work.to_string_lossy() }).to_string())
    }

    /// `{}` -> the `corsa/1` report (`mzlab-cli flow report`) of the last run of `flow/run`, for the report page of the Flow.
    pub(crate) fn flow_report(&self, _p: &Value) -> Result<Answer> {
        let dir = std::env::temp_dir().join("mzlab-flow").join("out");
        text(mzlab_cli::flow_report::report(&dir)?)
    }
}

/// `{log_sha256, turn, provider, model, level}` of the page's assistant, or `None` when the field is absent or incomplete.
fn llm_ref(v: &Value) -> Option<mzlab_core::flow::LlmRef> {
    let s = |k: &str| v[k].as_str().map(str::to_string);
    Some(mzlab_core::flow::LlmRef {
        log_sha256: s("log_sha256")?,
        turn: u32::try_from(v["turn"].as_u64()?).ok()?,
        provider: s("provider")?,
        model: s("model")?,
        level: s("level")?,
    })
}

/// The file `name` of `dir` as the answer of `flow/read`: its text, or with `base64` a JSON object holding its bytes.
fn read_out(dir: &Path, name: &str, base64: bool) -> Result<String> {
    let path = dir.join(name);
    let size = std::fs::metadata(&path)
        .map_err(|e| crate::rpc::io_error(&e))?
        .len();
    if size > READ_MAX {
        return Err(Error::new("err.flow.read.big").with("name", name));
    }
    let bytes = std::fs::read(&path).map_err(|e| crate::rpc::io_error(&e))?;
    Ok(if base64 {
        json!({ "base64": to_base64(&bytes) }).to_string()
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

/// Standard base64 with padding (RFC 4648), written by hand to keep the app free of one more dependency.
fn to_base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = ((c[0] as u32) << 16)
            | ((*c.get(1).unwrap_or(&0) as u32) << 8)
            | (*c.get(2).unwrap_or(&0) as u32);
        for k in 0..4 {
            if k <= c.len() {
                out.push(A[((n >> (18 - 6 * k)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_folder_gives_the_corsa_report() {
        let dir = std::env::temp_dir().join(format!("mzlab-flow-rep-{}", std::process::id()));
        let work = dir.join("work");
        std::fs::create_dir_all(&work).unwrap();
        let plan = r#"{"format":"mzflow/2","inputs":[],"steps":[]}"#;
        let (_, prov) = mzlab_cli::flow::run_plan(plan.as_bytes(), &dir, &work, 1, None).unwrap();
        std::fs::write(work.join("prov.json"), prov).unwrap();
        let text = mzlab_cli::flow_report::report(&work).unwrap();
        assert!(text.contains("\"corsa/1\""), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn base64_matches_the_rfc_vectors() {
        for (raw, want) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(to_base64(raw.as_bytes()), want);
        }
    }

    #[test]
    fn a_text_file_is_read_as_before_and_a_binary_one_as_base64() {
        let dir = std::env::temp_dir().join(format!("mzlab-flow-read-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("t.csv"), "a,b\n1,2\n").unwrap();
        let bin = [0u8, 255, 128, 1, 2, 254];
        std::fs::write(dir.join("t.mzfeat"), bin).unwrap();
        assert_eq!(read_out(&dir, "t.csv", false).unwrap(), "a,b\n1,2\n");
        let v: Value = serde_json::from_str(&read_out(&dir, "t.mzfeat", true).unwrap()).unwrap();
        assert_eq!(v["base64"], to_base64(&bin));
        assert_eq!(v["base64"], "AP+AAQL+");
        assert!(read_out(&dir, "missing.bin", true).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
