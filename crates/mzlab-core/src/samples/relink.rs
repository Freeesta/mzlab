//! Finds again the files of a `samples/1` that were moved or renamed: by SHA-256 first and, only when a side does not know its SHA-256, by the
//! same file name (flagged `by_name`, so the page can ask the user). A known SHA-256 that differs is a replaced file: never matched by name.

use super::Sample;
use crate::json::Out;

/// A file of the series found again at `path`; `by_name` when the SHA-256 could not decide.
#[derive(Debug, Clone, PartialEq)]
pub struct RelinkMatch {
    pub index: usize,
    pub path: String,
    pub by_name: bool,
}

/// The name of a file without its folders (`/` or `\`).
fn base(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// `candidates` are (path, sha256 or "" when unknown). One candidate serves one file; the candidates are taken in the order of their path, so the
/// same input gives the same answer. Among copies with the same SHA-256 the one with the same name is preferred. A name shared by several free
/// candidates is not guessed. The answer is in the order of the files.
pub fn relink(cs: &[Sample], candidates: &[(String, String)]) -> Vec<RelinkMatch> {
    let mut cand: Vec<&(String, String)> = candidates.iter().collect();
    crate::sortc::slice_by(&mut cand, |a, b| a.0.cmp(&b.0));
    let mut used = vec![false; cand.len()];
    let mut out: Vec<RelinkMatch> = Vec::new();
    for (i, c) in cs.iter().enumerate() {
        if c.sha256.is_empty() {
            continue;
        }
        let same = |k: &usize| !used[*k] && cand[*k].1 == c.sha256;
        let pick = (0..cand.len())
            .filter(same)
            .find(|k| base(&cand[*k].0) == base(&c.file))
            .or_else(|| (0..cand.len()).find(same));
        if let Some(k) = pick {
            used[k] = true;
            out.push(RelinkMatch {
                index: i,
                path: cand[k].0.clone(),
                by_name: false,
            });
        }
    }
    for (i, c) in cs.iter().enumerate() {
        if out.iter().any(|a| a.index == i) {
            continue;
        }
        let free: Vec<usize> = (0..cand.len())
            .filter(|k| {
                !used[*k]
                    && base(&cand[*k].0) == base(&c.file)
                    && (c.sha256.is_empty() || cand[*k].1.is_empty())
            })
            .collect();
        if let [k] = free[..] {
            used[k] = true;
            out.push(RelinkMatch {
                index: i,
                path: cand[k].0.clone(),
                by_name: true,
            });
        }
    }
    crate::sortc::slice_by(&mut out, |a, b| a.index.cmp(&b.index));
    out
}

/// `{"matches":[{"index","path","by_name"}]}`.
pub fn relink_json(a: &[RelinkMatch]) -> String {
    let mut j = Out::default();
    j.raw("{\"matches\":[");
    for (i, x) in a.iter().enumerate() {
        j.raw(if i > 0 { ",{\"index\":" } else { "{\"index\":" })
            .int(x.index as i64)
            .raw(",\"by_name\":")
            .raw(if x.by_name { "true" } else { "false" })
            .raw(",\"path\":")
            .str(&x.path)
            .raw("}");
    }
    j.raw("]}");
    j.0
}
