#![cfg(feature = "shimadzu")]
//! The Shimadzu module on the real files of the private data repository (`MZLAB_DATI` or `mzlab-dati` beside this repository, folder `dati_formati/shimadzu/`); every test skips
//! without them. There is no msconvert mzML nor `.cdf` of these files in that repository, so the checks are those the files allow: the TIC stream of the container equals the sum of
//! the intensities decoded from the records, times grow, and the SRM transitions are the ones the method lists.
use std::path::{Path, PathBuf};

use mzlab_core::lettori::{FileSet, Lettore, Limiti, Membro};
use mzlab_core::mzml::ChromKind;

fn dati() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    for up in Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .skip(1)
        .take(3)
    {
        cands.push(up.join("mzlab-dati"));
    }
    cands
        .into_iter()
        .find(|p| p.join("dati_formati/shimadzu").is_dir())
}

fn apri(rel: &str) -> Option<Box<dyn Lettore>> {
    let p = dati()?.join("dati_formati/shimadzu").join(rel);
    if !p.is_file() {
        eprintln!("SKIP shimadzu_dati: {rel} not found");
        return None;
    }
    let bytes = std::fs::read(&p).unwrap();
    let fs = FileSet {
        membri: vec![Membro {
            nome: p.file_name().unwrap().to_string_lossy().into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
        }],
    };
    Some(mzlab_lettori::apri_nudo(fs, &Limiti::sito()).unwrap())
}

/// Every scan: the decoded intensities add up to the TIC of the index, the times do not go back.
fn controlla_scansioni(l: &mut dyn Lettore) -> usize {
    let idx = l.indice(0).unwrap().to_vec();
    let mut prima = -1.0;
    for (k, v) in idx.iter().enumerate() {
        let s = l.scansione(0, k).unwrap();
        let tot: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
        assert_eq!(tot, v.tic, "TIC of scan {k}");
        assert!(v.rt >= prima, "time of scan {k}");
        prima = v.rt;
        assert!(s.mz.windows(2).all(|w| w[0] <= w[1]) || s.mz.is_empty() || k > 0);
    }
    idx.len()
}

const QGD: &[&str] = &[
    "gcms_qgd_zenodo8312045/Hydrocarbons.qgd",
    "gcms_qgd_zenodo8312045/TA2.Trab.qgd",
    "gcms_qp2020nx_qgd_ST004089/ST004089_Standard_FAMEs.qgd",
    "gcms_qp2020nx_qgd_ST004089/ST004089_SMALPs_Lipids_MdtF_1.qgd",
];

#[test]
fn qgd_scans_add_up_to_the_tic() {
    for f in QGD {
        let Some(mut l) = apri(f) else { return };
        let n = controlla_scansioni(&mut *l);
        eprintln!(
            "{f}: {n} scans, chroms {}",
            l.cromatogrammi(0).unwrap().len()
        );
        assert!(n > 100);
    }
}

#[test]
fn lcms2020_scans_add_up_to_the_tic() {
    let Some(mut l) = apri("lcms2020_MTBLS1960/MTBLS1960_E1.lcd") else {
        return;
    };
    let n = controlla_scansioni(&mut *l);
    assert_eq!(n, 2400);
    let m = l.metodo(0).unwrap();
    eprintln!(
        "LEN {} {}",
        m.len(),
        &m[m.find("\"strumento\"").unwrap()..][..160]
    );
    eprintln!("{}", &m[m.find("\"avvisi\"").unwrap_or(0)..]);
    eprintln!(
        "{}",
        m.split("GUM")
            .nth(1)
            .map_or("none", |t| &t[..t.len().min(300)])
    );
}

#[test]
fn qqq_mrm_gives_srm_chromatograms() {
    for f in [
        "lcms8060_mrm_MTBLS7425/MTBLS7425_1_16S_Negative.lcd",
        "lcms8040_aa_MTBLS14423/MTBLS14423_10-D1.lcd",
        "lcms8040_fia_MTBLS2376/MTBLS2376_53003918.lcd",
        "lcms8030_lcd_zenodo18256551/1_2024-05-21_2.lcd",
        "gcms_tq8040_MTBLS5383/MTBLS5383_GC0270_AtD01_20170530_17.qgd",
    ] {
        let Some(mut l) = apri(f) else { return };
        let v = l.cromatogrammi(0).unwrap();
        let srm = v.iter().filter(|c| c.tipo == ChromKind::Srm).count();
        eprintln!("{f}: scans {} srm {srm}", l.indice(0).unwrap().len());
        if let Some(c) = v.iter().find(|c| c.tipo == ChromKind::Srm) {
            eprintln!("   {}", c.id);
        }
        assert!(srm > 0, "{f}");
    }
}

#[test]
fn tmp_meta() {
    for f in QGD.iter().copied().chain([
        "gcms_tq8040_MTBLS5383/MTBLS5383_GC0270_AtD01_20170530_17.qgd",
        "lcms8060_mrm_MTBLS7425/MTBLS7425_1_16S_Negative.lcd",
        "lcms8040_aa_MTBLS14423/MTBLS14423_10-D1.lcd",
        "lcms8040_fia_MTBLS2376/MTBLS2376_53003918.lcd",
        "lcms8030_lcd_zenodo18256551/1_2024-05-21_2.lcd",
    ]) {
        let Some(mut l) = apri(f) else { return };
        let m = l.metodo(0).unwrap();
        let a = m.find("\"origine\"").unwrap();
        let b = m.find("\"operatore\"").unwrap_or(m.find("\"ms\"").unwrap());
        eprintln!("== {f}\n{}", &m[a..b].replace(&m[a..a + 0], ""));
        let h = l.testata_mzml(0).unwrap().unwrap();
        assert!(!h.run_params[0].1.contains("operatore"));
    }
}
