//! What a `mol draw` spec is: a SMILES, a molfile text, a name, an InChIKey or an InChI, told apart by prefix or by shape (`classify`).

use mzlab_core::molecules;

/// The kind of a spec (`smiles`, `molfile`, `name`, `inchikey`, `inchi`), as `draw` reads it.
pub fn kind_of(spec: &str) -> &'static str {
    classify(spec).0
}

/// The kind of a spec and the text that follows its prefix. Auto-detection: a molfile text, an InChI, a 27-character key, a text made of
/// SMILES tokens only, otherwise a name.
pub(super) fn classify(spec: &str) -> (&'static str, &str) {
    let t = spec.trim();
    for (p, kind) in [
        ("smiles:", "smiles"),
        ("name:", "name"),
        ("inchikey:", "inchikey"),
    ] {
        if let Some(r) = t.strip_prefix(p) {
            return (kind, r.trim());
        }
    }
    if let Some(r) = spec.strip_prefix("molfile:") {
        return ("molfile", r);
    }
    if spec.contains("M  END") {
        return ("molfile", spec);
    }
    if t.starts_with("InChI=") {
        // an InChI is never parsed or hashed here: only the InChIKey written beside it can find the molecule
        let key = t
            .split_whitespace()
            .map(|w| w.strip_prefix("InChIKey=").unwrap_or(w))
            .find(|w| molecules::is_inchikey(w));
        return key.map_or(("inchi", t), |k| ("inchikey", k));
    }
    if molecules::is_inchikey(t) {
        return ("inchikey", t);
    }
    if smiles_tokens(t) {
        return ("smiles", t);
    }
    ("name", t)
}

/// Whether a text is made of SMILES tokens only: bracket atoms, the organic subset (`Cl` and `Br` as two letters, aromatic in lower case),
/// bonds, branches, ring digits, `%nn` and dots, with at least one atom.
pub(super) fn smiles_tokens(t: &str) -> bool {
    let c: Vec<char> = t.chars().collect();
    let (mut i, mut atoms) = (0, 0);
    while i < c.len() {
        match c[i] {
            '[' => {
                let Some(j) = c[i..].iter().position(|&x| x == ']') else {
                    return false;
                };
                i += j;
                atoms += 1;
            }
            'C' if c.get(i + 1) == Some(&'l') => {
                i += 1;
                atoms += 1;
            }
            'B' if c.get(i + 1) == Some(&'r') => {
                i += 1;
                atoms += 1;
            }
            'B' | 'C' | 'N' | 'O' | 'P' | 'S' | 'F' | 'I' | 'b' | 'c' | 'n' | 'o' | 'p' | 's'
            | '*' => atoms += 1,
            '0'..='9'
            | '('
            | ')'
            | '='
            | '#'
            | '$'
            | ':'
            | '/'
            | '\\'
            | '.'
            | '%'
            | '@'
            | '+'
            | '-' => {}
            _ => return false,
        }
        i += 1;
    }
    atoms > 0
}
