//! `link/1`: the outside lookups of the Draw right-click menu (PubChem, ChEBI, BioTransformer) for the selection or the whole drawing. The page
//! never fetches them: it opens the URL in a new tab only when the user clicks, and copies `copy` first when the site takes no structure in its URL.
//! Here the URLs are built with RFC 3986 percent-encoding (every byte outside the unreserved set), so a SMILES or InChI with `#`, `+`, `/`, `[`, `=`
//! or `@` reaches the site intact. A part defaults to the substructure search, the whole drawing to the exact one.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out, Value};

/// `s` percent-encoded: unreserved ASCII (`A-Z a-z 0-9 - . _ ~`) kept, every other UTF-8 byte as `%XX` (upper-case hex).
pub fn pct(s: &str) -> String {
    let mut o = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

/// One row: id (the page's label key `draw.link.<id>`), the URL to open, the text to copy before opening (none when the URL carries it).
struct Link {
    id: &'static str,
    url: String,
    copy: Option<String>,
}

fn links(smiles: &str, inchi: Option<&str>, preferred: &'static str) -> Vec<Link> {
    let pubchem = |id| Link {
        id,
        url: "https://pubchem.ncbi.nlm.nih.gov/".into(),
        copy: Some(smiles.into()),
    };
    // the preferred PubChem search first, then the others in their fixed order
    let mut v = vec![pubchem(preferred)];
    for id in ["pubchem-exact", "pubchem-similar", "pubchem-sub"] {
        if id != preferred {
            v.push(pubchem(id));
        }
    }
    // ChEBI documents `/chebi/search?query=` with an InChI; only offered when Indigo gave one
    if let Some(i) = inchi.filter(|i| i.starts_with("InChI=")) {
        v.push(Link {
            id: "chebi",
            url: format!("https://www.ebi.ac.uk/chebi/search?query={}", pct(i)),
            copy: None,
        });
    }
    // BioTransformer's prediction page has a SMILES box and no documented URL parameter for it
    v.push(Link {
        id: "biotransformer",
        url: "https://biotransformer.ca/new".into(),
        copy: Some(smiles.into()),
    });
    v
}

/// `link/1` of a request `{"smiles", "inchi"?, "part"}`: `{"format", "part", "preferred", "links": [{"id", "url", "copy"}]}`, `preferred` the id
/// of the PubChem search the rows start with (a part: substructure; the whole drawing: exact). An empty SMILES or a SMILES with a space or control character is `error.link.request`.
pub fn link_json(request: &str) -> Result<String> {
    let bad = || Error::new("error.link.request");
    let v = json::parse(request.as_bytes()).map_err(|_| bad())?;
    let smiles = v.get("smiles").and_then(Value::str).ok_or_else(bad)?;
    if smiles.is_empty() || smiles.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(bad());
    }
    let inchi = v.get("inchi").and_then(Value::str).map(str::trim);
    let part = v.get("part").and_then(Value::bool).unwrap_or(false);
    let preferred = if part { "pubchem-sub" } else { "pubchem-exact" };
    let mut o = Out::default();
    o.raw("{\"format\":\"link/1\",\"part\":")
        .raw(if part { "true" } else { "false" })
        .raw(",\"preferred\":")
        .str(preferred)
        .raw(",\"links\":[");
    for (k, l) in links(smiles, inchi, preferred).iter().enumerate() {
        o.raw(if k > 0 { ",{\"id\":" } else { "{\"id\":" })
            .str(l.id)
            .raw(",\"url\":")
            .str(&l.url)
            .raw(",\"copy\":");
        match &l.copy {
            Some(c) => o.str(c),
            None => o.raw("null"),
        };
        o.raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}

#[cfg(test)]
#[path = "link_tests.rs"]
mod tests;
