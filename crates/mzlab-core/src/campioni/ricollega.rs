//! Finds again the files of a `campioni/2` that were moved or renamed: by SHA-256 first and, only when a side does not know its SHA-256, by the
//! same file name (flagged `by_name`, so the page can ask the user). A known SHA-256 that differs is a replaced file: never matched by name.

use super::Campione;
use crate::json::Out;

/// A file of the series found again at `path`; `by_name` when the SHA-256 could not decide.
#[derive(Debug, Clone, PartialEq)]
pub struct Abbinamento {
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
pub fn ricollega(cs: &[Campione], candidates: &[(String, String)]) -> Vec<Abbinamento> {
    let mut cand: Vec<&(String, String)> = candidates.iter().collect();
    crate::sortc::slice_by(&mut cand, |a, b| a.0.cmp(&b.0));
    let mut used = vec![false; cand.len()];
    let mut out: Vec<Abbinamento> = Vec::new();
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
            out.push(Abbinamento {
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
            out.push(Abbinamento {
                index: i,
                path: cand[k].0.clone(),
                by_name: true,
            });
        }
    }
    crate::sortc::slice_by(&mut out, |a, b| a.index.cmp(&b.index));
    out
}

/// `{"abbinati":[{"indice","percorso","per_nome"}]}`.
pub fn ricollega_json(a: &[Abbinamento]) -> String {
    let mut j = Out::default();
    j.raw("{\"abbinati\":[");
    for (i, x) in a.iter().enumerate() {
        j.raw(if i > 0 {
            ",{\"indice\":"
        } else {
            "{\"indice\":"
        })
        .int(x.index as i64)
        .raw(",\"per_nome\":")
        .raw(if x.by_name { "true" } else { "false" })
        .raw(",\"percorso\":")
        .str(&x.path)
        .raw("}");
    }
    j.raw("]}");
    j.0
}
