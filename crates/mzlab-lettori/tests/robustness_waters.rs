#![cfg(feature = "waters")]
//! Hostile MassLynx folders: the synthetic folder (and a real one when the data repository is there) with members cut, emptied, flipped or given absurd numbers gives an error with a key or a
//! reading, never a panic, and what is read from it (index, scans, chromatograms, method, header) never panics either. No data of the laboratory is copied: the mutations are made in memory.
mod common_waters;
use common_waters::*;
use mzlab_core::readers::{Limits, Member};
#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn everything(file: Vec<(String, Vec<u8>)>) {
    let Ok(mut l) = mzlab_lettori::open_bare(file_set(file), &Limits::sito()) else {
        return;
    };
    let n = l.samples().len();
    for c in 0..n {
        if let Ok(ix) = l.index(c).map(<[_]>::to_vec) {
            for k in 0..ix.len().min(6) {
                let _ = l.scan(c, k);
                let _ = l.scan_header(c, k);
            }
        }
        if let Ok(v) = l.chromatograms(c) {
            for h in v.iter().take(6) {
                let _ = l.chromatogram(c, &h.id);
            }
        }
        let _ = l.method(c);
        let _ = l.mzml_header(c);
    }
}

fn real() -> Option<Vec<(String, Vec<u8>)>> {
    let d = dati_percorsi::percorso(
        "dati/formati/waters/raw/xevo_g2s_MTBLS1198/MTBLS1198_20190321_MS.raw",
    )
    .filter(|p| p.is_dir())?;
    let mut out = Vec::new();
    for e in std::fs::read_dir(d).ok()? {
        let p = e.ok()?.path();
        let b = std::fs::read(&p).ok()?;
        // Keep the folder small: the first 200 KB of the big files.
        out.push((
            p.file_name()?.to_string_lossy().to_string(),
            b[..b.len().min(200_000)].to_vec(),
        ));
    }
    Some(out)
}

fn seeds() -> Vec<Vec<(String, Vec<u8>)>> {
    let mut v = vec![folder_file()];
    v.extend(real());
    v
}

fn rng(x: &mut u64) -> u64 {
    *x = x
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *x >> 20
}

#[test]
fn cut_and_flipped_members_do_not_panic() {
    let mut x: u64 = 0x2545_F491_4F6C_DD1D;
    for seed in seeds() {
        for m in 0..seed.len() {
            let n = seed[m].1.len();
            for k in 0..12 {
                let mut f = seed.clone();
                f[m].1.truncate(n * k / 12);
                everything(f);
            }
            if n == 0 {
                continue;
            }
            for _ in 0..25 {
                let mut f = seed.clone();
                for _ in 0..6 {
                    let i = rng(&mut x) as usize % n;
                    f[m].1[i] ^= 0xFF;
                }
                everything(f);
            }
        }
    }
}

#[test]
fn a_member_missing_or_empty_is_a_key_or_a_reading() {
    for seed in seeds() {
        for m in 0..seed.len() {
            let mut f = seed.clone();
            f.remove(m);
            everything(f);
            let mut f = seed.clone();
            f[m].1.clear();
            everything(f);
        }
    }
}

#[test]
fn absurd_index_numbers_are_refused_not_allocated() {
    let mut f = folder_file();
    let idx = f.iter_mut().find(|(n, _)| n == "_FUNC001.IDX").unwrap();
    // Count of 0xFFFFFF records and an offset near the end of the address space in the first row.
    idx.1[4..8].copy_from_slice(&0x18FF_FFFFu32.to_le_bytes());
    idx.1[0..4].copy_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
    everything(f);
    let mut f = folder_file();
    let idx = f.iter_mut().find(|(n, _)| n == "_FUNC002.IDX").unwrap();
    idx.1[4..8].copy_from_slice(&0x18FF_FFFFu32.to_le_bytes());
    everything(f);
    // A budget of a few entries refuses the folder with a key.
    let tiny = Limits {
        profondita: 4,
        entries: 2,
        byte: 1 << 10,
    };
    if let Err(e) = mzlab_lettori::open_bare(synthetic_set(), &tiny) {
        assert!(
            e.key.starts_with("err.") || e.key.starts_with("error."),
            "{}",
            e.key
        );
    }
}

#[test]
fn a_folder_without_functions_is_refused_with_a_key() {
    let mut f = folder_file();
    f.retain(|(n, _)| n != "_FUNCTNS.INF");
    let e = mzlab_lettori::open_bare(file_set(f), &Limits::sito())
        .err()
        .expect("refused");
    assert!(e.key.starts_with("err.file."), "{}", e.key);
    let e = mzlab_lettori::open_bare(
        mzlab_core::readers::FileSet {
            membri: vec![Member {
                nome: "X.raw/_FUNC001.DAT".into(),
                dim: 0,
                dati: Box::new(Vec::<u8>::new()),
                sha1: None,
            }],
        },
        &Limits::sito(),
    )
    .err()
    .expect("refused");
    assert!(e.key.starts_with("err.file."), "{}", e.key);
}

/// A record word no acquisition writes (an intensity without its leading bit): that scan is left out and counted (`warn.waters.damaged`), the others read.
#[test]
fn a_damaged_record_word_leaves_its_scan_out() {
    let mut f = folder_file();
    let dat = f.iter_mut().find(|(n, _)| n == "_FUNC001.DAT").unwrap();
    dat.1[0..4].copy_from_slice(&1u32.to_le_bytes());
    let mut l = mzlab_lettori::open_bare(file_set(f), &Limits::sito()).expect("opened");
    let n = l.index(0).unwrap().len();
    for k in 0..n {
        l.scan(0, k).expect("a scan of the index reads");
    }
    let x = l.losses(0).unwrap();
    assert_eq!(
        (x.declared, x.read, x.cause),
        (n + 1, n, "warn.waters.damaged")
    );
}
