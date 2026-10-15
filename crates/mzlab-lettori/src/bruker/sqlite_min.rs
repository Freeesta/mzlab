//! A read-only SQLite 3 reader without an SQL engine, for the `analysis.tdf` / `analysis.tsf` of a Bruker `.d` folder: the 100-byte header, the page size, the b-tree of a table (interior pages and
//! leaves), varints, the record format, overflow pages and `sqlite_schema` to find a table by name. Written from the public file-format description (https://www.sqlite.org/fileformat2.html).
//! Every page pointer, length and count read from the file passes through `Limiti`; a page is visited once (a cycle is an error), the depth of the b-tree is bounded and so are the
//! bytes of an overflow chain. A write-ahead log (`analysis.tdf-wal`, left by an acquisition that did not close its database) is read too: the latest committed frame of each page wins.

use super::sqlite_schema::colonne_di;
use super::sqlite_wal::leggi_wal;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limiti;
use std::collections::{HashMap, HashSet};

const MAGIC: &[u8; 16] = b"SQLite format 3\0";
/// The pages of a b-tree interior or leaf, of a table.
const INT_TAB: u8 = 5;
const FOGLIA_TAB: u8 = 13;
/// The pages of the b-tree of a `WITHOUT ROWID` table (an index tree whose keys are the rows).
const INT_IDX: u8 = 2;
const FOGLIA_IDX: u8 = 10;

pub(super) fn guasto(m: &str) -> Error {
    Error::new("err.dam.unreadable").with("message", format!("sqlite: {m}"))
}

/// A value of a record.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl Val {
    pub fn f64(&self) -> Option<f64> {
        match self {
            Val::Int(i) => Some(*i as f64),
            Val::Real(r) => Some(*r),
            _ => None,
        }
    }
    pub fn i64(&self) -> Option<i64> {
        match self {
            Val::Int(i) => Some(*i),
            Val::Real(r) if r.fract() == 0.0 && r.abs() < 9.0e18 => Some(*r as i64),
            _ => None,
        }
    }
    pub fn testo(&self) -> Option<&str> {
        match self {
            Val::Text(t) => Some(t),
            _ => None,
        }
    }
    /// The value as the text a person reads (numbers as they print; `None` for NULL and blobs).
    pub fn come_testo(&self) -> Option<String> {
        match self {
            Val::Text(t) => Some(t.clone()),
            Val::Int(i) => Some(i.to_string()),
            Val::Real(r) => Some(r.to_string()),
            _ => None,
        }
    }
}

/// A table of the file: its root page, the names of its columns (from the `CREATE TABLE` text) and which column is the alias of the rowid.
#[derive(Debug, Clone)]
pub struct Tabella {
    pub nome: String,
    pub radice: u32,
    pub colonne: Vec<String>,
    /// Index of the `INTEGER PRIMARY KEY` column, whose record value is NULL (the rowid stands for it).
    pub alias_rowid: Option<usize>,
    /// `WITHOUT ROWID` table: the column stored at each position of the record (the primary key comes first).
    pub ordine: Option<Vec<usize>>,
}

impl Tabella {
    pub fn col(&self, nome: &str) -> Option<usize> {
        self.colonne
            .iter()
            .position(|c| c.eq_ignore_ascii_case(nome))
    }
}

pub struct Sqlite {
    main: Box<dyn ReadAt>,
    dim: u64,
    /// Pages of the main file.
    pagine_main: u64,
    /// The write-ahead log: its bytes and, for each page, the offset of its latest committed frame.
    wal: Option<(Box<dyn ReadAt>, HashMap<u32, u64>)>,
    pagina: usize,
    /// Bytes of a page usable by cells (page size minus the reserved space).
    utile: usize,
    pagine: u64,
    tabelle: Vec<Tabella>,
    limiti: Limiti,
}

/// Reads `n` bytes at `off` (an error when the file is shorter).
pub(super) fn leggi(d: &mut dyn ReadAt, off: u64, n: usize) -> Result<Vec<u8>> {
    let mut b = vec![0u8; n];
    d.read_at(off, &mut b).map_err(|_| guasto("short read"))?;
    Ok(b)
}

/// A varint of up to 9 bytes at the start of `b`: (value, bytes used).
pub fn varint(b: &[u8]) -> Option<(u64, usize)> {
    let mut v: u64 = 0;
    for i in 0..8 {
        let x = *b.get(i)?;
        v = (v << 7) | u64::from(x & 0x7f);
        if x & 0x80 == 0 {
            return Some((v, i + 1));
        }
    }
    let x = *b.get(8)?;
    Some(((v << 8) | u64::from(x), 9))
}

fn u16_be(b: &[u8], o: usize) -> Option<usize> {
    Some(usize::from(u16::from_be_bytes([
        *b.get(o)?,
        *b.get(o + 1)?,
    ])))
}

pub(super) fn u32_be(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

impl Sqlite {
    /// Opens the database `main` of `dim` bytes (with its write-ahead log `wal`, when the folder has one) and reads its `sqlite_schema`.
    pub fn apri(
        main: Box<dyn ReadAt>,
        dim: u64,
        wal: Option<(Box<dyn ReadAt>, u64)>,
        limiti: &Limiti,
    ) -> Result<Sqlite> {
        let mut s = Sqlite {
            main,
            dim,
            pagine_main: 0,
            wal: None,
            pagina: 0,
            utile: 0,
            pagine: 0,
            tabelle: Vec::new(),
            limiti: *limiti,
        };
        let mut h = [0u8; 100];
        let letti = (dim as usize).min(100);
        s.main
            .read_at(0, &mut h[..letti])
            .map_err(|_| guasto("short read"))?;
        let mut commit = None;
        if let Some((mut w, wdim)) = wal {
            if let Some((pagina, mappa, pagine)) = leggi_wal(&mut *w, wdim, limiti)? {
                s.pagina = pagina;
                commit = Some(pagine);
                s.wal = Some((w, mappa));
            }
        }
        if s.pagina == 0 {
            let raw = u16_be(&h, 16).ok_or_else(|| guasto("header"))?;
            s.pagina = if raw == 1 { 65536 } else { raw };
        }
        if !(512..=65536).contains(&s.pagina) || !s.pagina.is_power_of_two() {
            return Err(guasto("page size"));
        }
        s.pagine_main = dim / s.pagina as u64;
        s.pagine = commit.filter(|&n| n > 0).unwrap_or(s.pagine_main);
        limiti.voci_ok(s.pagine)?;
        // page 1 may live in the log only: its first 100 bytes are the header of the database
        let p1 = s.pagina_n(1)?;
        if p1.len() < 100 || &p1[..16] != MAGIC {
            return Err(guasto("not a database"));
        }
        s.utile = s
            .pagina
            .checked_sub(usize::from(p1[20]))
            .filter(|&u| u >= 480)
            .ok_or_else(|| guasto("reserved space"))?;
        if p1[56..60] != [0, 0, 0, 1] && p1[56..60] != [0, 0, 0, 0] {
            return Err(guasto("text encoding"));
        }
        let mut righe = Vec::new();
        s.scorri(1, |_, v| {
            righe.push(v);
            Ok(true)
        })?;
        for v in righe {
            if v.first().and_then(Val::testo) == Some("table") {
                let (Some(nome), Some(radice), Some(sql)) = (
                    v.get(1).and_then(Val::testo),
                    v.get(3).and_then(Val::i64),
                    v.get(4).and_then(Val::testo),
                ) else {
                    continue;
                };
                let (colonne, alias_rowid, ordine) = colonne_di(sql);
                s.tabelle.push(Tabella {
                    nome: nome.to_string(),
                    radice: u32::try_from(radice).unwrap_or(0),
                    colonne,
                    alias_rowid,
                    ordine,
                });
            }
        }
        Ok(s)
    }

    /// SHA-1 of the main file of the database (not of its log).
    pub fn sha1(&mut self) -> Option<String> {
        mzlab_core::metodo::sha1_hex(&mut self.main, self.dim).ok()
    }

    pub fn tabella(&self, nome: &str) -> Option<&Tabella> {
        self.tabelle
            .iter()
            .find(|t| t.nome.eq_ignore_ascii_case(nome))
    }

    pub fn nomi_tabelle(&self) -> impl Iterator<Item = &str> {
        self.tabelle.iter().map(|t| t.nome.as_str())
    }

    /// Whether the database came with a log whose frames were applied.
    pub fn con_wal(&self) -> bool {
        self.wal.as_ref().is_some_and(|(_, m)| !m.is_empty())
    }

    fn pagina_n(&mut self, n: u32) -> Result<Vec<u8>> {
        if n == 0 || u64::from(n) > self.pagine {
            return Err(guasto("page number"));
        }
        let pagina = self.pagina;
        if let Some((w, mappa)) = &mut self.wal {
            if let Some(&off) = mappa.get(&n) {
                return leggi(&mut **w, off, pagina);
            }
        }
        if u64::from(n) > self.pagine_main {
            return Err(guasto("page beyond the file"));
        }
        leggi(&mut self.main, (u64::from(n) - 1) * pagina as u64, pagina)
    }

    /// Calls `f(rowid, values)` for each row of the table rooted at `radice`, in rowid order; `f` returns `false` to stop. Rows are visited once: the walk is iterative, bounded in depth and
    /// in pages (a page that comes twice is a cycle). The value of an `INTEGER PRIMARY KEY` column is NULL in the record: use `righe` for the tables of the schema.
    pub fn scorri<F>(&mut self, radice: u32, mut f: F) -> Result<()>
    where
        F: FnMut(i64, Vec<Val>) -> Result<bool>,
    {
        let mut pila: Vec<Passo> = vec![Passo::Pagina(radice, 0)];
        let mut viste = HashSet::new();
        while let Some(passo) = pila.pop() {
            let (n, prof) = match passo {
                Passo::Riga(v) => {
                    if !f(0, v)? {
                        return Ok(());
                    }
                    continue;
                }
                Passo::Pagina(n, prof) => (n, prof),
            };
            self.limiti.profondita_ok(prof)?;
            if !viste.insert(n) {
                return Err(guasto("page cycle"));
            }
            let p = self.pagina_n(n)?;
            let base = if n == 1 { 100 } else { 0 };
            let tipo = *p.get(base).ok_or_else(|| guasto("page"))?;
            let ncelle = u16_be(&p, base + 3).ok_or_else(|| guasto("page"))?;
            let cap = if matches!(tipo, INT_TAB | INT_IDX) {
                base + 12
            } else {
                base + 8
            };
            if cap + 2 * ncelle > self.pagina {
                return Err(guasto("cell count"));
            }
            let npag = self.pagina;
            let celle = |i: usize| -> Result<usize> {
                let c = u16_be(&p, cap + 2 * i).ok_or_else(|| guasto("cell pointer"))?;
                (c < npag).then_some(c).ok_or_else(|| guasto("cell offset"))
            };
            match tipo {
                INT_TAB => {
                    let destra = u32_be(&p, base + 8).ok_or_else(|| guasto("page"))?;
                    pila.push(Passo::Pagina(destra, prof + 1));
                    for i in (0..ncelle).rev() {
                        let c = celle(i)?;
                        pila.push(Passo::Pagina(
                            u32_be(&p, c).ok_or_else(|| guasto("cell"))?,
                            prof + 1,
                        ));
                    }
                }
                INT_IDX => {
                    // left subtree, the key of the cell, the next left subtree, ..., the right-most child
                    let destra = u32_be(&p, base + 8).ok_or_else(|| guasto("page"))?;
                    pila.push(Passo::Pagina(destra, prof + 1));
                    for i in (0..ncelle).rev() {
                        let c = celle(i)?;
                        let (_, v) = self.cella(&p, c + 4, true)?;
                        pila.push(Passo::Riga(v));
                        pila.push(Passo::Pagina(
                            u32_be(&p, c).ok_or_else(|| guasto("cell"))?,
                            prof + 1,
                        ));
                    }
                }
                FOGLIA_TAB | FOGLIA_IDX => {
                    for i in 0..ncelle {
                        let c = celle(i)?;
                        let (rowid, v) = self.cella(&p, c, tipo == FOGLIA_IDX)?;
                        if !f(rowid, v)? {
                            return Ok(());
                        }
                    }
                }
                _ => return Err(guasto("page type")),
            }
        }
        Ok(())
    }

    /// A cell: payload (with its overflow chain) decoded into values; `indice`: a cell of an index tree (no rowid, other payload limit), `c` at its payload size.
    fn cella(&mut self, p: &[u8], c: usize, indice: bool) -> Result<(i64, Vec<Val>)> {
        let (tot, a) = varint(p.get(c..).unwrap_or(&[])).ok_or_else(|| guasto("cell"))?;
        let (rowid, b) = if indice {
            (0, 0)
        } else {
            varint(p.get(c + a..).unwrap_or(&[])).ok_or_else(|| guasto("cell"))?
        };
        let inizio = c + a + b;
        let u = self.utile;
        let x = if indice {
            (u - 12) * 64 / 255 - 23
        } else {
            u - 35
        };
        if tot > self.limiti.byte {
            return Err(Error::memory_budget());
        }
        let locale = if tot <= x as u64 {
            tot as usize
        } else {
            let m = ((u - 12) * 32 / 255) - 23;
            let k = m + (tot as usize - m) % (u - 4);
            if k <= x {
                k
            } else {
                m
            }
        };
        let mut dati = p
            .get(inizio..inizio + locale)
            .ok_or_else(|| guasto("cell payload"))?
            .to_vec();
        if (locale as u64) < tot {
            let mut next = u32_be(p, inizio + locale).ok_or_else(|| guasto("overflow"))?;
            let max_pagine = tot / (u as u64 - 4) + 2;
            let mut viste = 0u64;
            while (dati.len() as u64) < tot {
                viste += 1;
                if viste > max_pagine {
                    return Err(guasto("overflow chain"));
                }
                let q = self.pagina_n(next)?;
                next = u32_be(&q, 0).ok_or_else(|| guasto("overflow"))?;
                let resta = (tot as usize - dati.len()).min(u - 4);
                dati.extend_from_slice(q.get(4..4 + resta).ok_or_else(|| guasto("overflow"))?);
            }
        }
        Ok((rowid as i64, record(&dati, &self.limiti)?))
    }

    /// All rows of table `t` with the `INTEGER PRIMARY KEY` column filled in from the rowid.
    pub fn righe<F>(&mut self, t: &Tabella, mut f: F) -> Result<()>
    where
        F: FnMut(&[Val]) -> Result<bool>,
    {
        self.scorri(t.radice, |rowid, mut v| {
            if let Some(o) = &t.ordine {
                let mut r = vec![Val::Null; t.colonne.len().max(v.len())];
                for (pos, val) in v.into_iter().enumerate() {
                    if let Some(&col) = o.get(pos) {
                        r[col] = val;
                    }
                }
                return f(&r);
            }
            if let Some(a) = t.alias_rowid {
                if matches!(v.get(a), Some(Val::Null)) {
                    v[a] = Val::Int(rowid);
                }
            }
            f(&v)
        })
    }
}

/// The values of a record: header (size, serial types) then the bodies.
pub fn record(b: &[u8], limiti: &Limiti) -> Result<Vec<Val>> {
    let (hl, mut o) = varint(b).ok_or_else(|| guasto("record header"))?;
    let hl = usize::try_from(hl).map_err(|_| guasto("record header"))?;
    if hl > b.len() || hl < o {
        return Err(guasto("record header"));
    }
    let mut tipi = Vec::new();
    while o < hl {
        let (t, n) = varint(&b[o..hl]).ok_or_else(|| guasto("serial type"))?;
        o += n;
        tipi.push(t);
        limiti.voci_ok(tipi.len() as u64)?;
    }
    let mut corpo = hl;
    let mut out = Vec::with_capacity(tipi.len());
    for t in tipi {
        let len = match t {
            0 | 8 | 9 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 4,
            5 => 6,
            6 | 7 => 8,
            10 | 11 => return Err(guasto("serial type")),
            n => ((n - 12) / 2) as usize,
        };
        let s = b
            .get(corpo..corpo.checked_add(len).ok_or_else(|| guasto("record"))?)
            .ok_or_else(|| guasto("record body"))?;
        corpo += len;
        out.push(match t {
            0 => Val::Null,
            8 => Val::Int(0),
            9 => Val::Int(1),
            1..=6 => {
                let mut v: i64 = if s[0] & 0x80 != 0 { -1 } else { 0 };
                for &x in s {
                    v = (v << 8) | i64::from(x);
                }
                Val::Int(v)
            }
            7 => Val::Real(f64::from_be_bytes(
                s.try_into().map_err(|_| guasto("real"))?,
            )),
            n if n % 2 == 0 => Val::Blob(s.to_vec()),
            _ => Val::Text(String::from_utf8_lossy(s).into_owned()),
        });
    }
    Ok(out)
}

enum Passo {
    Pagina(u32, usize),
    Riga(Vec<Val>),
}

#[cfg(test)]
mod prove {
    use super::*;

    #[test]
    fn varint_lunghi() {
        assert_eq!(varint(&[0x05]), Some((5, 1)));
        assert_eq!(varint(&[0x81, 0x00]), Some((128, 2)));
        assert_eq!(varint(&[0xff; 9]), Some((u64::MAX, 9)));
        assert_eq!(varint(&[0xff; 8]), None);
    }
}
