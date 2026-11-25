//! The question of `mzlab-cli raw xic` as the flags give it (`raw_xic_cmd`): `--set` names, the route query built from them and the cap and polarity
//! the CLI keeps. The route (`mzlab_core::rawxic::Ask`) reads the same text, so a value the site would refuse is refused here before any file is opened.

use std::path::PathBuf;

use mzlab_core::error::Error;
use mzlab_core::rawxic::Ask;
use mzlab_core::routes::Query;

use crate::extract::Parsed;

/// Rows printed by default.
const LIMIT: usize = 50;
/// The keys of `--set` and of the route.
const SET_KEYS: &[&str] = &["min_sn", "min_points", "alpha", "side_min", "gap_min"];

/// The question: the files, the route question (`mzlab_core::rawxic::Ask`), the cap and the polarity.
pub(crate) struct XicAsk {
    pub(crate) files: Vec<PathBuf>,
    pub(crate) ask: Ask,
    pub(crate) limit: usize,
    pub(crate) pol: Option<i8>,
}

/// A query value with every byte but the unreserved ones percent-encoded (`auto:300@5` keeps its meaning through `Query::parse`).
fn enc(v: &str) -> String {
    v.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// `--set a=1,b=2` (repeatable) as pairs; a name outside `SET_KEYS` is refused.
fn set_of(p: &Parsed) -> std::result::Result<Vec<(String, String)>, Error> {
    let mut out: Vec<(String, String)> = Vec::new();
    for v in p.all("--set") {
        for item in v.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (k, val) = item
                .split_once('=')
                .filter(|(k, _)| SET_KEYS.contains(k) || crate::raw_xic_ions::FACT_KEYS.contains(k))
                .ok_or_else(|| {
                    Error::new("err.cli.rawxic.set").with("item", item).with(
                        "names",
                        [SET_KEYS, crate::raw_xic_ions::FACT_KEYS]
                            .concat()
                            .join(", "),
                    )
                })?;
            out.retain(|(n, _)| n != k);
            out.push((k.to_string(), val.to_string()));
        }
    }
    Ok(out)
}

pub(crate) fn xic_ask_of(p: &Parsed) -> std::result::Result<XicAsk, Error> {
    let files: Vec<PathBuf> = p.pos.iter().map(PathBuf::from).collect();
    if files.is_empty() {
        return Err(Error::new("err.cli.rawxic.needsFile"));
    }
    let mz = p
        .get("--mz")
        .ok_or_else(|| Error::new("err.cli.rawxic.needsMz"))?;
    let rt = p
        .get("--rt")
        .ok_or_else(|| Error::new("err.cli.rawxic.needsRt"))?;
    let set = set_of(p)?;
    let ions = p.flag("--ions");
    if !ions
        && (p.get("--scan-type").is_some()
            || set
                .iter()
                .any(|(k, _)| crate::raw_xic_ions::FACT_KEYS.contains(&k.as_str())))
    {
        return Err(Error::new("err.cli.rawxic.needsIons"));
    }
    let mut query = format!("mz={}&rt={}", enc(mz), enc(rt));
    if ions {
        query += "&ions=1";
        if let Some(v) = p.get("--scan-type") {
            query += &format!("&scan_type={}", enc(v));
        }
    }
    for (flag, key) in [
        ("--da", "da"),
        ("--ms-level", "ms_level"),
        ("--mz-offset", "mz_offset"),
    ] {
        if let Some(v) = p.get(flag) {
            query += &format!("&{key}={}", enc(v));
        }
    }
    for (k, v) in &set {
        query += &format!("&{k}={}", enc(v));
    }
    let limit = match p.get("--limit") {
        None => LIMIT,
        Some(v) => v
            .parse::<usize>()
            .ok()
            .filter(|n| *n >= 1)
            .ok_or_else(|| Error::new("err.cli.rawxic.limit").with("value", v))?,
    };
    let pol = match p.get("--polarity") {
        None => None,
        Some("+") => Some(1),
        Some("-") => Some(-1),
        Some(v) => return Err(Error::new("err.cli.rawxic.polarity").with("value", v)),
    };
    // the route reads the same text, so a value the site would refuse is refused here before any file is opened
    let ask = Ask::of(&Query::parse(&query))?;
    Ok(XicAsk {
        files,
        ask,
        limit,
        pol,
    })
}
