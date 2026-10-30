//! The CAS sidecar (`mzlab-cas/1`): the records of NORMAN SusDat that have a CAS number, in the columns `resolve` needs, written by `tools/genera_sospetti.py`
//! next to the suspect index. The browser and the app read it instead of the 124 MB SusDat file; the records fill `Resolver.susdat` and the SusDat step of
//! `resolve` runs unchanged. Text, tab separated, one header line: `# mzlab-cas/1`, then `cas key ms_key formula mass form ms_smiles` per record
//! (`ms_key` and `ms_smiles` are empty when the form listed is the MS-ready one; `form` is `parent`, `salt` or `anion`).

use std::collections::BTreeSet;

use super::resolve::{Resolver, Sus};
use super::Error;

pub const FORMAT: &str = "mzlab-cas/1";

/// A SMILES that makes `form_of` give back `form` (the sidecar stores the verdict, not the structure).
fn probe(form: &str) -> &'static str {
    match form {
        "salt" => ".",
        "anion" => "[-]",
        _ => "",
    }
}

/// Fills `res.susdat` with the records of the CAS numbers in `wanted`.
pub fn read(text: &str, wanted: &BTreeSet<String>, res: &mut Resolver) -> Result<(), Error> {
    let mut lines = text.lines();
    let head = lines.next().unwrap_or("");
    if head.trim_start_matches('#').trim() != FORMAT {
        return Err(Error::parse("sidecar", "not a mzlab-cas/1 file"));
    }
    for l in lines {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 6 || !wanted.contains(f[0]) {
            continue;
        }
        res.susdat.entry(f[0].into()).or_default().push(Sus {
            key: f[1].into(),
            ms_key: f[2].into(),
            ms_smiles: f.get(6).copied().unwrap_or("").into(),
            smiles: probe(f[5]).into(),
            formula: f[3].into(),
            mass: f[4].into(),
            cid: String::new(),
        });
    }
    Ok(())
}
