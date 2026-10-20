#![cfg(feature = "waters")]
//! External truth for the Waters MSe + lock mass + ion mobility run `Blk_ext_2_C18MSe_1.raw` (MTBLS7394): the values below were read from the full msconvert (profile) mzML of the same run
//! (the oracle is not in the repository). Skips without the real folder (`MZLAB_ROSINA_DATI`, default `~/mzLab/rosina-dati`). msconvert writes one spectrum per drift bin (200 per cycle);
//! the reader writes one per cycle, so the oracle is compared on cycle sums: TIC, RT and the intensity-weighted mean m/z.
use std::path::PathBuf;

use mzlab_core::lettori::{FileSet, Limiti, Membro};

fn cartella() -> Option<FileSet> {
    let base = std::env::var_os("MZLAB_ROSINA_DATI")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join("mzLab/rosina-dati")))?;
    let d = base.join("waters/raw/MTBLS7394_mse+lockmass+ims/Blk_ext_2_C18MSe_1.raw");
    if !d.is_dir() {
        eprintln!("SKIP waters_mse_oracolo: {} not found", d.display());
        return None;
    }
    let nome = d.file_name()?.to_string_lossy().to_string();
    let mut membri = Vec::new();
    for e in std::fs::read_dir(&d).ok()? {
        let p = e.ok()?.path();
        if !p.is_file() {
            continue;
        }
        let f = std::fs::File::open(&p).ok()?;
        membri.push(Membro {
            nome: format!("{nome}/{}", p.file_name()?.to_string_lossy()),
            dim: f.metadata().ok()?.len(),
            dati: Box::new(f),
            sha1: None,
        });
    }
    Some(FileSet { membri })
}

/// (function, cycle, RT in minutes, TIC of the cycle, intensity-weighted mean m/z) of the oracle.
const ORACOLO: &[(u32, u32, f64, f64, f64)] = &[
    (1, 1, 0.068800002337, 59478.0, 285.8191690992991),
    (1, 2, 0.119599997997, 55347.0, 272.89677553846985),
    (1, 3, 0.170416668057, 52744.0, 281.0631935025462),
    (2, 1, 0.094200000167, 19247.0, 309.3814946165356),
    (2, 2, 0.145016670227, 19988.0, 299.8612960537171),
    (2, 3, 0.223766669631, 21651.0, 306.9755721440861),
    (3, 1, 0.042116668075, 420094.0, 504.01384624600394),
    (3, 2, 0.197099998593, 334312.0, 448.1792944013757),
    (3, 3, 0.352083325386, 340904.0, 445.13315156738537),
];

#[test]
fn mse_and_lock_mass_match_the_msconvert_oracle() {
    let Some(fs) = cartella() else { return };
    let mut l = mzlab_lettori::apri_nudo(fs, &Limiti::sito()).unwrap();
    let idx = l.indice(0).unwrap().to_vec();
    // Cycles per function: 48400/200, 48200/200 and 19400/200 spectra of the oracle.
    for (f, n) in [(1, 242), (2, 241), (3, 97)] {
        let c = idx
            .iter()
            .filter(|v| v.id.starts_with(&format!("function={f} ")))
            .count();
        assert_eq!(c, n, "cycles of function {f}");
    }
    for &(f, cyc, rt, tic, mz_medio) in ORACOLO {
        let id = format!("function={f} process=0 scan={cyc}");
        let k = idx
            .iter()
            .position(|v| v.id == id)
            .unwrap_or_else(|| panic!("{id} missing"));
        let v = &idx[k];
        assert!((v.rt - rt).abs() < 1e-6, "{id}: RT {} vs {rt}", v.rt);
        assert_eq!(v.tic, tic, "{id}: TIC");
        assert_eq!(v.livello, if f == 2 { 2 } else { 1 }, "{id}: level");
        assert_eq!(
            v.filtro.contains("lock mass"),
            f == 3,
            "{id}: filter {}",
            v.filtro
        );
        // msconvert's 775 +- 725 m/z "precursor" of every MSe spectrum is the scan range, not an isolation.
        assert!(v.precursore.is_none(), "{id}: no precursor");
        let s = l.scansione(0, k).unwrap();
        let (mut somma, mut pesata) = (0.0, 0.0);
        for (m, i) in s.mz.iter().zip(&s.intensity) {
            somma += f64::from(*i);
            pesata += m * f64::from(*i);
        }
        assert!(
            (somma - tic).abs() <= 2e-3 * tic,
            "{id}: sum {somma} vs {tic}"
        );
        let medio = pesata / somma;
        assert!(
            (medio - mz_medio).abs() <= 1e-6 * mz_medio,
            "{id}: mean m/z {medio} vs {mz_medio}"
        );
    }
}
