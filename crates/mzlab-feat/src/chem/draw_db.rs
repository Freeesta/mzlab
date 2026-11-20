//! The lookup of `name:` and `inchikey:` specs of `mol draw` in the `molecules/1` CSV given as `--set db=PATH`: a lean reader on `molecules::table` and its own
//! ranking, because the whole `molecules` search would add about 70 KB to the molkit `.wasm`. The ranking is that of the molecule box (equal, starts with,
//! starts a word, contains); the nearest names are the ones a typing slip away.

use crate::sortc::SortC;
use mzlab_core::error::{Error, Result};
use mzlab_core::molecules;

use super::draw::{from_smiles, note, Drawn};

const NEAREST: usize = 3;

/// One molecule of the `molecules/1` CSV that has a SMILES: its name, synonyms, SMILES and InChIKey.
struct Entry {
    name: String,
    names: Vec<String>,
    smiles: String,
    key: String,
}

/// The entries of a molecules CSV (columns by header name, any order); rows without a SMILES are left out.
fn entries(csv: &str) -> Result<Vec<Entry>> {
    let rows = molecules::table(csv);
    let head: Vec<String> = rows
        .first()
        .map(|r| r.1.iter().map(|h| h.trim().to_lowercase()).collect())
        .unwrap_or_default();
    let col = |n: &str| head.iter().position(|h| h == n);
    let (Some(name), Some(smiles)) = (col("name"), col("smiles")) else {
        return Err(Error::new("err.mol.draw.db")
            .with("message", "the CSV needs the columns name and smiles"));
    };
    let (syn, key) = (col("synonyms"), col("inchikey"));
    let cell = |r: &[String], c: Option<usize>| {
        c.and_then(|c| r.get(c))
            .map_or("", |v| v.trim())
            .to_string()
    };
    Ok(rows[1..]
        .iter()
        .map(|(_, r)| {
            (
                cell(r, Some(name)),
                cell(r, syn),
                cell(r, Some(smiles)),
                cell(r, key),
            )
        })
        .filter(|e| !e.0.is_empty() && !e.2.is_empty())
        .map(|(name, syn, smiles, key)| {
            let mut names = vec![name.to_lowercase()];
            names.extend(
                syn.split('|')
                    .map(|x| x.trim().to_lowercase())
                    .filter(|x| !x.is_empty()),
            );
            Entry {
                name,
                names,
                smiles,
                key: key.to_ascii_uppercase(),
            }
        })
        .collect())
}

/// Text score of `w` (lower case) in one lower-case name: 0 equal, 1 starts with, 2 starts a word, 3 contains (the ranking of the molecule box).
fn score(name: &str, w: &str) -> Option<u8> {
    let i = name.find(w)?;
    Some(if name == w {
        0
    } else if i == 0 {
        1
    } else if name[..i]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_whitespace() || "-(,".contains(c))
    {
        2
    } else {
        3
    })
}

/// True when `a` and `b` differ by at most one inserted, deleted or replaced character.
fn within_one(a: &[char], b: &[char]) -> bool {
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if long.len() - short.len() > 1 {
        return false;
    }
    let i = short.iter().zip(long).take_while(|(x, y)| x == y).count();
    if short.len() == long.len() {
        i == short.len() || short[i + 1..] == long[i + 1..]
    } else {
        short[i..] == long[i + 1..]
    }
}

/// The names one typing slip away from `w` (four characters or more), as a whole name or as the start of one.
fn nearest(es: &[Entry], w: &str) -> Vec<String> {
    let q: Vec<char> = w.chars().collect();
    if q.len() < 4 {
        return Vec::new();
    }
    let mut hits: Vec<(u8, usize, &str)> = Vec::new();
    for e in es {
        let best = e
            .names
            .iter()
            .filter_map(|n| {
                let c: Vec<char> = n.chars().collect();
                if within_one(&q, &c) {
                    return Some(4);
                }
                (q.len() - 1..=q.len() + 1)
                    .any(|l| l < c.len() && within_one(&q, &c[..l]))
                    .then_some(5)
            })
            .min();
        if let Some(b) = best {
            hits.push((b, e.name.encode_utf16().count(), &e.name));
        }
    }
    hits.sortc_by_key(|h| (h.0, h.1));
    hits.iter().take(NEAREST).map(|h| h.2.to_string()).collect()
}

/// The rows that answer the query, best first as (row, score): an InChIKey matches its rows at score 0, a name by `score` then by length.
fn find(es: &[Entry], kind: &str, w: &str, key: &str) -> Vec<(usize, u8)> {
    if kind == "inchikey" {
        return (0..es.len())
            .filter(|&k| es[k].key == key)
            .map(|k| (k, 0))
            .collect();
    }
    let mut h: Vec<(usize, u8)> = (0..es.len())
        .filter_map(|k| {
            es[k]
                .names
                .iter()
                .filter_map(|n| score(n, w))
                .min()
                .map(|s| (k, s))
        })
        .collect();
    h.sortc_by_key(|&(k, s)| (s, es[k].name.encode_utf16().count()));
    h
}

/// No entry at all: the nearest names, when there are any, else the short message without that clause.
fn not_found(kind: &str, q: &str, es: &[Entry], w: &str) -> Error {
    let near = if kind == "inchikey" {
        Vec::new()
    } else {
        nearest(es, w)
    };
    if near.is_empty() {
        Error::new("err.mol.draw.not_found_bare").with("spec", q)
    } else {
        Error::new("err.mol.draw.not_found")
            .with("spec", q)
            .with("nearest", near.join(", "))
    }
}

/// A name or an InChIKey found in the molecules CSV `db`, drawn from its SMILES.
pub(super) fn from_db(kind: &'static str, q: &str, db: Option<&str>) -> Result<Drawn> {
    if kind == "inchi" {
        return Err(Error::new("err.mol.draw.inchi"));
    }
    let Some(csv) = db else {
        return Err(Error::new("err.mol.draw.needDb").with("kind", kind));
    };
    let es = entries(csv)?;
    let w = q.trim().to_lowercase();
    let key = q.to_ascii_uppercase();
    let mut hits = find(&es, kind, &w, &key);
    let Some(&(top, score)) = hits.first() else {
        return Err(not_found(kind, q, &es, &w));
    };
    let m = &es[top];
    let mut d = from_smiles(&m.smiles)?;
    d.title = m.name.clone();
    let (label, hit) = if kind == "inchikey" {
        ("key_hit", format!("{} ({key})", m.name))
    } else {
        ("name_hit", format!("{} (score {score})", m.name))
    };
    d.choices.push(note(label, hit));
    if score > 0 {
        d.warnings.push(note(
            "name_approximate",
            format!(
                "{q} matched {} only as part of a name (score {score}; 0 = equal)",
                m.name
            ),
        ));
    }
    hits.retain(|h| h.1 == score);
    if hits.len() > 1 {
        let names = hits.iter().take(5).map(|h| es[h.0].name.as_str());
        let key = if kind == "inchikey" {
            "key_ambiguous"
        } else {
            "name_ambiguous"
        };
        d.warnings
            .push(note(key, names.collect::<Vec<_>>().join(", ")));
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::within_one;

    fn w(a: &str, b: &str) -> bool {
        within_one(
            &a.chars().collect::<Vec<_>>(),
            &b.chars().collect::<Vec<_>>(),
        )
    }

    #[test]
    fn within_one_never_indexes_past_the_end() {
        // Equal and identical (the case that indexed `short[len + 1..]`), empty, one slip, two slips, length gap.
        assert!(w("caffeine", "caffeine"));
        assert!(w("", ""));
        assert!(w("a", "a"));
        assert!(w("caffeine", "caffeino"));
        assert!(w("caffeine", "caffeie"));
        assert!(w("caffeie", "caffeine"));
        assert!(!w("caffeine", "caffeinoo"));
        assert!(!w("caffeine", "cafxeino"));
        assert!(!w("ab", "abcd"));
    }
}
