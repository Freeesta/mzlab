//! The route `flow/run` of `rpc`: runs a `mzflow/2` plan of the graphical Flow with the engine of `mzlab-cli` (`flow::run_plan`), the one behind
//! `cargo run -p mzlab-cli --example flow`. The files the plan names are the ones registered with the app (they stay where they are: the run sees them
//! through links in a staging folder); the outputs go to a folder of the system temp dir that the next run replaces.

use std::path::Path;

use mzlab_core::flow::parse;
use mzlab_core::{Error, Result};
use serde_json::{json, Value};

use crate::engine::State;
use crate::rpc::{text, Answer};

/// A link to `from` named `to`: a symbolic link where the system has them, else a hard link, else a copy.
fn link(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    if std::os::unix::fs::symlink(from, to).is_ok() {
        return Ok(());
    }
    std::fs::hard_link(from, to).or_else(|_| std::fs::copy(from, to).map(|_| ()))
}

/// The largest output file the page may read back, in bytes.
const READ_MAX: u64 = 4 << 20;

impl State {
    /// `{name}` -> the text of the output file `name` of the last run (a plain file name, at most 4 MB).
    pub(crate) fn flow_read(&self, p: &Value) -> Result<Answer> {
        let name = crate::store::clean(p["name"].as_str().unwrap_or(""))?;
        let path = std::env::temp_dir()
            .join("mzlab-flow")
            .join("out")
            .join(name);
        let size = std::fs::metadata(&path)
            .map_err(|e| crate::rpc::io_error(&e))?
            .len();
        if size > READ_MAX {
            return Err(Error::new("err.flow.read.big").with("name", name));
        }
        let bytes = std::fs::read(&path).map_err(|e| crate::rpc::io_error(&e))?;
        text(String::from_utf8_lossy(&bytes))
    }

    /// `{plan: "<mzflow/2 text>"}` -> `{prov: "<prov/1 text>", dir: "<folder of the outputs>"}`. A file the plan names that is not registered is `err.flow.input.missing`.
    pub(crate) fn flow_run(&self, p: &Value) -> Result<Answer> {
        let plan_text = p["plan"]
            .as_str()
            .ok_or_else(|| Error::new("err.flow.json"))?;
        let plan = parse(plan_text.as_bytes())?;
        let root = std::env::temp_dir().join("mzlab-flow");
        let _ = std::fs::remove_dir_all(&root);
        let (inputs, work) = (root.join("in"), root.join("out"));
        for d in [&inputs, &work] {
            std::fs::create_dir_all(d).map_err(|e| crate::rpc::io_error(&e))?;
        }
        for f in &plan.inputs {
            let name = crate::store::clean(&f.name)?;
            let entry = self
                .store
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(name);
            let entry =
                entry.ok_or_else(|| Error::new("err.flow.input.missing").with("input", name))?;
            link(&entry.path, &inputs.join(name)).map_err(|e| crate::rpc::io_error(&e))?;
        }
        let workers = std::thread::available_parallelism()
            .map_or(1, |n| n.get().saturating_sub(1).clamp(1, 4));
        let (_, prov) =
            mzlab_cli::flow::run_plan(plan_text.as_bytes(), &inputs, &work, workers, None)?;
        text(json!({ "prov": prov, "dir": work.to_string_lossy() }).to_string())
    }
}
