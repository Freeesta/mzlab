//! A SQLite file built by hand (pages, records, a b-tree) for the tests of the Bruker reader, with no real data.
#![allow(dead_code)]

use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limits;
use mzlab_lettori::bruker::sqlite_min::Sqlite;

pub mod pasef;

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
    let (mut kinds, mut corpo) = (Vec::new(), Vec::new());
    for v in vals {
        match v {
            V::N => kinds.push(0),
            V::I(x) => {
                kinds.push(6);
                corpo.extend(x.to_be_bytes());
            }
            V::R(x) => {
                kinds.push(7);
                corpo.extend(x.to_be_bytes());
            }
            V::T(s) => {
                kinds.extend(varint(13 + 2 * s.len() as u64));
                corpo.extend(s.as_bytes());
            }
        }
    }
    let mut o = vec![(kinds.len() + 1) as u8];
    o.extend(kinds);
    o.extend(corpo);
    o
}

/// A b-tree page of `tipo` (13 leaf table, 10 leaf index, 5 interior table) with its `celle`, the header at `base`.
pub fn page(tipo: u8, base: usize, cell_list: &[Vec<u8>], destra: u32) -> Vec<u8> {
    let mut p = vec![0u8; PAG];
    let mut fine = PAG;
    let mut ptr = Vec::new();
    for c in cell_list {
        fine -= c.len();
        p[fine..fine + c.len()].copy_from_slice(c);
        ptr.push(fine as u16);
    }
    let intest = if tipo == 5 { 12 } else { 8 };
    p[base] = tipo;
    p[base + 3..base + 5].copy_from_slice(&(cell_list.len() as u16).to_be_bytes());
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

pub fn leaf_cell(rowid: u64, rec: &[u8]) -> Vec<u8> {
    let mut c = varint(rec.len() as u64);
    c.extend(varint(rowid));
    c.extend(rec);
    c
}

pub fn foglia(rows: &[Vec<V<'_>>]) -> Vec<u8> {
    let c: Vec<Vec<u8>> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| leaf_cell(i as u64 + 1, &record(r)))
        .collect();
    page(13, 0, &c, 0)
}

/// A database whose tables are `(name, CREATE text, page image)`; table `i` is rooted at page `i + 2`.
pub fn db(tabelle: &[(&str, &str, Vec<u8>)]) -> Vec<u8> {
    let schema: Vec<Vec<u8>> = tabelle
        .iter()
        .enumerate()
        .map(|(i, (n, sql, _))| {
            leaf_cell(
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
    let mut p1 = page(13, 100, &schema, 0);
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

pub fn limits() -> Limits {
    Limits::sito()
}

pub fn apri_db(b: Vec<u8>, wal: Option<Vec<u8>>) -> mzlab_core::error::Result<Sqlite> {
    let dim = b.len() as u64;
    let w = wal.map(|w| {
        let n = w.len() as u64;
        (Box::new(w) as Box<dyn ReadAt>, n)
    });
    Sqlite::apri(Box::new(b), dim, w, &limits())
}

// ---- a whole `.d` folder ----

pub fn raw_zstd(d: &[u8]) -> Vec<u8> {
    let mut z = vec![0x28, 0xb5, 0x2f, 0xfd, 0xa0];
    z.extend((d.len() as u32).to_le_bytes());
    z.extend((((d.len() as u32) << 3) | 1).to_le_bytes()[..3].iter());
    z.extend(d);
    z
}

/// A codec-2 block of 2 scans: scan 0 has TOF 100 and 150, scan 1 has TOF 100.
pub fn blocco2() -> Vec<u8> {
    let words: [u32; 8] = [0, 4, 101, 10, 50, 20, 101, 5];
    let mut piani = Vec::new();
    for k in 0..4 {
        piani.extend(words.iter().map(|w| (w >> (8 * k)) as u8));
    }
    let z = raw_zstd(&piani);
    let mut b = ((z.len() + 8) as u32).to_le_bytes().to_vec();
    b.extend(2u32.to_le_bytes());
    b.extend(z);
    b
}

pub fn minimal_tdf(num_peaks: i64) -> Vec<u8> {
    tdf_frames(&[0], num_peaks)
}

/// A database of one MS1 frame per offset in `tims` (`TimsId`), each `num_peaks` peaks over 2 scans, 1 s apart.
pub fn tdf_frames(tims: &[i64], num_peaks: i64) -> Vec<u8> {
    let rows: Vec<Vec<V<'_>>> = tims
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            vec![
                V::N,
                V::R(1.0 + i as f64),
                V::T("+"),
                V::I(0),
                V::I(0),
                V::I(t),
                V::I(100),
                V::I(50),
                V::I(2),
                V::I(num_peaks),
                V::I(1),
                V::R(25.0),
                V::R(30.0),
                V::R(100.0),
            ]
        })
        .collect();
    let frames = foglia(&rows);
    let cal = foglia(&[vec![
        V::N,
        V::I(1),
        V::R(0.125),
        V::R(18502.75),
        V::R(280.6435408329972),
        V::R(150581.44261048065),
        V::R(0.0),
        V::R(0.0),
        V::R(0.0),
        V::R(25.0),
        V::R(30.0),
        V::R(0.0),
        V::R(0.0),
    ]]);
    db(&[
        ("Frames", "CREATE TABLE Frames (Id INTEGER PRIMARY KEY, Time REAL, Polarity TEXT, ScanMode INTEGER, MsMsType INTEGER, TimsId INTEGER, SummedIntensities INTEGER, MaxIntensity INTEGER, NumScans INTEGER, NumPeaks INTEGER, MzCalibration INTEGER, T1 REAL, T2 REAL, AccumulationTime REAL)", frames),
        ("MzCalibration", "CREATE TABLE MzCalibration (Id INTEGER PRIMARY KEY, ModelType INTEGER, DigitizerTimebase REAL, DigitizerDelay REAL, C0 REAL, C1 REAL, C2 REAL, C3 REAL, C4 REAL, T1 REAL, T2 REAL, dC1 REAL, dC2 REAL)", cal),
    ])
}

/// A database of tables of any size: `(name, CREATE text, rows)`, every table its leaves of `rows` and, past one leaf, an interior root (at most one level: a few thousand rows).
pub fn db_rows(tabelle: &[(&str, &str, Vec<Vec<V<'_>>>)]) -> Vec<u8> {
    let mut pagine: Vec<Vec<u8>> = Vec::new();
    let mut radici = Vec::new();
    for (_, _, rows) in tabelle {
        let cell_list: Vec<Vec<u8>> = rows
            .iter()
            .enumerate()
            .map(|(i, r)| leaf_cell(i as u64 + 1, &record(r)))
            .collect();
        let mut foglie: Vec<(Vec<Vec<u8>>, u64)> = vec![(Vec::new(), 0)];
        let mut used = 8;
        for (i, c) in cell_list.into_iter().enumerate() {
            if used + c.len() + 2 > PAG && !foglie.last().unwrap().0.is_empty() {
                foglie.push((Vec::new(), 0));
                used = 8;
            }
            used += c.len() + 2;
            let f = foglie.last_mut().unwrap();
            f.0.push(c);
            f.1 = i as u64 + 1;
        }
        // pages are numbered from 2 (page 1 is the schema)
        let first_one = pagine.len() as u32 + 2;
        for (c, _) in &foglie {
            pagine.push(page(13, 0, c, 0));
        }
        if foglie.len() == 1 {
            radici.push(first_one);
            continue;
        }
        let n = foglie.len() as u32;
        let interne: Vec<Vec<u8>> = foglie[..foglie.len() - 1]
            .iter()
            .enumerate()
            .map(|(j, (_, max))| {
                let mut c = (first_one + j as u32).to_be_bytes().to_vec();
                c.extend(varint(*max));
                c
            })
            .collect();
        assert!(
            interne.len() * 15 + 12 < PAG,
            "table too large for one interior page"
        );
        radici.push(pagine.len() as u32 + 2);
        pagine.push(page(5, 0, &interne, first_one + n - 1));
    }
    let schema: Vec<Vec<u8>> = tabelle
        .iter()
        .zip(&radici)
        .enumerate()
        .map(|(i, ((n, sql, _), &r))| {
            leaf_cell(
                i as u64 + 1,
                &record(&[
                    V::T("table"),
                    V::T(n),
                    V::T(n),
                    V::I(i64::from(r)),
                    V::T(sql),
                ]),
            )
        })
        .collect();
    let mut p1 = page(13, 100, &schema, 0);
    p1[..16].copy_from_slice(b"SQLite format 3\0");
    p1[16..18].copy_from_slice(&(PAG as u16).to_be_bytes());
    p1[18] = 1;
    p1[19] = 1;
    p1[21..24].copy_from_slice(&[64, 32, 32]);
    p1[28..32].copy_from_slice(&(pagine.len() as u32 + 1).to_be_bytes());
    p1[44..48].copy_from_slice(&4u32.to_be_bytes());
    p1[56..60].copy_from_slice(&1u32.to_be_bytes());
    let mut out = p1;
    for p in pagine {
        out.extend(p);
    }
    out
}
