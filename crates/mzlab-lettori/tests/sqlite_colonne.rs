//! `Sqlite::righe_cols` against `righe`: the same values for every choice of columns, over a rowid alias, a WITHOUT ROWID table and a tree with an overflow chain; and the timing of a 10^6-row scan.
#![cfg(feature = "bruker")]

mod comune_sqlite;

use comune_sqlite::*;
use mzlab_lettori::bruker::sqlite_min::{Sqlite, Val};

fn blob_v(b: &[u8]) -> Vec<u8> {
    // serial type 12 + 2 * len: a blob
    let mut t = varint(12 + 2 * b.len() as u64);
    let mut r = vec![];
    r.append(&mut t);
    r
}

/// A record with a blob among its values (the builder above has no blob).
fn record_blob(testo: &str, blob: &[u8], n: i64) -> Vec<u8> {
    let tipi: Vec<u8> = [
        vec![0u8],
        varint(13 + 2 * testo.len() as u64),
        blob_v(blob),
        vec![6u8],
    ]
    .concat();
    let mut o = vec![(tipi.len() + 1) as u8];
    o.extend(tipi);
    o.extend(testo.as_bytes());
    o.extend(blob);
    o.extend(n.to_be_bytes());
    o
}

/// Table pages: an interior page (root), two leaves and an overflow page, the whole tree placed after the earlier tables.
fn albero(primo: u32) -> Vec<u8> {
    let grande = "x".repeat(6000 - 40);
    let mut rec = record_blob(&grande, &[1, 2, 3], -9);
    // tot = payload size, local part by the rules of the file format (usable 4096)
    let tot = rec.len() as u64;
    let (u, x) = (4096usize, 4096usize - 35);
    let m = (u - 12) * 32 / 255 - 23;
    let k = m + (tot as usize - m) % (u - 4);
    let local = if tot as usize <= x {
        tot as usize
    } else if k <= x {
        k
    } else {
        m
    };
    assert!(tot as usize > x, "the payload must overflow");
    let resto = rec.split_off(local);
    let mut cella = varint(tot);
    cella.extend(varint(5));
    cella.extend(&rec);
    cella.extend((primo + 3).to_be_bytes());
    let l1 = pagina(
        13,
        0,
        &[
            cella_foglia(1, &record_blob("uno", &[9, 9], 10)),
            cella_foglia(2, &record_blob("due", &[], 20)),
        ],
        0,
    );
    let l2 = pagina(
        13,
        0,
        &[
            cella_foglia(3, &record(&[V::N, V::T("tre"), V::N, V::I(30)])),
            cella_foglia(4, &record(&[V::N])),
            cella,
        ],
        0,
    );
    let mut cel_int = (primo + 1).to_be_bytes().to_vec();
    cel_int.extend(varint(2));
    let int = pagina(5, 0, &[cel_int], primo + 2);
    let mut ov = vec![0u8; PAG];
    ov[4..4 + resto.len()].copy_from_slice(&resto);
    [int, l1, l2, ov].concat()
}

/// Tables `A` (rowid alias), `W` (WITHOUT ROWID) and `T` (two leaves, an overflow chain).
fn tre_tabelle() -> Sqlite {
    let a = foglia(&[
        vec![V::N, V::T("a"), V::R(1.5), V::I(7)],
        vec![V::I(40), V::T("bé"), V::N],
        vec![V::N, V::T("corta")],
    ]);
    // WITHOUT ROWID: the record is (b, a)
    let celle: Vec<Vec<u8>> = [("x", 1), ("y", 2)]
        .iter()
        .map(|(b, a)| {
            let r = record(&[V::T(b), V::I(*a)]);
            let mut c = varint(r.len() as u64);
            c.extend(r);
            c
        })
        .collect();
    let w = pagina(10, 0, &celle, 0);
    apri_db(
        db(&[
            (
                "A",
                "CREATE TABLE A (Id INTEGER PRIMARY KEY, Nome TEXT, X REAL, Y)",
                a,
            ),
            (
                "W",
                "CREATE TABLE W (a INTEGER, b TEXT, PRIMARY KEY (b)) WITHOUT ROWID",
                w,
            ),
            (
                "T",
                "CREATE TABLE T (Id INTEGER PRIMARY KEY, Testo TEXT, Blob BLOB, N INTEGER)",
                albero(4),
            ),
        ]),
        None,
    )
    .unwrap()
}

/// `righe_cols` gives, for every choice of columns, what `righe` gives for them.
fn confronta(s: &mut Sqlite, nome: &str) {
    let t = s.tabella(nome).unwrap().clone();
    let mut tutte_le_righe: Vec<Vec<Val>> = Vec::new();
    s.righe(&t, |r| {
        tutte_le_righe.push(r.to_vec());
        Ok(true)
    })
    .unwrap();
    assert!(tutte_le_righe.len() >= 2, "{nome}");
    let n = t.colonne.len();
    // every subset of the columns in table order, in reverse order, and with a missing one and a repeated one
    let mut scelte: Vec<Vec<Option<usize>>> = Vec::new();
    for m in 0u32..(1 << n) {
        let v: Vec<Option<usize>> = (0..n).filter(|i| m >> i & 1 == 1).map(Some).collect();
        scelte.push(v.iter().rev().copied().collect());
        scelte.push(v);
    }
    scelte.push(vec![Some(0), None, Some(0), Some(n - 1), Some(n + 3)]);
    for idx in scelte {
        let attese: Vec<Vec<Val>> = tutte_le_righe
            .iter()
            .map(|r| {
                idx.iter()
                    .map(|i| i.and_then(|i| r.get(i).cloned()).unwrap_or(Val::Null))
                    .collect()
            })
            .collect();
        let mut avute: Vec<Vec<Val>> = Vec::new();
        s.righe_cols(&t, &idx, |r| {
            avute.push(r.to_vec());
            Ok(true)
        })
        .unwrap();
        assert_eq!(avute, attese, "{nome} {idx:?}");
    }
}

#[test]
fn righe_cols_da_gli_stessi_valori_di_righe() {
    let mut s = tre_tabelle();
    for nome in ["A", "W", "T"] {
        confronta(&mut s, nome);
    }
    // the tree really was walked: interior page, both leaves and the overflow chain
    let t = s.tabella("T").unwrap().clone();
    let mut visti = Vec::new();
    s.righe_cols(&t, &[Some(0), Some(1), Some(3)], |r| {
        visti.push((r[0].i64().unwrap(), r[1].testo().map(str::len), r[2].i64()));
        Ok(true)
    })
    .unwrap();
    assert_eq!(
        visti,
        vec![
            (1, Some(3), Some(10)),
            (2, Some(3), Some(20)),
            (3, Some(3), Some(30)),
            (4, None, None),
            (5, Some(5960), Some(-9))
        ]
    );
}

/// Timing of a full scan of a 10^6-row table: all columns (`righe`) against the two that are needed.
/// The pages of a two-level table of `n_rows` rows: the root (page 2), then the middle pages, then the leaves.
fn grande(n_rows: usize) -> (Vec<u8>, Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut leaves: Vec<Vec<u8>> = Vec::new();
    let mut maxes: Vec<u64> = Vec::new();
    let mut cells: Vec<Vec<u8>> = Vec::new();
    let mut used = 8usize;
    let name = "sample-name-of-thirty-chars-xx";
    for i in 1..=n_rows as u64 {
        let rec = record(&[
            V::N,
            V::I(i as i64 * 7),
            V::R(i as f64 * 0.5),
            V::T(name),
            V::I(3),
        ]);
        let c = cella_foglia(i, &rec);
        if used + c.len() + 2 > PAG {
            leaves.push(pagina(13, 0, &cells, 0));
            maxes.push(i - 1);
            cells.clear();
            used = 8;
        }
        used += c.len() + 2;
        cells.push(c);
    }
    leaves.push(pagina(13, 0, &cells, 0));
    maxes.push(n_rows as u64);
    let per_mid = 400usize;
    let n_mid = leaves.len().div_ceil(per_mid);
    let first_leaf = 3 + n_mid as u32;
    let mut mids = Vec::new();
    let mut root_cells = Vec::new();
    for m in 0..n_mid {
        let lo = m * per_mid;
        let hi = ((m + 1) * per_mid).min(leaves.len());
        let mut cs = Vec::new();
        for k in lo..hi - 1 {
            let mut c = (first_leaf + k as u32).to_be_bytes().to_vec();
            c.extend(varint(maxes[k]));
            cs.push(c);
        }
        mids.push(pagina(5, 0, &cs, first_leaf + hi as u32 - 1));
        if m + 1 < n_mid {
            let mut c = (3 + m as u32).to_be_bytes().to_vec();
            c.extend(varint(maxes[hi - 1]));
            root_cells.push(c);
        }
    }
    let root = pagina(5, 0, &root_cells, 3 + n_mid as u32 - 1);
    (root, mids, leaves)
}

#[test]
#[ignore]
fn timing_of_a_million_row_scan() {
    let n_rows = 1_000_000usize;
    let (root, mids, leaves) = grande(n_rows);
    let sql = "CREATE TABLE B (Id INTEGER PRIMARY KEY, A INTEGER, X REAL, Name TEXT, Z INTEGER)";
    let mut img = db(&[("B", sql, root)]);
    for p in mids.iter().chain(leaves.iter()) {
        img.extend(p);
    }
    let mut s = apri_db(img.clone(), None).unwrap();
    let t = s.tabella("B").unwrap().clone();
    for round in 0..3 {
        let t0 = std::time::Instant::now();
        let mut sum = 0i64;
        let mut cnt = 0usize;
        s.righe(&t, |r| {
            sum += r[1].i64().unwrap();
            cnt += r[2].f64().is_some() as usize;
            Ok(true)
        })
        .unwrap();
        let old = t0.elapsed();
        let t1 = std::time::Instant::now();
        let mut sum2 = 0i64;
        let mut cnt2 = 0usize;
        s.righe_cols(&t, &[Some(1), Some(2)], |r| {
            sum2 += r[0].i64().unwrap();
            cnt2 += r[1].f64().is_some() as usize;
            Ok(true)
        })
        .unwrap();
        let new = t1.elapsed();
        assert_eq!((sum, cnt), (sum2, cnt2));
        assert_eq!(cnt, n_rows);
        eprintln!(
            "round {round}: righe {old:?}  righe_cols {new:?}  ({} pages)",
            img.len() / PAG
        );
    }
}
