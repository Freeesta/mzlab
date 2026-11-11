//! `sub/1`: where a drawn moiety occurs in a list of structures (feature `record`, matcher `mzlab_mol::SubQuery`). The request is
//! `{"query", "targets": [..], "stereo", "max"}` with SMILES or molfiles as text; the answer lists, per target that
//! contains the query, its distinct matches (target atoms of every query atom, merged query hydrogens left out). A hit is a suspect to
//! check, never an identification. The same bytes from the `.wasm` `sub`, the desktop route `feat/grafo` and `mzlab-cli mol-search --sub`.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out};
use mzlab_mol::{read_target, SubOptions, SubQuery};

/// Most matches listed per target; the count `n` stops there too.
const MAX_MATCHES: usize = 1000;

/// One entry of `hits`: the target index, the number of matches and the target atoms of each.
fn hit(o: &mut Out, first: bool, i: usize, rows: &[Vec<u32>]) {
    o.raw(if first { "{\"i\":" } else { ",{\"i\":" })
        .int(i as i64)
        .raw(",\"n\":")
        .int(rows.len() as i64)
        .raw(",\"maps\":[");
    for (k, r) in rows.iter().enumerate() {
        o.raw(if k > 0 { ",[" } else { "[" });
        for (j, a) in r.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).int(i64::from(*a));
        }
        o.raw("]");
    }
    o.raw("]}");
}

/// The `sub/1` answer of a request.
pub fn search_json(request: &str) -> Result<String> {
    let bad = || Error::new("error.sub.request");
    let v = json::parse(request.as_bytes()).map_err(|_| bad())?;
    let query = v.get("query").and_then(|q| q.str()).ok_or_else(bad)?;
    let targets = v.get("targets").map(|t| t.arr()).ok_or_else(bad)?;
    let opts = SubOptions {
        stereo: v.get("stereo").and_then(|s| s.bool()).unwrap_or(false),
        max_matches: v
            .get("max")
            .and_then(|m| m.f64())
            .map_or(MAX_MATCHES, |m| (m.max(1.0) as usize).min(MAX_MATCHES)),
        ..SubOptions::default()
    };
    let q =
        SubQuery::from_text(query).map_err(|e| Error::new("error.sub.query").with("detail", e))?;
    let mut o = Out::default();
    o.raw("{\"format\":\"sub/1\",\"atoms\":")
        .int(q.mapped_atoms() as i64)
        .raw(",\"stereo\":")
        .raw(if opts.stereo { "true" } else { "false" })
        .raw(",\"hits\":[");
    let mut errors = Out::default();
    let (mut first, mut first_err) = (true, true);
    for (i, t) in targets.iter().enumerate() {
        let found = t
            .str()
            .ok_or_else(|| mzlab_mol::MolError::Json("not a text".into()))
            .and_then(read_target)
            .and_then(|m| q.matches(&m, &opts));
        match found {
            Ok(rows) if rows.is_empty() => {}
            Ok(rows) => {
                hit(&mut o, first, i, &rows);
                first = false;
            }
            Err(e) => {
                errors
                    .raw(if first_err { "{\"i\":" } else { ",{\"i\":" })
                    .int(i as i64)
                    .raw(",\"error\":")
                    .str(&e.to_string())
                    .raw("}");
                first_err = false;
            }
        }
    }
    o.raw("],\"errors\":[").raw(&errors.0).raw("]}");
    Ok(o.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_and_errors() {
        let r = search_json(
            r#"{"query":"S(=O)(=O)N","targets":["CS(=O)(=O)N","CCO","C1CC","NS(N)(=O)=O"]}"#,
        )
        .unwrap();
        assert_eq!(
            r,
            r#"{"format":"sub/1","atoms":4,"stereo":false,"hits":[{"i":0,"n":1,"maps":[[1,2,3,4]]},{"i":3,"n":2,"maps":[[1,3,4,0],[1,3,4,2]]}],"errors":[{"i":2,"error":"SMILES: ring closure 1 is never closed"}]}"#
        );
        assert_eq!(search_json("{}").unwrap_err().key, "error.sub.request");
        assert_eq!(
            search_json(r#"{"query":"C1C","targets":[]}"#)
                .unwrap_err()
                .key,
            "error.sub.query"
        );
    }
}
