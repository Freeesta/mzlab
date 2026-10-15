//! Hostile and hand-built input for the Bruker reader: a SQLite file and a `tdf_bin` made inside the test (no real data), page cycles, long varints, fake zstd, truncations, a write-ahead log.
#![cfg(feature = "bruker")]

use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_lettori::bruker::sqlite_min::Sqlite;

const PAG: usize = 4096;

enum V<'a> {
    N,
    I(i64),
    R(f64),
    T(&'a str),
}

fn varint(mut v: u64) -> Vec<u8> {
    let mut o = vec![(v & 0x7f) as u8];
    v >>= 7;
    while v > 0 {
        o.push((v & 0x7f) as u8 | 0x80);
        v >>= 7;
    }
    o.reverse();
    o
}

fn record(vals: &[V<'_>]) -> Vec<u8> {
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
fn pagina(tipo: u8, base: usize, celle: &[Vec<u8>], destra: u32) -> Vec<u8> {
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

fn cella_foglia(rowid: u64, rec: &[u8]) -> Vec<u8> {
    let mut c = varint(rec.len() as u64);
    c.extend(varint(rowid));
    c.extend(rec);
    c
}

fn foglia(righe: &[Vec<V<'_>>]) -> Vec<u8> {
    let c: Vec<Vec<u8>> = righe
        .iter()
        .enumerate()
        .map(|(i, r)| cella_foglia(i as u64 + 1, &record(r)))
        .collect();
    pagina(13, 0, &c, 0)
}

/// A database whose tables are `(name, CREATE text, page image)`; table `i` is rooted at page `i + 2`.
fn db(tabelle: &[(&str, &str, Vec<u8>)]) -> Vec<u8> {
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

fn limiti() -> Limiti {
    Limiti::sito()
}

fn apri_db(b: Vec<u8>, wal: Option<Vec<u8>>) -> mzlab_core::error::Result<Sqlite> {
    let dim = b.len() as u64;
    let w = wal.map(|w| {
        let n = w.len() as u64;
        (Box::new(w) as Box<dyn ReadAt>, n)
    });
    Sqlite::apri(Box::new(b), dim, w, &limiti())
}

fn tutte(s: &mut Sqlite, nome: &str) -> mzlab_core::error::Result<Vec<Vec<String>>> {
    let t = s.tabella(nome).unwrap().clone();
    let mut out = Vec::new();
    s.righe(&t, |r| {
        out.push(
            r.iter()
                .map(|v| v.come_testo().unwrap_or_default())
                .collect(),
        );
        Ok(true)
    })?;
    Ok(out)
}

#[test]
fn sqlite_fatto_a_mano_si_legge() {
    let t = foglia(&[
        vec![V::N, V::T("a"), V::R(1.5)],
        vec![V::N, V::T("b"), V::I(7)],
    ]);
    let mut s = apri_db(
        db(&[(
            "T",
            "CREATE TABLE T (Id INTEGER PRIMARY KEY, Nome TEXT, X REAL)",
            t,
        )]),
        None,
    )
    .unwrap();
    let r = tutte(&mut s, "T").unwrap();
    assert_eq!(r.len(), 2);
    assert_eq!(r[0][0], "1");
    assert_eq!(r[1][1], "b");
    assert!(!s.con_wal());
}

#[test]
fn senza_rowid_mette_la_chiave_per_prima() {
    // WITHOUT ROWID table T(a, b, PRIMARY KEY(b)): the record is (b, a)
    let celle: Vec<Vec<u8>> = [("x", 1), ("y", 2)]
        .iter()
        .map(|(b, a)| {
            let r = record(&[V::T(b), V::I(*a)]);
            let mut c = varint(r.len() as u64);
            c.extend(r);
            c
        })
        .collect();
    let p = pagina(10, 0, &celle, 0);
    let mut s = apri_db(
        db(&[(
            "T",
            "CREATE TABLE T (a INTEGER, b TEXT, PRIMARY KEY (b)) WITHOUT ROWID",
            p,
        )]),
        None,
    )
    .unwrap();
    let r = tutte(&mut s, "T").unwrap();
    assert_eq!(
        r,
        vec![
            vec!["1".to_string(), "x".into()],
            vec!["2".into(), "y".into()]
        ]
    );
}

#[test]
fn pagina_che_rimanda_a_se_stessa_non_gira_in_eterno() {
    // an interior page whose only child is itself
    let c = {
        let mut c = 2u32.to_be_bytes().to_vec();
        c.extend(varint(5));
        c
    };
    let p = pagina(5, 0, &[c], 2);
    let mut s = apri_db(
        db(&[("T", "CREATE TABLE T (Id INTEGER PRIMARY KEY)", p)]),
        None,
    )
    .unwrap();
    assert!(tutte(&mut s, "T").is_err());
}

#[test]
fn varint_lunghi_e_record_guasti_sono_errori() {
    // header length claims a 10-byte varint
    let mut rec = vec![0xff; 10];
    rec.extend([1, 2, 3]);
    let p = pagina(13, 0, &[cella_foglia(1, &rec)], 0);
    let mut s = apri_db(
        db(&[("T", "CREATE TABLE T (Id INTEGER PRIMARY KEY, X)", p)]),
        None,
    )
    .unwrap();
    assert!(tutte(&mut s, "T").is_err());
    // a cell pointer outside the page
    let mut p = foglia(&[vec![V::N, V::I(1)]]);
    p[8..10].copy_from_slice(&0xfff0u16.to_be_bytes());
    p[10..12].copy_from_slice(&0xffffu16.to_be_bytes());
    let mut s = apri_db(
        db(&[("T", "CREATE TABLE T (Id INTEGER PRIMARY KEY, X)", p)]),
        None,
    )
    .unwrap();
    let _ = tutte(&mut s, "T");
}

#[test]
fn ogni_troncamento_e_un_errore_o_un_risultato() {
    let t = foglia(&[vec![V::N, V::T("a")]]);
    let b = db(&[("T", "CREATE TABLE T (Id INTEGER PRIMARY KEY, X TEXT)", t)]);
    for n in (0..b.len()).step_by(37) {
        if let Ok(mut s) = apri_db(b[..n].to_vec(), None) {
            if s.tabella("T").is_some() {
                let _ = tutte(&mut s, "T");
            }
        }
    }
}

fn somma(b: &[u8], mut s0: u32, mut s1: u32) -> (u32, u32) {
    for w in b.chunks_exact(8) {
        s0 = s0
            .wrapping_add(u32::from_le_bytes(w[..4].try_into().unwrap()))
            .wrapping_add(s1);
        s1 = s1
            .wrapping_add(u32::from_le_bytes(w[4..].try_into().unwrap()))
            .wrapping_add(s0);
    }
    (s0, s1)
}

/// A log with one committed frame that replaces `pagina_n` by `img`.
fn wal(pagina_n: u32, img: &[u8], commit: bool) -> Vec<u8> {
    let mut h = Vec::new();
    for x in [0x377f_0682u32, 3_007_000, PAG as u32, 0, 11, 22] {
        h.extend(x.to_be_bytes());
    }
    let (a, b) = somma(&h, 0, 0);
    h.extend(a.to_be_bytes());
    h.extend(b.to_be_bytes());
    let mut f = Vec::new();
    f.extend(pagina_n.to_be_bytes());
    f.extend(if commit { pagina_n } else { 0 }.to_be_bytes());
    let (c, d) = somma(&f, a, b);
    f.extend(11u32.to_be_bytes());
    f.extend(22u32.to_be_bytes());
    let (c, d) = somma(img, c, d);
    f.extend(c.to_be_bytes());
    f.extend(d.to_be_bytes());
    f.extend(img);
    h.extend(f);
    h
}

#[test]
fn il_log_vince_se_confermato() {
    let sql = "CREATE TABLE T (Id INTEGER PRIMARY KEY, X TEXT)";
    let vecchia = foglia(&[vec![V::N, V::T("vecchio")]]);
    let nuova = foglia(&[vec![V::N, V::T("nuovo")]]);
    let base = db(&[("T", sql, vecchia)]);
    let mut s = apri_db(base.clone(), Some(wal(2, &nuova, true))).unwrap();
    assert!(s.con_wal());
    assert_eq!(tutte(&mut s, "T").unwrap()[0][1], "nuovo");
    // not committed: ignored
    let mut s = apri_db(base.clone(), Some(wal(2, &nuova, false))).unwrap();
    assert_eq!(tutte(&mut s, "T").unwrap()[0][1], "vecchio");
    // wrong checksum: ignored
    let mut w = wal(2, &nuova, true);
    let n = w.len();
    w[n - 1] ^= 1;
    let mut s = apri_db(base, Some(w)).unwrap();
    assert_eq!(tutte(&mut s, "T").unwrap()[0][1], "vecchio");
}

// ---- a whole folder ----

fn zstd_grezzo(d: &[u8]) -> Vec<u8> {
    let mut z = vec![0x28, 0xb5, 0x2f, 0xfd, 0xa0];
    z.extend((d.len() as u32).to_le_bytes());
    z.extend((((d.len() as u32) << 3) | 1).to_le_bytes()[..3].iter());
    z.extend(d);
    z
}

/// A codec-2 block of 2 scans: scan 0 has TOF 100 and 150, scan 1 has TOF 100.
fn blocco2() -> Vec<u8> {
    let parole: [u32; 8] = [0, 4, 101, 10, 50, 20, 101, 5];
    let mut piani = Vec::new();
    for k in 0..4 {
        piani.extend(parole.iter().map(|w| (w >> (8 * k)) as u8));
    }
    let z = zstd_grezzo(&piani);
    let mut b = ((z.len() + 8) as u32).to_le_bytes().to_vec();
    b.extend(2u32.to_le_bytes());
    b.extend(z);
    b
}

fn cartella(bin: Vec<u8>, tdf: Vec<u8>) -> FileSet {
    let m = |n: &str, b: Vec<u8>| Membro {
        nome: format!("x.d/{n}"),
        dim: b.len() as u64,
        dati: Box::new(b),
    };
    FileSet {
        membri: vec![m("analysis.tdf", tdf), m("analysis.tdf_bin", bin)],
    }
}

fn tdf_minimo(num_peaks: i64) -> Vec<u8> {
    let frames = foglia(&[vec![
        V::N,
        V::R(1.0),
        V::T("+"),
        V::I(0),
        V::I(0),
        V::I(0),
        V::I(100),
        V::I(50),
        V::I(2),
        V::I(num_peaks),
        V::I(1),
        V::R(25.0),
        V::R(30.0),
        V::R(100.0),
    ]]);
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

#[test]
fn una_cartella_fatta_a_mano_si_apre_e_somma_le_scansioni() {
    let mut l = mzlab_lettori::apri(cartella(blocco2(), tdf_minimo(3)), &limiti()).unwrap();
    assert_eq!(l.formato().id(), "bruker-tdf");
    assert_eq!(l.indice(0).unwrap().len(), 1);
    let s = l.scansione(0, 0).unwrap();
    // equal TOF indices of different scans are one point
    assert_eq!(s.mz.len(), 2);
    assert!(s.mz[0] > 0.0 && s.mz[0] < s.mz[1]);
    let tot: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
    assert_eq!(tot, 35.0);
}

#[test]
fn zstd_falso_e_blocchi_guasti_sono_errori_con_chiave() {
    let chiave = |bin: Vec<u8>, np: i64| {
        let mut l = mzlab_lettori::apri(cartella(bin, tdf_minimo(np)), &limiti()).unwrap();
        l.scansione(0, 0).err().map(|e| e.key)
    };
    assert!(chiave(blocco2(), 3).is_none());
    // not a zstd frame
    let mut b = blocco2();
    for x in &mut b[8..] {
        *x = 0x5a;
    }
    assert!(chiave(b, 3).is_some());
    // a zstd header that announces a huge window and size
    let mut b = vec![0u8; 8];
    b.extend([
        0x28, 0xb5, 0x2f, 0xfd, 0xe0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f, 0, 0, 0,
    ]);
    let n = b.len() as u32;
    b[..4].copy_from_slice(&n.to_le_bytes());
    b[4..8].copy_from_slice(&2u32.to_le_bytes());
    assert!(chiave(b, 3).is_some());
    // the table lies about the number of peaks
    assert!(chiave(blocco2(), 4).is_some());
    assert!(chiave(blocco2(), 1 << 40).is_some());
    // a block cut short, and an empty file of peaks
    let b = blocco2();
    assert!(chiave(b[..b.len() - 5].to_vec(), 3).is_some());
    assert!(chiave(Vec::new(), 3).is_some());
}

#[test]
fn ogni_troncamento_del_database_di_una_cartella_non_manda_in_panico() {
    let t = tdf_minimo(3);
    for n in (0..t.len()).step_by(211) {
        if let Ok(mut l) = mzlab_lettori::apri(cartella(blocco2(), t[..n].to_vec()), &limiti()) {
            let _ = l.indice(0).map(|v| v.len());
            let _ = l.scansione(0, 0);
        }
    }
}

#[test]
fn solo_baf_dice_come_esportare() {
    let b = vec![1u8, 2, 3];
    let fs = FileSet {
        membri: vec![Membro {
            nome: "x.d/analysis.baf".into(),
            dim: 3,
            dati: Box::new(b),
        }],
    };
    let e = mzlab_lettori::apri(fs, &limiti()).err().unwrap();
    assert_eq!(e.key, "err.file.brukerBaf");
}

#[test]
fn una_cartella_agilent_non_e_di_bruker() {
    let m = |n: &str| Membro {
        nome: n.into(),
        dim: 16,
        dati: Box::new(vec![0u8; 16]),
    };
    let mut fs = FileSet {
        membri: vec![
            m("x.D/AcqData/MSScan.bin"),
            m("x.D/AcqData/MSPeriodicActuals.bin"),
        ],
    };
    assert_eq!(
        (mzlab_lettori::bruker::tdf::VOCE.riconosci)(&mut fs),
        mzlab_core::lettori::Fiducia::No
    );
    assert_eq!(
        (mzlab_lettori::bruker::tdf::VOCE_TSF.riconosci)(&mut fs),
        mzlab_core::lettori::Fiducia::No
    );
}
