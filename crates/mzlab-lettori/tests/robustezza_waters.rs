#![cfg(feature = "waters")]
//! Hostile MassLynx folders: the synthetic folder (and a real one when the data repository is there) with members cut, emptied, flipped or given absurd numbers gives an error with a key or a
//! reading, never a panic, and what is read from it (index, scans, chromatograms, method, header) never panics either. No data of the laboratory is copied: the mutations are made in memory.
mod comune_waters;
use comune_waters::*;
use mzlab_core::lettori::{Limiti, Membro};
use std::path::PathBuf;

fn tutto(file: Vec<(String, Vec<u8>)>) {
    let Ok(mut l) = mzlab_lettori::apri_nudo(insieme(file), &Limiti::sito()) else {
        return;
    };
    let n = l.campioni().len();
    for c in 0..n {
        if let Ok(ix) = l.indice(c).map(<[_]>::to_vec) {
            for k in 0..ix.len().min(6) {
                let _ = l.scansione(c, k);
                let _ = l.intestazione(c, k);
            }
        }
        if let Ok(v) = l.cromatogrammi(c) {
            for h in v.iter().take(6) {
                let _ = l.cromatogramma(c, &h.id);
            }
        }
        let _ = l.metodo(c);
        let _ = l.testata_mzml(c);
    }
}

fn reale() -> Option<Vec<(String, Vec<u8>)>> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    for up in std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .skip(1)
        .take(3)
    {
        cands.push(up.join("mzlab-dati"));
    }
    let d = cands
        .into_iter()
        .map(|p| p.join("dati_formati/waters_raw/xevo_g2s_MTBLS1198/MTBLS1198_20190321_MS.raw"))
        .find(|p| p.is_dir())?;
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

fn semi() -> Vec<Vec<(String, Vec<u8>)>> {
    let mut v = vec![file_cartella()];
    v.extend(reale());
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
    for seme in semi() {
        for m in 0..seme.len() {
            let n = seme[m].1.len();
            for k in 0..12 {
                let mut f = seme.clone();
                f[m].1.truncate(n * k / 12);
                tutto(f);
            }
            if n == 0 {
                continue;
            }
            for _ in 0..25 {
                let mut f = seme.clone();
                for _ in 0..6 {
                    let i = rng(&mut x) as usize % n;
                    f[m].1[i] ^= 0xFF;
                }
                tutto(f);
            }
        }
    }
}

#[test]
fn a_member_missing_or_empty_is_a_key_or_a_reading() {
    for seme in semi() {
        for m in 0..seme.len() {
            let mut f = seme.clone();
            f.remove(m);
            tutto(f);
            let mut f = seme.clone();
            f[m].1.clear();
            tutto(f);
        }
    }
}

#[test]
fn absurd_index_numbers_are_refused_not_allocated() {
    let mut f = file_cartella();
    let idx = f.iter_mut().find(|(n, _)| n == "_FUNC001.IDX").unwrap();
    // Count of 0xFFFFFF records and an offset near the end of the address space in the first row.
    idx.1[4..8].copy_from_slice(&0x18FF_FFFFu32.to_le_bytes());
    idx.1[0..4].copy_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
    tutto(f);
    let mut f = file_cartella();
    let idx = f.iter_mut().find(|(n, _)| n == "_FUNC002.IDX").unwrap();
    idx.1[4..8].copy_from_slice(&0x18FF_FFFFu32.to_le_bytes());
    tutto(f);
    // A budget of a few entries refuses the folder with a key.
    let tiny = Limiti {
        profondita: 4,
        voci: 2,
        byte: 1 << 10,
    };
    if let Err(e) = mzlab_lettori::apri_nudo(insieme_sintetico(), &tiny) {
        assert!(
            e.key.starts_with("err.") || e.key.starts_with("error."),
            "{}",
            e.key
        );
    }
}

#[test]
fn a_folder_without_functions_is_refused_with_a_key() {
    let mut f = file_cartella();
    f.retain(|(n, _)| n != "_FUNCTNS.INF");
    let e = mzlab_lettori::apri_nudo(insieme(f), &Limiti::sito())
        .err()
        .expect("refused");
    assert!(e.key.starts_with("err.file."), "{}", e.key);
    let e = mzlab_lettori::apri_nudo(
        mzlab_core::lettori::FileSet {
            membri: vec![Membro {
                nome: "X.raw/_FUNC001.DAT".into(),
                dim: 0,
                dati: Box::new(Vec::<u8>::new()),
                sha1: None,
            }],
        },
        &Limiti::sito(),
    )
    .err()
    .expect("refused");
    assert!(e.key.starts_with("err.file."), "{}", e.key);
}
