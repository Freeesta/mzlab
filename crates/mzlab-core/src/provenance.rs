//! Provenance of an exported table (schema `provenienza/1`): the one place that decides which fields the first
//! line `# mzLab x.y.z · key=value · ...` of a CSV/TSV (or the "Provenance" sheet of an .xlsx) carries.
//! The page sends what it knows about the export as a flat JSON object; the version is the crate's own.
use crate::error::{Error, Result};
use crate::json::{self, Out, Value};

pub const SCHEMA: &str = "provenienza/1";

/// The fields written, in this order; a key that is absent or empty is left out.
pub const FIELDS: [&str; 9] = ["file", "background", "snip", "smooth", "tolerance", "polarity", "table", "date", "note"];

fn clean(s: &str) -> String {
    s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().replace(" · ", " - ").trim().to_string()
}

/// `ctx` = JSON object of strings/numbers/booleans. Returns `{"schema","version","line","rows"}`.
pub fn build(ctx: &str) -> Result<String> {
    let v = json::parse(if ctx.trim().is_empty() { b"{}" } else { ctx.as_bytes() }).map_err(|_| Error::new("err.provenance.unreadable"))?;
    if !matches!(v, Value::Obj(_)) {
        return Err(Error::new("err.provenance.unreadable"));
    }
    let version = env!("CARGO_PKG_VERSION");
    let mut rows: Vec<(String, String)> = vec![("mzLab".into(), version.into())];
    for k in FIELDS {
        let s = match v.get(k) {
            Some(Value::Str(s)) => clean(s),
            Some(Value::Num(n)) => format!("{n}"),
            Some(Value::Bool(b)) => b.to_string(),
            _ => continue,
        };
        if !s.is_empty() {
            rows.push((k.into(), s));
        }
    }
    let mut line = format!("# mzLab {version}");
    for (k, s) in rows.iter().skip(1) {
        line.push_str(&format!(" · {k}={s}"));
    }
    let mut o = Out::default();
    o.raw("{\"schema\":").str(SCHEMA).raw(",\"version\":").str(version).raw(",\"line\":").str(&line).raw(",\"rows\":[");
    for (i, (k, s)) in rows.iter().enumerate() {
        o.raw(if i == 0 { "[" } else { ",[" }).str(k).raw(",").str(s).raw("]");
    }
    o.raw("]}");
    Ok(o.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_has_version_and_fields_in_order() {
        let j: serde_json::Value = serde_json::from_str(&build(r#"{"snip":"1 min","file":"a.mzML","x":"ignored","smooth":true,"note":""}"#).unwrap()).unwrap();
        let line = j["line"].as_str().unwrap();
        assert!(line.starts_with(&format!("# mzLab {} · file=a.mzML · snip=1 min · smooth=true", env!("CARGO_PKG_VERSION"))), "{line}");
        assert!(!line.contains("ignored") && !line.contains("note") && !line.contains('\n'));
        assert_eq!(j["schema"], SCHEMA);
        assert_eq!(j["rows"][0][0], "mzLab");
    }

    #[test]
    fn control_characters_and_separators_cannot_break_the_line() {
        let j: serde_json::Value = serde_json::from_str(&build(r#"{"file":"a\tb\nc · d"}"#).unwrap()).unwrap();
        assert_eq!(j["line"].as_str().unwrap().lines().count(), 1);
        assert!(j["line"].as_str().unwrap().ends_with("file=a b c - d"));
    }

    #[test]
    fn empty_and_bad_input() {
        assert!(build("").is_ok());
        assert!(build("[1]").is_err());
    }
}
