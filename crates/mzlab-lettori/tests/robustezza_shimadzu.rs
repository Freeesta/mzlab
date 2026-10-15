#![cfg(feature = "shimadzu")]
//! Hostile Shimadzu files: synthetic containers (and, when the data repository is there, one real file per family) cut at many points, with bytes flipped and with 32-bit words set to
//! 0xFFFFFFFF (counts, sizes and offsets) give an error with a key or a reading, never a panic, and what is read never panics either. No data of the laboratory is copied: the mutations are
//! made in memory.
mod comune;

use comune::shimadzu::*;
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use std::path::PathBuf;

fn insieme(byte: Vec<u8>) -> FileSet {
    FileSet {
        membri: vec![Membro {
            nome: "x.lcd".into(),
            dim: byte.len() as u64,
            dati: Box::new(byte),
            sha1: None,
        }],
    }
}

fn reali() -> Vec<Vec<u8>> {
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
    let Some(d) = cands
        .into_iter()
        .find(|p| p.join("dati_formati/shimadzu").is_dir())
    else {
        eprintln!("SKIP robustezza_shimadzu: no data repository, synthetic seeds only");
        return Vec::new();
    };
    let mut v = Vec::new();
    for sub in std::fs::read_dir(d.join("dati_formati/shimadzu"))
        .into_iter()
        .flatten()
        .flatten()
    {
        let mut fs: Vec<_> = std::fs::read_dir(sub.path())
            .into_iter()
            .flatten()
            .flatten()
            .collect();
        fs.sort_by_key(|f| f.path());
        // the smallest file of each folder keeps the test quick
        if let Some(b) = fs
            .iter()
            .filter(|f| {
                f.metadata()
                    .is_ok_and(|m| m.is_file() && m.len() < 1_000_000)
            })
            .filter_map(|f| std::fs::read(f.path()).ok())
            .next()
        {
            v.push(b);
        }
    }
    v
}

fn semi() -> Vec<Vec<u8>> {
    let mut v = vec![
        qgd(6),
        qgd_mrm(4),
        lcd2020(3),
        lcd_tlm(4),
        lcd_qtof(),
        lcd_solo_pda(),
    ];
    v.extend(reali());
    v
}

fn tutto(byte: Vec<u8>) {
    let Ok(mut l) = mzlab_lettori::apri(insieme(byte), &Limiti::sito()) else {
        return;
    };
    let Ok(ix) = l.indice(0).map(<[_]>::to_vec) else {
        return;
    };
    for k in [0, ix.len() / 2, ix.len().saturating_sub(1)] {
        if k < ix.len() {
            let _ = l.scansione(0, k);
            let _ = l.intestazione(0, k);
        }
    }
    if let Ok(v) = l.cromatogrammi(0) {
        for c in v.iter().take(4) {
            let _ = l.cromatogramma(0, &c.id);
        }
    }
    let _ = l.metodo(0);
    let _ = l.testata_mzml(0);
}

fn prossimo(x: &mut u64) -> usize {
    *x = x
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*x >> 20) as usize
}

#[test]
fn truncated_flipped_and_saturated_files_do_not_panic() {
    for s in semi() {
        let n = s.len();
        for k in 0..40 {
            tutto(s[..n * k / 40].to_vec());
        }
        let mut x: u64 = 0x2545_F491_4F6C_DD1D;
        for r in 0..60 {
            let mut m = s.clone();
            for _ in 0..8 {
                let i = prossimo(&mut x) % m.len();
                if r % 2 == 0 {
                    m[i] ^= 0xFF;
                } else {
                    let i = (i / 4 * 4).min(m.len().saturating_sub(4));
                    m[i..i + 4].copy_from_slice(&[0xFF; 4]);
                }
            }
            tutto(m);
        }
    }
}

#[test]
fn every_refusal_has_a_key() {
    for s in semi() {
        for cut in [0, 7, 511, 512, 1024, s.len() / 3] {
            if let Err(e) =
                mzlab_lettori::apri(insieme(s[..cut.min(s.len())].to_vec()), &Limiti::sito())
            {
                assert!(e.key.starts_with("err."), "{}", e.key);
            }
        }
    }
}

#[test]
fn a_table_over_the_limit_is_refused_with_a_key() {
    let l = Limiti {
        voci: 4,
        ..Limiti::sito()
    };
    for b in [qgd(6), lcd2020(3), lcd_tlm(8)] {
        let e = mzlab_lettori::apri_nudo(insieme(b), &l)
            .err()
            .expect("over the limit");
        assert!(
            e.key.starts_with("err.") || e.key.starts_with("error."),
            "{}",
            e.key
        );
    }
}
