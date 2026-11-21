//! `--mz-offset DA|auto:MZ[@RT]` of `trace xic` and `features`: the flag, its stderr lines and the warning of an estimate that did not go cleanly.
//! The engine (`mzlab_core::mzcal`) measures and applies it; this only reads the flag and says what was done.

use mzlab_core::mzcal::{Offset, Spec};

use crate::extract::Parsed;

/// The flag as a `Spec` (`Zero` when absent); the error is the key and its values, a usage error (exit 2).
pub(crate) fn flag(p: &Parsed) -> std::result::Result<Spec, String> {
    let Some(t) = p.get("--mz-offset") else {
        return Ok(Spec::Zero);
    };
    check(t)?;
    Ok(Spec::parse(t).unwrap_or_default())
}

/// A `--mz-offset` text that is not a number or `auto:MZ[@RT]` within the limit: the message of the usage error (exit 2).
pub(crate) fn check(t: &str) -> std::result::Result<(), String> {
    Spec::parse(t).map(drop).map_err(|e| {
        e.params
            .iter()
            .fold(e.key.to_string(), |a, (k, v)| format!("{a} {k}={v}"))
    })
}

/// The stderr lines for `name`: what was applied, and the warning when the estimate found no ion or a wide one. Nothing for an offset of 0.
pub(crate) fn lines(name: &str, o: &Offset) -> String {
    if o.da == 0.0 && o.auto.is_none() {
        return String::new();
    }
    let mut s = format!("{}\n", o.line(name));
    if let Some(k) = o.warning() {
        let text = crate::error_text::catalog_text(k).unwrap_or_default();
        s += &format!("warning: {name}: {text} ({k})\n");
    }
    s
}
