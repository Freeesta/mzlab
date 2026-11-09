//! A read-only SQLite 3 reader without an SQL engine, for the `analysis.tdf` / `analysis.tsf` of a Bruker `.d` folder: the 100-byte header, the page size, the b-tree of a table (interior pages and
//! leaves), varints, the record format, overflow pages and `sqlite_schema` to find a table by name. Written from the public file-format description (https://www.sqlite.org/fileformat2.html).
//! Every page pointer, length and count read from the file passes through `Limiti`; a page is visited once (a cycle is an error), the depth of the b-tree is bounded and so are the
//! bytes of an overflow chain. A write-ahead log (`analysis.tdf-wal`, left by an acquisition that did not close its database) is read too: the latest committed frame of each page wins.

use super::sqlite_schema::columns_of;

use super::sqlite_wal::read_wal;

use mzlab_core::error::{Error, Result};

use mzlab_core::idxread::ReadAt;

use mzlab_core::readers::Limits;

use std::collections::{HashMap, HashSet};

mod cols;
use cols::record_in;

const MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// The pages of a b-tree interior or leaf, of a table.
const INT_TAB: u8 = 5;

const LEAF_TAB: u8 = 13;

/// The pages of the b-tree of a `WITHOUT ROWID` table (an index tree whose keys are the rows).
const INT_IDX: u8 = 2;

const LEAF_IDX: u8 = 10;

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
    pub fn text(&self) -> Option<&str> {
        match self {
            Val::Text(t) => Some(t),
            _ => None,
        }
    }
    /// The value as the text a person reads (numbers as they print; `None` for NULL and blobs).
    pub fn as_text(&self) -> Option<String> {
        match self {
            Val::Text(t) => Some(t.clone()),
            Val::Int(i) => Some(i.to_string()),
            Val::Real(r) => Some(r.to_string()),
            _ => None,
        }
    }
}

impl Table {
    pub fn col(&self, name: &str) -> Option<usize> {
        self.colonne
            .iter()
            .position(|c| c.eq_ignore_ascii_case(name))
    }
}

impl Sqlite {
    /// Opens the database `main` of `dim` bytes (with its write-ahead log `wal`, when the folder has one) and reads its `sqlite_schema`.
    pub fn open(
        main: Box<dyn ReadAt>,
        dim: u64,
        wal: Option<(Box<dyn ReadAt>, u64)>,
        limits: &Limits,
    ) -> Result<Sqlite> {
        let mut s = Sqlite {
            main,
            dim,
            main_pages: 0,
            wal: None,
            page: 0,
            useful: 0,
            pagine: 0,
            tables: Vec::new(),
            limits: *limits,
        };
        let mut h = [0u8; 100];
        let read_items = (dim as usize).min(100);
        s.main
            .read_at(0, &mut h[..read_items])
            .map_err(|_| broken("short read"))?;
        let mut commit = None;
        if let Some((mut w, wdim)) = wal
            && let Some((page, mappa, pages)) = read_wal(&mut *w, wdim, limits)?
        {
            s.page = page;
            commit = Some(pages);
            s.wal = Some((w, mappa));
        }
        if s.page == 0 {
            let raw = u16_be(&h, 16).ok_or_else(|| broken("header"))?;
            s.page = if raw == 1 { 65536 } else { raw };
        }
        if !(512..=65536).contains(&s.page) || !s.page.is_power_of_two() {
            return Err(broken("page size"));
        }
        s.main_pages = dim / s.page as u64;
        s.pagine = commit.filter(|&n| n > 0).unwrap_or(s.main_pages);
        limits.ok_entries(s.pagine)?;
        // page 1 may live in the log only: its first 100 bytes are the header of the database
        let p1 = s.page_n(1)?;
        if p1.len() < 100 || &p1[..16] != MAGIC {
            return Err(broken("not a database"));
        }
        s.useful = s
            .page
            .checked_sub(usize::from(p1[20]))
            .filter(|&u| u >= 480)
            .ok_or_else(|| broken("reserved space"))?;
        if p1[56..60] != [0, 0, 0, 1] && p1[56..60] != [0, 0, 0, 0] {
            return Err(broken("text encoding"));
        }
        s.read_schema()?;
        Ok(s)
    }

    /// The tables of `sqlite_schema` (page 1): name, root page, columns.
    fn read_schema(&mut self) -> Result<()> {
        let mut rows = Vec::new();
        self.walk(1, |_, v| {
            rows.push(v);
            Ok(true)
        })?;
        for v in rows {
            if v.first().and_then(Val::text) == Some("table") {
                let (Some(name), Some(root), Some(sql)) = (
                    v.get(1).and_then(Val::text),
                    v.get(3).and_then(Val::i64),
                    v.get(4).and_then(Val::text),
                ) else {
                    continue;
                };
                let (columns, alias_rowid, order) = columns_of(sql);
                self.tables.push(Table {
                    nome: name.to_string(),
                    radice: u32::try_from(root).unwrap_or(0),
                    colonne: columns,
                    alias_rowid,
                    ordine: order,
                });
            }
        }
        Ok(())
    }

    /// SHA-1 of the main file of the database (not of its log).
    pub fn sha1(&mut self) -> Option<String> {
        mzlab_core::method::sha1_hex(&mut self.main, self.dim).ok()
    }

    pub fn table(&self, name: &str) -> Option<&Table> {
        self.tables
            .iter()
            .find(|t| t.nome.eq_ignore_ascii_case(name))
    }

    /// Whether the database came with a log whose frames were applied.
    pub fn with_wal(&self) -> bool {
        self.wal.as_ref().is_some_and(|(_, m)| !m.is_empty())
    }

    fn page_n(&mut self, n: u32) -> Result<Vec<u8>> {
        let mut p = Vec::new();
        self.page_in(n, &mut p)?;
        Ok(p)
    }

    /// Page `n` into `buf` (its capacity is kept from one page to the next); the log wins over the main file.
    fn page_in(&mut self, n: u32, buf: &mut Vec<u8>) -> Result<()> {
        if n == 0 || u64::from(n) > self.pagine {
            return Err(broken("page number"));
        }
        let page = self.page;
        if let Some((w, mappa)) = &mut self.wal
            && let Some(&off) = mappa.get(&n)
        {
            return read_into(&mut **w, off, page, buf);
        }
        if u64::from(n) > self.main_pages {
            return Err(broken("page beyond the file"));
        }
        read_into(&mut self.main, (u64::from(n) - 1) * page as u64, page, buf)
    }

    /// Calls `f(rowid, values)` for each row of the table rooted at `radice`, in rowid order; `f` returns `false` to stop. Rows are visited once: the walk is iterative, bounded in depth and
    /// in pages (a page that comes twice is a cycle). The value of an `INTEGER PRIMARY KEY` column is NULL in the record: use `righe` for the tables of the schema.
    pub fn walk<F>(&mut self, root: u32, mut f: F) -> Result<()>
    where
        F: FnMut(i64, Vec<Val>) -> Result<bool>,
    {
        self.walk_with(root, None, |rowid, v| f(rowid, std::mem::take(v)))
    }

    /// The header of page `n` held in `p`: where it starts, its type, its number of cells and where the cell pointers begin.
    fn page_header(&self, p: &[u8], n: u32) -> Result<(usize, u8, usize, usize)> {
        let base = if n == 1 { 100 } else { 0 };
        let kind = *p.get(base).ok_or_else(|| broken("page"))?;
        let ncells = u16_be(p, base + 3).ok_or_else(|| broken("page"))?;
        let cap = base
            + if matches!(kind, INT_TAB | INT_IDX) {
                12
            } else {
                8
            };
        if cap + 2 * ncells > self.page {
            return Err(broken("cell count"));
        }
        Ok((base, kind, ncells, cap))
    }

    /// `scorri` that decodes only the record positions `voglio` marks `true` (the others are `Val::Null`; `None` = all) and hands `f` one vector reused from row to row, which `f` may take from.
    pub fn walk_with<F>(&mut self, root: u32, want: Option<&[bool]>, mut f: F) -> Result<()>
    where
        F: FnMut(i64, &mut Vec<Val>) -> Result<bool>,
    {
        let mut stack: Vec<Step> = vec![Step::Page(root, 0)];
        let mut views = HashSet::new();
        let (mut p, mut q, mut row) = (Vec::new(), Vec::new(), Vec::new());
        while let Some(step) = stack.pop() {
            let (n, prof) = match step {
                Step::Row(mut v) => {
                    if !f(0, &mut v)? {
                        return Ok(());
                    }
                    continue;
                }
                Step::Page(n, prof) => (n, prof),
            };
            self.limits.profondita_ok(prof)?;
            if !views.insert(n) {
                return Err(broken("page cycle"));
            }
            self.page_in(n, &mut p)?;
            let (base, kind, ncells, cap) = self.page_header(&p, n)?;
            let npag = self.page;
            let cell_list = |i: usize| -> Result<usize> {
                let c = u16_be(&p, cap + 2 * i).ok_or_else(|| broken("cell pointer"))?;
                (c < npag).then_some(c).ok_or_else(|| broken("cell offset"))
            };
            match kind {
                INT_TAB => {
                    let right = u32_be(&p, base + 8).ok_or_else(|| broken("page"))?;
                    stack.push(Step::Page(right, prof + 1));
                    for i in (0..ncells).rev() {
                        let c = cell_list(i)?;
                        stack.push(Step::Page(
                            u32_be(&p, c).ok_or_else(|| broken("cell"))?,
                            prof + 1,
                        ));
                    }
                }
                INT_IDX => {
                    // left subtree, the key of the cell, the next left subtree, ..., the right-most child
                    let destra = u32_be(&p, base + 8).ok_or_else(|| broken("page"))?;
                    stack.push(Step::Page(destra, prof + 1));
                    for i in (0..ncells).rev() {
                        let c = cell_list(i)?;
                        let mut v = Vec::new();
                        self.cell(&p, c + 4, true, want, &mut q, &mut v)?;
                        stack.push(Step::Row(v));
                        stack.push(Step::Page(
                            u32_be(&p, c).ok_or_else(|| broken("cell"))?,
                            prof + 1,
                        ));
                    }
                }
                LEAF_TAB | LEAF_IDX => {
                    for i in 0..ncells {
                        let c = cell_list(i)?;
                        let rowid = self.cell(&p, c, kind == LEAF_IDX, want, &mut q, &mut row)?;
                        if !f(rowid, &mut row)? {
                            return Ok(());
                        }
                    }
                }
                _ => return Err(broken("page type")),
            }
        }
        Ok(())
    }

    /// A cell: payload (with its overflow chain) decoded into values; `indice`: a cell of an index tree (no rowid, other payload limit), `c` at its payload size.
    /// The values go to `out` (`voglio` as in `scorri_con`), the rowid is returned; `q` is the page buffer of the overflow chain.
    fn cell(
        &mut self,
        p: &[u8],
        c: usize,
        index: bool,
        want: Option<&[bool]>,
        q: &mut Vec<u8>,
        out: &mut Vec<Val>,
    ) -> Result<i64> {
        let (tot, a) = varint(p.get(c..).unwrap_or(&[])).ok_or_else(|| broken("cell"))?;
        let (rowid, b) = if index {
            (0, 0)
        } else {
            varint(p.get(c + a..).unwrap_or(&[])).ok_or_else(|| broken("cell"))?
        };
        let start = c + a + b;
        let u = self.useful;
        let x = if index {
            (u - 12) * 64 / 255 - 23
        } else {
            u - 35
        };
        if tot > self.limits.byte {
            return Err(Error::memory_budget());
        }
        let local = if tot <= x as u64 {
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
        let local_p = p
            .get(start..start + local)
            .ok_or_else(|| broken("cell payload"))?;
        if (local as u64) >= tot {
            // no overflow: the record is read where it lies in the page
            record_in(local_p, &self.limits, want, out)?;
            return Ok(rowid as i64);
        }
        let mut data = local_p.to_vec();
        let mut next = u32_be(p, start + local).ok_or_else(|| broken("overflow"))?;
        let max_pages = tot / (u as u64 - 4) + 2;
        let mut views = 0u64;
        while (data.len() as u64) < tot {
            views += 1;
            if views > max_pages {
                return Err(broken("overflow chain"));
            }
            self.page_in(next, q)?;
            next = u32_be(q, 0).ok_or_else(|| broken("overflow"))?;
            let remains = (tot as usize - data.len()).min(u - 4);
            data.extend_from_slice(q.get(4..4 + remains).ok_or_else(|| broken("overflow"))?);
        }
        record_in(&data, &self.limits, want, out)?;
        Ok(rowid as i64)
    }

    /// All rows of table `t` with the `INTEGER PRIMARY KEY` column filled in from the rowid.
    pub fn rows<F>(&mut self, t: &Table, mut f: F) -> Result<()>
    where
        F: FnMut(&[Val]) -> Result<bool>,
    {
        self.walk(t.radice, |rowid, mut v| {
            if let Some(o) = &t.ordine {
                let mut r = vec![Val::Null; t.colonne.len().max(v.len())];
                for (pos, val) in v.into_iter().enumerate() {
                    if let Some(&col) = o.get(pos) {
                        r[col] = val;
                    }
                }
                return f(&r);
            }
            if let Some(a) = t.alias_rowid
                && matches!(v.get(a), Some(Val::Null))
            {
                v[a] = Val::Int(rowid);
            }
            f(&v)
        })
    }
}

pub(super) fn broken(m: &str) -> Error {
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

/// A table of the file: its root page, the names of its columns (from the `CREATE TABLE` text) and which column is the alias of the rowid.
#[derive(Debug, Clone)]
pub struct Table {
    pub nome: String,
    pub radice: u32,
    pub colonne: Vec<String>,
    /// Index of the `INTEGER PRIMARY KEY` column, whose record value is NULL (the rowid stands for it).
    pub alias_rowid: Option<usize>,
    /// `WITHOUT ROWID` table: the column stored at each position of the record (the primary key comes first).
    pub ordine: Option<Vec<usize>>,
}

pub struct Sqlite {
    main: Box<dyn ReadAt>,
    dim: u64,
    /// Pages of the main file.
    main_pages: u64,
    /// The write-ahead log: its bytes and, for each page, the offset of its latest committed frame.
    wal: Option<(Box<dyn ReadAt>, HashMap<u32, u64>)>,
    page: usize,
    /// Bytes of a page usable by cells (page size minus the reserved space).
    useful: usize,
    pagine: u64,
    tables: Vec<Table>,
    limits: Limits,
}

/// Reads `n` bytes at `off` (an error when the file is shorter).
pub(super) fn read(d: &mut dyn ReadAt, off: u64, n: usize) -> Result<Vec<u8>> {
    let mut b = Vec::new();
    read_into(d, off, n, &mut b)?;
    Ok(b)
}

/// `leggi` into `b`, which keeps its capacity.
fn read_into(d: &mut dyn ReadAt, off: u64, n: usize, b: &mut Vec<u8>) -> Result<()> {
    b.clear();
    b.resize(n, 0);
    d.read_at(off, b).map_err(|_| broken("short read"))
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

/// The values of a record: header (size, serial types) then the bodies.
pub fn record(b: &[u8], limits: &Limits) -> Result<Vec<Val>> {
    let mut out = Vec::new();
    record_in(b, limits, None, &mut out)?;
    Ok(out)
}

enum Step {
    Page(u32, usize),
    Row(Vec<Val>),
}

#[cfg(test)]
mod prove {
    use super::*;

    #[test]
    fn long_varints() {
        assert_eq!(varint(&[0x05]), Some((5, 1)));
        assert_eq!(varint(&[0x81, 0x00]), Some((128, 2)));
        assert_eq!(varint(&[0xff; 9]), Some((u64::MAX, 9)));
        assert_eq!(varint(&[0xff; 8]), None);
    }
}
