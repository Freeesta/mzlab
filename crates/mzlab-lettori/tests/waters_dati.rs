#![cfg(feature = "waters")]
//! MassLynx folders read through the module, on real files only (the private data repository, `MZLAB_DATI` or `mzlab-dati` beside this one, folder `dati/formati/`); every
//! test skips without them. Parity with the mzML of msconvert where the repository has one: `waters/raw/xevo_tqxs_pair_ST004453/` (SRM chromatograms and the analog traces).
//! The TOF, MSe, DDA and old TQ folders have no mzML of msconvert there: their numbers are checked against the index of the folder itself (the TIC of each scan).
mod comune_waters;
use std::path::Path;

use mzlab_core::lettori::{FileSet, Lettore, Limiti, Membro};
use mzlab_core::mzml::{self, Chromatogram, Item};

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

/// The folder `rel` (under `dati/formati/`) as a set of members named `X.raw/<file>`, read from disk by position.
pub fn cartella(rel: &str) -> Option<FileSet> {
    let Some(d) = dati_percorsi::percorso(&format!("dati/formati/{rel}")).filter(|p| p.is_dir())
    else {
        eprintln!("SKIP waters_dati: {rel} not found");
        return None;
    };
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

pub fn apri(rel: &str) -> Option<Box<dyn Lettore>> {
    let fs = cartella(rel)?;
    Some(
        mzlab_lettori::apri_nudo(fs, &Limiti::sito())
            .unwrap_or_else(|e| panic!("{rel}: {}", e.key)),
    )
}

const CARTELLE: &[&str] = &[
    "waters/raw/xevo_tqxs_pair_ST004453/ST004453_TQXS_04292022_038.raw",
    "waters/raw/xevo_tqxs_mrm_MTBLS12109/MTBLS12109_MP189_NAD_pos_ESI_CAL_0_1.raw",
    "waters/raw/xevo_tqs_mrm_MTBLS685/MTBLS685_MousePlasma_Set1_057.raw",
    "waters/raw/tqs_mrm_MTBLS2612/MTBLS2612_E1101118e02.raw",
    "waters/raw/xevo_tq_mrm_MTBLS661/MTBLS661_S48.raw",
    "waters/raw/xevo_tq_abs_desi_mrm_zenodo10807654/gp_lung_01.raw",
    "waters/raw/xevo_g2s_MTBLS1198/MTBLS1198_20190321_MS.raw",
    "waters/raw/synapt_hdms_MTBLS178/MTBLS178_Neg_R57.raw",
    "waters/raw/synapt_g2si_reims_MTBLS957/MTBLS957_JH_171129_QQC093C.raw",
    "aperti/lettori_open_test/pwiz/Waters/SONAR_Short.raw",
    "aperti/lettori_open_test/pwiz/Waters/HDMSe_Short_noLM.raw",
    "aperti/lettori_open_test/pwiz/Waters/HDDDA_Short_noLM.raw",
    "aperti/lettori_open_test/pwiz/Waters/HDMRM_Short_noLM.raw",
];

#[test]
fn every_real_folder_opens_and_its_scans_add_up_to_the_index() {
    for rel in CARTELLE {
        let Some(mut l) = apri(rel) else { continue };
        let idx = l.indice(0).unwrap().to_vec();
        assert!(
            idx.windows(2).all(|w| w[0].rt <= w[1].rt),
            "{rel}: index in time order"
        );
        let ch = l.cromatogrammi(0).unwrap();
        assert!(idx.len() + ch.len() > 0, "{rel}: nothing to show");
        for k in (0..idx.len()).step_by((idx.len() / 25).max(1)) {
            let s = l.scansione(0, k).unwrap();
            assert_eq!(s.mz.len(), s.intensity.len(), "{rel} scan {k}");
            assert!(
                s.mz.iter().all(|m| m.is_finite() && *m > 0.0),
                "{rel} scan {k}: m/z"
            );
            assert!(
                s.intensity.iter().all(|i| i.is_finite() && *i >= 0.0),
                "{rel} scan {k}"
            );
            let sum: f64 = s.intensity.iter().map(|&i| f64::from(i)).sum();
            if idx[k].tic > 0.0 {
                assert!(
                    (sum - idx[k].tic).abs() <= 2e-3 * idx[k].tic,
                    "{rel} scan {k}: sum {sum} vs TIC {}",
                    idx[k].tic
                );
            }
        }
        let m = l.metodo(0).unwrap();
        let doc = comune_waters::controlla_schema(&m, rel);
        assert_eq!(doc["origine"]["formato"], "waters-raw", "{rel}");
        assert!(m.len() < 400_000, "{rel}: metodo of {} B", m.len());
        let h = l.testata_mzml(0).unwrap().unwrap();
        assert!(
            h.run_params.iter().any(|(k, _)| k == "mzLab metodo/1"),
            "{rel}"
        );
    }
}

/// An id without the `ce=` and `name=` words mzLab adds to the one of msconvert.
fn bare(id: &str) -> String {
    id.split(" name=")
        .next()
        .unwrap_or(id)
        .split(' ')
        .filter(|w| !w.starts_with("ce="))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The chromatograms of an msconvert mzML, by id.
fn mzml_cromi(p: &Path) -> Vec<Chromatogram> {
    let mut v = Vec::new();
    mzml::parse(std::fs::File::open(p).unwrap(), |i| {
        if let Item::Chromatogram(c) = i {
            v.push(c);
        }
    })
    .unwrap();
    v
}

#[test]
fn chromatograms_match_the_msconvert_mzml() {
    if dati_percorsi::radice().is_none() {
        eprintln!("SKIP waters_dati: no data repository (MZLAB_DATI)");
        return;
    }
    let rel = "waters/raw/xevo_tqxs_pair_ST004453/ST004453_TQXS_04292022_038";
    let Some(mut l) = apri(&format!("{rel}.raw")) else {
        return;
    };
    let Some(mz) = dati_percorsi::percorso(&format!("dati/formati/{rel}.mzML")) else {
        eprintln!("SKIP waters_dati: no mzML for {rel}");
        return;
    };
    let theirs = mzml_cromi(&mz);
    assert!(l.indice(0).unwrap().is_empty(), "MRM folder has no spectra");
    let mine = l.cromatogrammi(0).unwrap();
    assert_eq!(mine.len(), theirs.len(), "number of traces");
    let (mut srm, mut other) = (0, 0);
    for t in &theirs {
        let want = t.id.trim_start_matches("- ");
        let alias = match want {
            "A" => "% A",
            "B" => "% B",
            w => w,
        };
        let id = mine
            .iter()
            .find(|c| bare(&c.id) == want || c.id == alias)
            .unwrap_or_else(|| {
                panic!(
                    "no trace for {} (mine: {:?})",
                    t.id,
                    mine.iter()
                        .filter(|c| c.id.contains("function=2"))
                        .take(14)
                        .map(|c| &c.id)
                        .collect::<Vec<_>>()
                )
            })
            .id
            .clone();
        let m = l.cromatogramma(0, &id).unwrap();
        assert_eq!(m.time.len(), t.time.len(), "{}", t.id);
        for (a, b) in m
            .time
            .iter()
            .zip(&t.time)
            .chain(m.intensity.iter().zip(&t.intensity))
        {
            assert!(
                (a - b).abs() <= 1e-6 * b.abs().max(1.0),
                "{}: {a} {b}",
                t.id
            );
        }
        if want.contains("SRM") {
            srm += 1
        } else {
            other += 1
        }
    }
    eprintln!("waters_dati: {srm} SRM + {other} other traces equal to msconvert");
}

#[test]
fn folders_without_function_table_are_refused_with_a_key() {
    for rel in [
        "waters/raw/xevo_g2xs_lipid_MTBLS1584/MTBLS1584_USF_Oct18_LCMS_CMH_lipidomics_046n.raw",
        "waters/raw/cyclic_ims_PXD052561/20230920_NM4_1187_ECD_2_CID_25.raw",
    ] {
        let Some(fs) = cartella(rel) else { continue };
        match mzlab_lettori::apri_nudo(fs, &Limiti::sito()) {
            Ok(mut l) => eprintln!("{rel}: opens, {} scans", l.indice(0).map_or(0, <[_]>::len)),
            Err(e) => assert!(e.key.starts_with("err.file."), "{rel}: {}", e.key),
        }
    }
}
