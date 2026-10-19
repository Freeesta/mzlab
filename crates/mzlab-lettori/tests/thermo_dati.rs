#![cfg(feature = "thermo")]
//! The SRM transitions of a TSQ `.raw` and the UV channels of a Dionex detector, read through the module. Real files only (the private data repository,
//! `MZLAB_DATI` or `mzlab-dati` beside this one, folder `dati/formati/`); every test skips without them. There is no msconvert mzML of a TSQ or UV file in that
//! repository, so the parity with msconvert (same number of transitions, Q1/Q3 within 1e-3, times and intensities within 1e-6) runs only when a `<file>.mzML` is
//! found beside the `.raw` (or in `dati/formati/aperti/mzml_msconvert/`), and is skipped otherwise.
use std::path::{Path, PathBuf};

use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::mzml::{self, ChromKind, Item};

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn file(rel: &str) -> Option<PathBuf> {
    let p = dati_percorsi::percorso(&format!("dati/formati/{rel}")).filter(|p| p.is_file());
    if p.is_none() {
        eprintln!("SKIP thermo_dati: {rel} not found");
    }
    p
}

fn apri(p: &Path) -> Box<dyn mzlab_core::lettori::Lettore> {
    let bytes = std::fs::read(p).unwrap();
    let fs = FileSet {
        membri: vec![Membro {
            nome: p.file_name().unwrap().to_string_lossy().into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }],
    };
    mzlab_lettori::apri_nudo(fs, &Limiti::sito()).unwrap()
}

#[test]
fn a_tsq_file_gives_srm_chromatograms_with_q1_q3_and_ce() {
    let Some(p) = file("thermo/raw/PXD057351_WT_R1.raw") else {
        return;
    };
    let mut l = apri(&p);
    let v = l.cromatogrammi(0).unwrap();
    let srm: Vec<_> = v.iter().filter(|c| c.tipo == ChromKind::Srm).collect();
    assert!(srm.len() >= 20, "{} SRM traces", srm.len());
    for c in &srm {
        let (q1, q3, ce, _) = mzlab_core::mzmlhead::srm_id(&c.id);
        assert!(q1.is_some() && q3.is_some(), "{}", c.id);
        assert!(ce.is_some_and(|x| x > 0.0), "no collision energy: {}", c.id);
        let t = l.cromatogramma(0, &c.id).unwrap();
        assert_eq!(t.time.len(), t.intensity.len());
        assert!(t.time.windows(2).all(|w| w[0] <= w[1]), "{}", c.id);
        assert!(t.q1.is_some() && t.q3.is_some());
    }
}

#[test]
fn polarity_comes_from_the_method_table() {
    let Some(p) = file("thermo/raw/MTBLS7889_Fermentation_3_Media_test_001.raw") else {
        return;
    };
    let mut l = apri(&p);
    let ids: Vec<String> = l
        .cromatogrammi(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert!(ids.iter().any(|i| i.ends_with("polarity=negative")));
    assert!(ids
        .iter()
        .any(|i| i.starts_with("SRM") && !i.ends_with("polarity=negative")));
    let m = l.metodo(0).unwrap();
    assert!(m.contains(r#""transizioni""#) && m.contains("negative"));
}

#[test]
fn uv_channels_are_pda_traces_and_the_3d_field_is_a_warning() {
    let Some(p) = file("thermo/pda/MTBLS9839_MET-2021-P17-YG_neg_MS2dd30k_1121_EB_076.raw") else {
        return;
    };
    let mut l = apri(&p);
    let v = l.cromatogrammi(0).unwrap();
    let uv: Vec<_> = v.iter().filter(|c| c.tipo == ChromKind::Pda).collect();
    assert_eq!(uv.len(), 4);
    for c in uv {
        let t = l.cromatogramma(0, &c.id).unwrap();
        assert!(t.time.len() > 1000 && t.time.len() == t.intensity.len());
        assert!(t.time.last().is_some_and(|x| *x > 17.0), "minutes");
    }
    assert!(v.iter().any(|c| c.tipo == ChromKind::Pressure));
    let m = l.metodo(0).unwrap();
    assert!(m.contains("rivelatori") && m.contains("UV 3D field"));
    // UV never shows up as a mass spectrum
    let idx = l.indice(0).unwrap();
    assert!(idx
        .iter()
        .all(|r| r.livello >= 1 && !r.id.to_ascii_lowercase().contains("uv")));
}

#[test]
fn lock_mass_is_in_the_method() {
    let Some(p) = file("thermo/pda/MTBLS775_StandardsMix-b-carbolin_4-CTC_10ng-ml_1.raw") else {
        return;
    };
    let m = apri(&p).metodo(0).unwrap();
    assert!(
        m.contains("lock_mass") && m.contains("214.08943") && m.contains("correzione_ppm"),
        "{m:.200}"
    );
}

#[test]
fn the_chromatograms_match_the_msconvert_mzml_when_there_is_one() {
    let Some(d) = dati_percorsi::percorso("dati/formati") else {
        eprintln!("SKIP thermo_dati: no data repository (MZLAB_DATI)");
        return;
    };
    let mut done = 0;
    for rel in [
        "thermo/raw/PXD057351_WT_R1",
        "thermo/raw/PXD064022_122323_HbMu_MRM_Assay_Test_B3",
        "thermo/pda/MTBLS9839_MET-2021-P17-YG_neg_MS2dd30k_1121_EB_076",
    ] {
        let raw = d.join(format!("{rel}.raw"));
        let name = Path::new(rel)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let mzml_path = [
            raw.with_extension("mzML"),
            d.join("aperti/mzml_msconvert").join(format!("{name}.mzML")),
        ]
        .into_iter()
        .find(|p| p.is_file());
        let (true, Some(mzml_path)) = (raw.is_file(), mzml_path) else {
            eprintln!("SKIP thermo_dati: no msconvert mzML for {name}");
            continue;
        };
        let mut theirs = Vec::new();
        mzml::parse(std::fs::File::open(&mzml_path).unwrap(), |i| {
            if let Item::Chromatogram(c) = i {
                if matches!(c.kind, ChromKind::Srm | ChromKind::Pda) {
                    theirs.push(c);
                }
            }
        })
        .unwrap();
        let mut l = apri(&raw);
        let mine: Vec<_> = l
            .cromatogrammi(0)
            .unwrap()
            .into_iter()
            .filter(|c| matches!(c.tipo, ChromKind::Srm | ChromKind::Pda))
            .collect();
        assert_eq!(mine.len(), theirs.len(), "{name}: number of traces");
        for t in &theirs {
            let m = mine
                .iter()
                .filter(|c| c.tipo == t.kind)
                .map(|c| l.cromatogramma(0, &c.id).unwrap())
                .find(|c| match (c.q1, c.q3, t.q1, t.q3) {
                    (Some(a), Some(b), Some(x), Some(y)) => {
                        (a - x).abs() < 1e-3 && (b - y).abs() < 1e-3
                    }
                    _ => c.id == t.id,
                })
                .unwrap_or_else(|| panic!("{name}: no trace for {}", t.id));
            assert_eq!(m.time.len(), t.time.len(), "{name}: {}", t.id);
            for (a, b) in m
                .time
                .iter()
                .zip(&t.time)
                .chain(m.intensity.iter().zip(&t.intensity))
            {
                assert!(
                    (a - b).abs() <= 1e-6 * b.abs().max(1.0),
                    "{name}: {} {a} {b}",
                    t.id
                );
            }
        }
        done += 1;
    }
    eprintln!("thermo_dati: {done} file(s) compared with msconvert");
}
