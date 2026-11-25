//! `raw-plan/1`: the plan document of the `raw` commands that read a peak or a spectrum (`--dry-run`): the resolved query and each file with its size
//! and cache status; no scan is read.
use std::path::PathBuf;

use crate::extract::{jstr, Parsed};
use crate::raw_cmd::name_of;
use crate::Outcome;

/// The plan document of any `raw` command: `params` (JSON) as the query, the files with their size and cache status.
pub(crate) fn plan_of(p: &Parsed, files: &[PathBuf], command: &str, params: &str) -> Outcome {
    let dir = crate::idx_cache::dir();
    let mut rows = Vec::new();
    for f in files {
        let label = match p.sample(f) {
            Ok(s) => s.map_or(String::new(), |id| format!("{}@{id}", name_of(f))),
            Err(m) => return Outcome::fail(&m),
        };
        let cache = match &dir {
            None => "none".to_string(),
            Some(d) => match crate::idx_cache::peek(d, f, &label) {
                Ok((st, _, _)) => st.to_string(),
                Err(e) => return crate::sample_select::fail(&e),
            },
        };
        let size = std::fs::metadata(f).ok().map(|m| m.len());
        rows.push(format!(
            "{{\"file\":{},\"size_bytes\":{},\"cache\":\"{cache}\"}}",
            jstr(&name_of(f)),
            size.map_or("null".into(), |n| n.to_string())
        ));
    }
    let text = format!(
        "{{\"schema\":\"raw-plan/1\",\"command\":\"{command}\",\"query\":{params},\"files\":[{}]}}\n",
        rows.join(",")
    );
    crate::emit(text, &p.out())
}
