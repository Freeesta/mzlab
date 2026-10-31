//! Puts the inputs of a `flow/run` plan into its staging folder: the text the page holds (a sample list) is written, `b64:` + base64 (a private `.mzsus` the page read
//! from the libraries folder) is decoded to its bytes, a file registered with the app is linked. Used by `flow_run`.

use std::path::Path;

use mzlab_core::{Error, Result};
use serde_json::Value;

/// Standard base64 (RFC 4648, padding optional, whitespace ignored) to bytes; None for any other character.
fn from_base64(text: &str) -> Option<Vec<u8>> {
    let (mut out, mut acc, mut bits) = (Vec::with_capacity(text.len() / 4 * 3), 0u32, 0u32);
    for c in text
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
    {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

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
            let bytes = match t.strip_prefix("b64:") {
                Some(b) => from_base64(b)
                    .ok_or_else(|| Error::new("err.flow.input.missing").with("input", name))?,
                None => t.as_bytes().to_vec(),
            };
            std::fs::write(dir.join(name), bytes).map_err(|e| crate::rpc::io_error(&e))?;
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
    fn a_b64_input_is_staged_as_its_bytes() {
        let dir = std::env::temp_dir().join(format!("mzlab-flow-b64-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let none = |_: &str| None;
        let texts = json!({ "imported-X.mzsus": "b64:AP+AAQL+", "bad.mzsus": "b64:A*" });
        stage_inputs(&["imported-X.mzsus"], &texts, &none, &dir).unwrap();
        assert_eq!(
            std::fs::read(dir.join("imported-X.mzsus")).unwrap(),
            [0, 255, 128, 1, 2, 254]
        );
        assert_eq!(from_base64("TWE=").unwrap(), b"Ma");
        assert!(stage_inputs(&["bad.mzsus"], &texts, &none, &dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

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
