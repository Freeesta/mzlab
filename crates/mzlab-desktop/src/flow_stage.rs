//! Puts the inputs of a `flow/run` plan into its staging folder: the text the page holds (a sample list) is written, a file registered with the app is
//! linked. Used by `flow_run`.

use std::path::Path;

use mzlab_core::{Error, Result};
use serde_json::Value;

/// Puts each input of the plan into `dir`: the text the page sent for it in `texts` (a sample list the page holds, `{name: text}`), else a link to
/// the file registered with the app (`held`); a name that is neither is `err.flow.input.missing`.
pub(crate) fn stage_inputs(
    names: &[&str],
    texts: &Value,
    held: &dyn Fn(&str) -> Option<std::path::PathBuf>,
    dir: &Path,
) -> Result<()> {
    for raw in names {
        let name = crate::store::clean(raw)?;
        if let Some(t) = texts[name].as_str() {
            std::fs::write(dir.join(name), t).map_err(|e| crate::rpc::io_error(&e))?;
            continue;
        }
        let path =
            held(name).ok_or_else(|| Error::new("err.flow.input.missing").with("input", name))?;
        crate::flow_run::link(&path, &dir.join(name)).map_err(|e| crate::rpc::io_error(&e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_input_the_page_sent_as_text_needs_no_registering() {
        let dir = std::env::temp_dir().join(format!("mzlab-flow-in-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let none = |_: &str| None;
        let texts = json!({ "campioni.json": "{\"schema\":\"campioni/2\"}" });
        stage_inputs(&["campioni.json"], &texts, &none, &dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("campioni.json")).unwrap(),
            "{\"schema\":\"campioni/2\"}"
        );
        let e = stage_inputs(&["a.mzML"], &texts, &none, &dir).unwrap_err();
        assert_eq!(e.key, "err.flow.input.missing");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
