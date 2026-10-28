//! A SQLite file built by hand (pages, records, a b-tree) for the tests of the Bruker reader, with no real data.
#![allow(dead_code)]

use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limiti;
use mzlab_lettori::bruker::sqlite_min::Sqlite;

pub const PAG: usize = 4096;

pub enum V<'a> {
    N,
    I(i64),
    R(f64),
    T(&'a str),
}

pub fn varint(mut v: u64) -> Vec<u8> {
    let mut o = vec![(v & 0x7f) as u8];
    v >>= 7;
    while v > 0 {
        o.push((v & 0x7f) as u8 | 0x80);
        v >>= 7;
    }
    o.reverse();
    o
}

pub fn record(vals: &[V<'_>]) -> Vec<u8> {
    let (mut tipi, mut corpo) = (Vec::new(), Vec::new());
    for v in vals {
        match v {
            V::N => tipi.push(0),
            V::I(x) => {
                tipi.push(6);
                corpo.extend(x.to_be_bytes());
            }
            V::R(x) => {
                tipi.push(7);
                corpo.extend(x.to_be_bytes());
            }
            V::T(s) => {
                tipi.extend(varint(13 + 2 * s.len() as u64));
                corpo.extend(s.as_bytes());
            }
        }
    }
    let mut o = vec![(tipi.len() + 1) as u8];
    o.extend(tipi);
    o.extend(corpo);
    o
}

/// A b-tree page of `tipo` (13 leaf table, 10 leaf index, 5 interior table) with its `celle`, the header at `base`.
pub fn pagina(tipo: u8, base: usize, celle: &[Vec<u8>], destra: u32) -> Vec<u8> {
    let mut p = vec![0u8; PAG];
    let mut fine = PAG;
    let mut ptr = Vec::new();
    for c in celle {
        fine -= c.len();
        p[fine..fine + c.len()].copy_from_slice(c);
        ptr.push(fine as u16);
    }
    let intest = if tipo == 5 { 12 } else { 8 };
    p[base] = tipo;
    p[base + 3..base + 5].copy_from_slice(&(celle.len() as u16).to_be_bytes());
    p[base + 5..base + 7].copy_from_slice(&(fine as u16).to_be_bytes());
    if tipo == 5 {
        p[base + 8..base + 12].copy_from_slice(&destra.to_be_bytes());
    }
    for (i, o) in ptr.iter().enumerate() {
        let a = base + intest + 2 * i;
        p[a..a + 2].copy_from_slice(&o.to_be_bytes());
    }
    p
}

pub fn cella_foglia(rowid: u64, rec: &[u8]) -> Vec<u8> {
    let mut c = varint(rec.len() as u64);
    c.extend(varint(rowid));
    c.extend(rec);
    c
}

pub fn foglia(righe: &[Vec<V<'_>>]) -> Vec<u8> {
    let c: Vec<Vec<u8>> = righe
        .iter()
        .enumerate()
        .map(|(i, r)| cella_foglia(i as u64 + 1, &record(r)))
        .collect();
    pagina(13, 0, &c, 0)
}

/// A database whose tables are `(name, CREATE text, page image)`; table `i` is rooted at page `i + 2`.
pub fn db(tabelle: &[(&str, &str, Vec<u8>)]) -> Vec<u8> {
    let schema: Vec<Vec<u8>> = tabelle
        .iter()
        .enumerate()
        .map(|(i, (n, sql, _))| {
            cella_foglia(
                i as u64 + 1,
                &record(&[
                    V::T("table"),
                    V::T(n),
                    V::T(n),
                    V::I(i as i64 + 2),
                    V::T(sql),
                ]),
            )
        })
        .collect();
    let mut p1 = pagina(13, 100, &schema, 0);
    p1[..16].copy_from_slice(b"SQLite format 3\0");
    p1[16..18].copy_from_slice(&(PAG as u16).to_be_bytes());
    p1[18] = 1;
    p1[19] = 1;
    p1[21..24].copy_from_slice(&[64, 32, 32]);
    p1[28..32].copy_from_slice(&(tabelle.len() as u32 + 1).to_be_bytes());
    p1[44..48].copy_from_slice(&4u32.to_be_bytes());
    p1[56..60].copy_from_slice(&1u32.to_be_bytes());
    let mut out = p1;
    for (_, _, p) in tabelle {
        out.extend(p);
    }
    out
}

pub fn limiti() -> Limiti {
    Limiti::sito()
}

pub fn apri_db(b: Vec<u8>, wal: Option<Vec<u8>>) -> mzlab_core::error::Result<Sqlite> {
    let dim = b.len() as u64;
    let w = wal.map(|w| {
        let n = w.len() as u64;
        (Box::new(w) as Box<dyn ReadAt>, n)
    });
    Sqlite::apri(Box::new(b), dim, w, &limiti())
}
