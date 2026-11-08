//! Hostile and hand-built input for the Bruker reader: a SQLite file and a `tdf_bin` made inside the test (no real data), page cycles, long varints, fake zstd, truncations, a write-ahead log.
#![cfg(feature = "bruker")]

use mzlab_core::lettori::{FileSet, Membro};
use mzlab_lettori::bruker::sqlite_min::Sqlite;

mod comune_sqlite;

use comune_sqlite::*;

fn all_of_them(s: &mut Sqlite, nome: &str) -> mzlab_core::error::Result<Vec<Vec<String>>> {
    let t = s.tabella(nome).unwrap().clone();
    let mut out = Vec::new();
    s.righe(&t, |r| {
        out.push(r.iter().map(|v| v.as_text().unwrap_or_default()).collect());
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
    let r = all_of_them(&mut s, "T").unwrap();
    assert_eq!(r.len(), 2);
    assert_eq!(r[0][0], "1");
    assert_eq!(r[1][1], "b");
    assert!(!s.with_wal());
}

#[test]
fn senza_rowid_mette_la_chiave_per_prima() {
    // WITHOUT ROWID table T(a, b, PRIMARY KEY(b)): the record is (b, a)
    let cell_list: Vec<Vec<u8>> = [("x", 1), ("y", 2)]
        .iter()
        .map(|(b, a)| {
            let r = record(&[V::T(b), V::I(*a)]);
            let mut c = varint(r.len() as u64);
            c.extend(r);
            c
        })
        .collect();
    let p = page(10, 0, &cell_list, 0);
    let mut s = apri_db(
        db(&[(
            "T",
            "CREATE TABLE T (a INTEGER, b TEXT, PRIMARY KEY (b)) WITHOUT ROWID",
            p,
        )]),
        None,
    )
    .unwrap();
    let r = all_of_them(&mut s, "T").unwrap();
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
    let p = page(5, 0, &[c], 2);
    let mut s = apri_db(
        db(&[("T", "CREATE TABLE T (Id INTEGER PRIMARY KEY)", p)]),
        None,
    )
    .unwrap();
    assert!(all_of_them(&mut s, "T").is_err());
}

#[test]
fn varint_lunghi_e_record_guasti_sono_errori() {
    // header length claims a 10-byte varint
    let mut rec = vec![0xff; 10];
    rec.extend([1, 2, 3]);
    let p = page(13, 0, &[cella_foglia(1, &rec)], 0);
    let mut s = apri_db(
        db(&[("T", "CREATE TABLE T (Id INTEGER PRIMARY KEY, X)", p)]),
        None,
    )
    .unwrap();
    assert!(all_of_them(&mut s, "T").is_err());
    // a cell pointer outside the page
    let mut p = foglia(&[vec![V::N, V::I(1)]]);
    p[8..10].copy_from_slice(&0xfff0u16.to_be_bytes());
    p[10..12].copy_from_slice(&0xffffu16.to_be_bytes());
    let mut s = apri_db(
        db(&[("T", "CREATE TABLE T (Id INTEGER PRIMARY KEY, X)", p)]),
        None,
    )
    .unwrap();
    let _ = all_of_them(&mut s, "T");
}

#[test]
fn ogni_troncamento_e_un_errore_o_un_risultato() {
    let t = foglia(&[vec![V::N, V::T("a")]]);
    let b = db(&[("T", "CREATE TABLE T (Id INTEGER PRIMARY KEY, X TEXT)", t)]);
    for n in (0..b.len()).step_by(37) {
        if let Ok(mut s) = apri_db(b[..n].to_vec(), None)
            && s.tabella("T").is_some()
        {
            let _ = all_of_them(&mut s, "T");
        }
    }
}

fn summed(b: &[u8], mut s0: u32, mut s1: u32) -> (u32, u32) {
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
fn wal(page_n: u32, img: &[u8], commit: bool) -> Vec<u8> {
    let mut h = Vec::new();
    for x in [0x377f_0682u32, 3_007_000, PAG as u32, 0, 11, 22] {
        h.extend(x.to_be_bytes());
    }
    let (a, b) = summed(&h, 0, 0);
    h.extend(a.to_be_bytes());
    h.extend(b.to_be_bytes());
    let mut f = Vec::new();
    f.extend(page_n.to_be_bytes());
    f.extend(if commit { page_n } else { 0 }.to_be_bytes());
    let (c, d) = summed(&f, a, b);
    f.extend(11u32.to_be_bytes());
    f.extend(22u32.to_be_bytes());
    let (c, d) = summed(img, c, d);
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
    assert!(s.with_wal());
    assert_eq!(all_of_them(&mut s, "T").unwrap()[0][1], "nuovo");
    // not committed: ignored
    let mut s = apri_db(base.clone(), Some(wal(2, &nuova, false))).unwrap();
    assert_eq!(all_of_them(&mut s, "T").unwrap()[0][1], "vecchio");
    // wrong checksum: ignored
    let mut w = wal(2, &nuova, true);
    let n = w.len();
    w[n - 1] ^= 1;
    let mut s = apri_db(base, Some(w)).unwrap();
    assert_eq!(all_of_them(&mut s, "T").unwrap()[0][1], "vecchio");
}

// ---- a whole folder ----

fn cartella(bin: Vec<u8>, tdf: Vec<u8>) -> FileSet {
    let m = |n: &str, b: Vec<u8>| Membro {
        nome: format!("x.d/{n}"),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    };
    FileSet {
        membri: vec![m("analysis.tdf", tdf), m("analysis.tdf_bin", bin)],
    }
}

#[test]
fn una_cartella_fatta_a_mano_si_apre_e_somma_le_scansioni() {
    let mut l = mzlab_lettori::apri(cartella(blocco2(), tdf_minimo(3)), &limits()).unwrap();
    assert_eq!(l.format().id(), "bruker-tdf");
    assert_eq!(l.index(0).unwrap().len(), 1);
    let s = l.scan(0, 0).unwrap();
    // equal TOF indices of different scans are one point
    assert_eq!(s.mz.len(), 2);
    assert!(s.mz[0] > 0.0 && s.mz[0] < s.mz[1]);
    let tot: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
    assert_eq!(tot, 35.0);
}

#[test]
fn un_timstof_e_ad_alta_risoluzione() {
    // the header names the quadrupole-TOF: the scan table reads the run as high resolution (hr1), as the page and the record show it
    let mut l = mzlab_lettori::apri(cartella(blocco2(), tdf_minimo(3)), &limits()).unwrap();
    let p = mzlab_core::upload::prepare_lettore(&mut *l, 0, &[], "x.d").unwrap();
    assert!(p.table.hr1);
}

#[test]
fn bpc_uguale_prima_e_dopo_il_giro() {
    // the BPC asked first decodes the frames; after every spectrum has been read it comes from the maxima kept by the reader
    let apri = || mzlab_lettori::apri(cartella(blocco2(), tdf_minimo(3)), &limits()).unwrap();
    let before = apri().chromatogram(0, "BPC").unwrap();
    let mut l = apri();
    let n = l.index(0).unwrap().len();
    let maxima: Vec<f64> = (0..n)
        .map(|k| {
            l.scan(0, k)
                .unwrap()
                .intensity
                .iter()
                .fold(0.0f64, |m, &y| m.max(f64::from(y)))
        })
        .collect();
    let dopo = l.chromatogram(0, "BPC").unwrap();
    assert_eq!(before.intensity, dopo.intensity);
    assert_eq!(before.time, dopo.time);
    assert_eq!(dopo.intensity, maxima);
    assert!(maxima[0] > 0.0);
}

#[test]
fn zstd_falso_e_blocchi_guasti_sono_errori_con_chiave() {
    let chiave = |bin: Vec<u8>, np: i64| {
        let mut l = mzlab_lettori::apri(cartella(bin, tdf_minimo(np)), &limits()).unwrap();
        l.scan(0, 0).err().map(|e| e.key)
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
fn un_tdf_bin_tagliato_legge_i_frame_interi_e_conta_gli_altri() {
    let b = blocco2();
    let n = b.len() as i64;
    let tims = [0, n, 2 * n, 3 * n, 4 * n];
    let intero: Vec<u8> = b.iter().copied().cycle().take(b.len() * 5).collect();
    let mut l =
        mzlab_lettori::apri(cartella(intero.clone(), tdf_frames(&tims, 3)), &limits()).unwrap();
    assert_eq!((l.index(0).unwrap().len(), l.missing(0)), (5, 0));
    // cut inside the fourth block, at its first byte, and inside its header: three frames read, two counted
    for taglio in [
        3 * b.len() + 9,
        3 * b.len(),
        3 * b.len() + 3,
        4 * b.len() - 1,
    ] {
        let mut l = mzlab_lettori::apri(
            cartella(intero[..taglio].to_vec(), tdf_frames(&tims, 3)),
            &limits(),
        )
        .unwrap();
        assert_eq!(l.index(0).unwrap().len(), 3, "{taglio}");
        assert_eq!(l.missing(0), 2, "{taglio}");
        assert_eq!(l.samples()[0].scansioni, 3);
        for k in 0..3 {
            let s = l.scan(0, k).unwrap();
            assert_eq!(s.mz.len(), 2);
        }
        assert!(l.scan(0, 3).is_err());
        let tic = l.chromatogram(0, "TIC").unwrap();
        let bpc = l.chromatogram(0, "BPC").unwrap();
        assert_eq!((tic.time.len(), bpc.time.len()), (3, 3));
    }
    // the frames out of order in the table, the cut one first
    let mut l = mzlab_lettori::apri(
        cartella(
            intero[..2 * b.len() + 4].to_vec(),
            tdf_frames(&[2 * n, 0, n], 3),
        ),
        &limits(),
    )
    .unwrap();
    assert_eq!((l.index(0).unwrap().len(), l.missing(0)), (2, 1));
    // nothing left: an empty index, every spectrum counted
    let mut l = mzlab_lettori::apri(cartella(Vec::new(), tdf_frames(&tims, 3)), &limits()).unwrap();
    assert_eq!((l.index(0).unwrap().len(), l.missing(0)), (0, 5));
}

#[test]
fn ogni_troncamento_del_database_di_una_cartella_non_manda_in_panico() {
    let t = tdf_minimo(3);
    for n in (0..t.len()).step_by(211) {
        if let Ok(mut l) = mzlab_lettori::apri(cartella(blocco2(), t[..n].to_vec()), &limits()) {
            let _ = l.index(0).map(|v| v.len());
            let _ = l.scan(0, 0);
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
            sha1: None,
        }],
    };
    let e = mzlab_lettori::apri(fs, &limits()).err().unwrap();
    assert_eq!(e.key, "err.file.brukerBaf");
}

/// A folder that lacks the database or its `*_bin` (TDF or TSF) names the file it lacks; a database that is not SQLite (an empty copy) is an error with a key, not «not accepted».
#[test]
fn una_cartella_senza_un_suo_file_dice_quale() {
    let m = |n: &str, b: Vec<u8>| Membro {
        nome: format!("x.d/{n}"),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    };
    for (nomi, absent) in [
        (vec!["analysis.tsf_bin"], "analysis.tsf"),
        (vec!["analysis.tdf_bin"], "analysis.tdf"),
        (vec!["analysis.tsf"], "analysis.tsf_bin"),
        (vec!["analysis.tdf"], "analysis.tdf_bin"),
    ] {
        let membri = nomi.iter().map(|n| m(n, tdf_minimo(1))).collect();
        let e = mzlab_lettori::apri(FileSet { membri }, &limits())
            .err()
            .unwrap();
        assert_eq!(e.key, "err.file.brukerNoBin", "{nomi:?}");
        assert!(
            e.params.contains(&("file", absent.to_string())),
            "{:?}",
            e.params
        );
        assert!(
            e.params.contains(&("name", "x.d".to_string())),
            "{:?}",
            e.params
        );
    }
    let e = mzlab_lettori::apri(cartella(blocco2(), Vec::new()), &limits())
        .err()
        .unwrap();
    assert!(
        e.key.starts_with("err.") && e.key != "err.file.notAccepted",
        "{}",
        e.key
    );
}

#[test]
fn una_cartella_agilent_non_e_di_bruker() {
    let m = |n: &str| Membro {
        nome: n.into(),
        dim: 16,
        dati: Box::new(vec![0u8; 16]),
        sha1: None,
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
        (mzlab_lettori::bruker::tdf::ENTRY_TSF.riconosci)(&mut fs),
        mzlab_core::lettori::Fiducia::No
    );
}
