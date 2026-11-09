//! The library file `msnlib/1` of the user (`docs/agenti/msnlib.md`): `aggiungi` appends the entry of one standard in one condition (ion,
//! polarity, activations) to the text of the library and answers whether the same entry was already there. One entry per line, sorted by
//! (id, ion, activations), so the same entries give the same bytes. Used by the `.wasm` module of the page (`api/msnlib/aggiungi`).

use super::{Graph, SpettroG};
use crate::error::{Error, Result};
use crate::json::{parse, Out};

const TESTA: &str = "{\"formato\":\"msnlib/1\",\"voci\":[";
const TAIL: &str = "]}";

/// The activations of the spectra of the graph, sorted and without repeats.
fn attivazioni(spettri: &[SpettroG]) -> Vec<&str> {
    let mut a: Vec<&str> = spettri.iter().map(|s| s.att.as_str()).collect();
    crate::sortc::slice_ord(&mut a);
    a.dedup();
    a
}

/// Identity of an entry: standard, ion, polarity and activations (what makes two entries "the same measurement condition").
fn key(id: &str, formula: &str, polarita: i64, att: &[&str]) -> String {
    format!("{id}|{formula}|{polarita}|{}", att.join(","))
}

/// The line of an entry and its key; the graph must have the formula of the ion (an entry without it cannot be searched).
fn entry(id: &str, g: &Graph, graph: &str) -> Result<(String, String)> {
    let ione = g
        .ione
        .as_ref()
        .ok_or_else(|| Error::new("err.msnlib.noIon"))?;
    let att = attivazioni(&g.spettri);
    let mut o = Out::default();
    o.raw("{\"id\":").str(id).raw(",\"ione\":{\"formula\":");
    o.str(&ione.formula)
        .raw(",\"mz_teorico\":")
        .num(ione.theoretical_mz);
    o.raw(",\"polarita\":")
        .int(g.polarita as i64)
        .raw("},\"attivazione\":[");
    for (i, a) in att.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(a);
    }
    o.raw("],\"grafo\":").raw(graph).raw("}");
    Ok((o.0, key(id, &ione.formula, g.polarita as i64, &att)))
}

/// The key of an entry already in the library, read from its line.
fn key_of(row: &str) -> Result<String> {
    let bad = || Error::new("err.msnlib.bad");
    let v = parse(row.as_bytes()).map_err(|_| bad())?;
    let id = v.get("id").and_then(|x| x.str()).ok_or_else(bad)?;
    let i = v.get("ione").ok_or_else(bad)?;
    let f = i.get("formula").and_then(|x| x.str()).ok_or_else(bad)?;
    let p = i.get("polarita").and_then(|x| x.f64()).ok_or_else(bad)?;
    let att: Vec<&str> = v
        .get("attivazione")
        .map(|a| a.arr().iter().filter_map(|x| x.str()).collect())
        .unwrap_or_default();
    Ok(key(id, f, p as i64, &att))
}

/// The entries (line, key) of a library text; `""` is an empty library.
fn rows(library: &str) -> Result<Vec<(String, String)>> {
    if library.trim().is_empty() {
        return Ok(vec![]);
    }
    let bad = || Error::new("err.msnlib.bad");
    let corpo = library
        .trim_end()
        .strip_prefix(TESTA)
        .and_then(|t| t.strip_suffix(TAIL))
        .ok_or_else(bad)?;
    corpo
        .lines()
        .map(|l| l.trim().trim_end_matches(','))
        .filter(|l| !l.is_empty())
        .map(|l| Ok((l.to_string(), key_of(l)?)))
        .collect()
}

/// The library with the entry of `id` and the graph added, and `false`; the same text and `true` (a duplicate) when the library already has
/// an entry for the same standard, ion, polarity and activations. `grafo` is the `msngrafo/1` text of `g`.
pub fn aggiungi(library: &str, id: &str, g: &Graph, graph: &str) -> Result<(String, bool)> {
    let mut v = rows(library)?;
    let (row, k) = entry(id, g, graph)?;
    if v.iter().any(|(_, c)| *c == k) {
        return Ok((library.to_string(), true));
    }
    v.push((row, k));
    crate::sortc::slice_by(&mut v, |a, b| a.1.cmp(&b.1));
    let corpo: Vec<&str> = v.iter().map(|(r, _)| r.as_str()).collect();
    Ok((format!("{TESTA}\n{}\n{TAIL}\n", corpo.join(",\n")), false))
}

#[cfg(test)]
mod tests {
    use super::super::{Ione, SpettroG};
    use super::*;

    fn graph(formula: &str, att: &str) -> Graph {
        let mut g = Graph {
            polarita: 1,
            ione: Some(Ione {
                formula: formula.into(),
                theoretical_mz: 195.0877,
                mz_observed: 195.0878,
                ppm: 0.5,
            }),
            ..Graph::default()
        };
        g.spettri.push(SpettroG {
            albero: "195".into(),
            level: 2,
            att: att.into(),
            ce: 30.0,
            precursore: None,
            width: None,
            picchi: vec![],
        });
        g
    }

    #[test]
    fn an_entry_is_added_once_and_the_library_stays_sorted() {
        let (l1, d1) = aggiungi("", "STD-2", &graph("C8H11N4O2+", "hcd"), "{}").unwrap();
        assert!(!d1 && l1.starts_with(TESTA) && l1.trim_end().ends_with(TAIL));
        let (l2, d2) = aggiungi(&l1, "STD-1", &graph("C8H11N4O2+", "hcd"), "{}").unwrap();
        assert!(!d2 && l2.find("STD-1").unwrap() < l2.find("STD-2").unwrap());
        // the same standard, ion and activation is a duplicate and leaves the text as it was
        let (l3, d3) = aggiungi(&l2, "STD-1", &graph("C8H11N4O2+", "hcd"), "{}").unwrap();
        assert!(d3 && l3 == l2);
        // another activation of the same standard is another entry
        let (l4, d4) = aggiungi(&l2, "STD-1", &graph("C8H11N4O2+", "cid"), "{}").unwrap();
        assert!(!d4 && l4.lines().count() == l2.lines().count() + 1);
        // the same entries added in the other order give the same bytes
        let (m1, _) = aggiungi("", "STD-1", &graph("C8H11N4O2+", "hcd"), "{}").unwrap();
        let (m2, _) = aggiungi(&m1, "STD-2", &graph("C8H11N4O2+", "hcd"), "{}").unwrap();
        assert_eq!(m2, l2);
    }

    #[test]
    fn a_graph_without_the_ion_or_a_foreign_file_is_a_keyed_error() {
        let g = Graph::default();
        assert_eq!(
            aggiungi("", "S", &g, "{}").err().unwrap().key,
            "err.msnlib.noIon"
        );
        let g = graph("C8H11N4O2+", "hcd");
        assert_eq!(
            aggiungi("[1,2]", "S", &g, "{}").err().unwrap().key,
            "err.msnlib.bad"
        );
    }
}
