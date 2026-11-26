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

/// The one-line hint of a result that is empty with no offset given: a shifted m/z axis looks like an absent ion.
pub(crate) const SHIFT_HINT: &str = "if the ion should be there, the m/z axis may be shifted: --mz-offset auto:MZ[@RT] measures the offset on a known ion, --mz-offset DA applies one";

/// The stderr line of `raw xic`: nothing at any mass and no `--mz-offset` given (else nothing).
pub(crate) fn nothing_hint(given: bool, rows: &[String]) -> String {
    let all = !rows.is_empty() && rows.iter().all(|r| r.contains("\"verdict\":\"nothing\""));
    if given || !all {
        return String::new();
    }
    format!("hint: nothing found at any mass; {SHIFT_HINT}\n")
}
