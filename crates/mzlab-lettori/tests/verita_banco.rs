//! The Rosina bench (docs/studi/ROSINA.md F) tried on files built here: a truth `verita/1` made from the data that wrote an mzML must give `L`/`P` for the reader of the module `aperti`,
//! and a truth that differs by more than the tolerance must give a defect at the right level. Public CI: no real data.
#![cfg(feature = "aperti")]

mod comune;
#[path = "comune/verita.rs"]
mod verita;

use comune::*;
use mzlab_core::numpress::Np;
use serde_json::{json, Value};
use verita::{confronta, Esito, Tol};

/// The truth the data that wrote the file gives, in the shape of `verita/1`.
fn verita_di(scan: &[Scan]) -> Value {
    let sc: Vec<Value> = scan
        .iter()
        .enumerate()
        .map(|(k, x)| {
            json!({
                "id": format!("controllerType=0 controllerNumber=1 scan={}", k + 1),
                "rt_min": x.rt_min, "livello": x.livello, "polarita": 1,
                "precursore_mz": x.precursore, "profilo": false,
                "n_punti": x.mz.len(), "tic": x.intensita.iter().sum::<f64>()
            })
        })
        .collect();
    let mut punti = serde_json::Map::new();
    for k in [0, scan.len() / 2, scan.len() - 1] {
        punti.insert(
            k.to_string(),
            json!({"mz": scan[k].mz, "intensita": scan[k].intensita}),
        );
    }
    json!({
        "schema": "verita/1",
        "file": {"nome": "sintetico.mzML"},
        "conteggi": {"spettri": {"ms1": scan.iter().filter(|s| s.livello == 1).count(), "ms2": scan.iter().filter(|s| s.livello == 2).count()}, "cromatogrammi": {}},
        "scansioni": sc,
        "punti": punti,
        "cromatogrammi": [],
        "metadati": {"mzml:run@startTimeStamp": {"valore": "2024-03-05T10:11:12Z", "metodo": "origine.file_creato"}}
    })
}

/// Numpress rounds: the bench is run with the tolerances the codec allows (the real corpus uses `Tol::HR` and `Tol::LR`).
const TOL: Tol = Tol {
    mz_ppm: None,
    mz_da: 1e-3,
    rt_min: 1e-4,
    intensita_rel: 1e-3,
    cromatogramma_rel: 1e-6,
};

fn apri_file(scan: &[Scan]) -> Box<dyn mzlab_core::lettori::Lettore> {
    // the mzML of another program has no `metodo/1`; the one the bench is tried on carries the document, as an mzML written by mzLab does
    let metodo = r#"{"schema":"metodo/1","origine":{"file_creato":"2024-03-05T10:11:12Z"}}"#;
    apri(mzml_di(
        scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: false,
        },
        Some(metodo),
    ))
}

#[test]
fn una_verita_giusta_non_ha_difetti() {
    let scan = scansioni();
    let mut l = apri_file(&scan);
    let r = confronta(&mut *l, &verita_di(&scan), 0, TOL, "sintetico");
    assert!(r.difetti().is_empty(), "{:?}", r.difetti());
    // the truth has no chromatograms: F4 is declared missing, so the file is `P`, never `L`
    assert_eq!(r.esito(), "P", "{:?}", r.livelli);
    assert!(r.mancanti().iter().any(|m| m.starts_with("F4")));
    verita::scrivi_rapporto(&[r]);
}

#[test]
fn un_mz_fuori_tolleranza_e_un_difetto_di_f3() {
    let scan = scansioni();
    let mut v = verita_di(&scan);
    v["punti"]["0"]["mz"][3] = json!(scan[0].mz[3] + 0.5);
    let mut l = apri_file(&scan);
    let r = confronta(&mut *l, &v, 0, TOL, "sintetico");
    assert!(
        matches!(r.livelli[3], ("F3", Esito::Difetto(_))),
        "{:?}",
        r.livelli
    );
    assert_eq!(r.esito(), "DIFETTO");
}

#[test]
fn un_conteggio_o_un_tempo_sbagliato_sono_difetti_di_f1_e_f2() {
    let scan = scansioni();
    let mut l = apri_file(&scan);
    let mut v = verita_di(&scan);
    v["conteggi"]["spettri"]["ms2"] = json!(99);
    v["scansioni"][1]["rt_min"] = json!(7.0);
    let r = confronta(&mut *l, &v, 0, TOL, "sintetico");
    assert!(
        matches!(r.livelli[1], ("F1", Esito::Difetto(_))),
        "{:?}",
        r.livelli
    );
    assert!(
        matches!(r.livelli[2], ("F2", Esito::Difetto(_))),
        "{:?}",
        r.livelli
    );
}

#[test]
fn un_campo_dei_metadati_diverso_e_un_difetto_di_f5() {
    let scan = scansioni();
    let mut l = apri_file(&scan);
    let mut v = verita_di(&scan);
    v["metadati"]["mzml:run@startTimeStamp"]["valore"] = json!("1999-01-01T00:00:00Z");
    let r = confronta(&mut *l, &v, 0, TOL, "sintetico");
    let Esito::Difetto(d) = &r.livelli[5].1 else {
        panic!("{:?}", r.livelli)
    };
    assert!(d.iter().any(|x| x.contains("1999")), "{d:?}");
}

#[test]
fn un_campo_senza_percorso_non_e_un_difetto_ma_manca() {
    let scan = scansioni();
    let mut l = apri_file(&scan);
    let mut v = verita_di(&scan);
    v["metadati"] = json!({"mzml:analyzer": {"valore": "orbitrap"}});
    let r = confronta(&mut *l, &v, 0, TOL, "sintetico");
    let Esito::Manca(m) = &r.livelli[5].1 else {
        panic!("{:?}", r.livelli)
    };
    assert!(m[0].contains("not mapped"), "{m:?}");
}

#[test]
fn la_normalizzazione_piega_gli_spazi_e_il_percorso_segue_gli_indici() {
    assert_eq!(
        verita::normalizza("  Q   Exactive\tPlus "),
        "Q Exactive Plus"
    );
    let d = json!({"ms": {"periodi": [{"nome": "a"}, {"nome": "b"}]}});
    assert_eq!(
        verita::percorso_json(&d, "ms.periodi[1].nome"),
        Some(&json!("b"))
    );
    assert_eq!(verita::percorso_json(&d, "ms.assente"), None);
}

static RADICE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

fn percorso_prova(rel: &str) -> Option<std::path::PathBuf> {
    RADICE.get().map(|r| r.join(rel)).filter(|p| p.exists())
}

/// `corpus` finds the truth under `scratchpad/rosina/verita/`, the native file under `dati/formati/` with the same relative path, opens it and compares.
#[test]
fn il_corpus_si_scorre_su_una_cartella_costruita() {
    let radice = std::env::temp_dir().join(format!("rosina-banco-{}", std::process::id()));
    let scan = scansioni();
    let metodo = r#"{"schema":"metodo/1","origine":{"file_creato":"2024-03-05T10:11:12Z"}}"#;
    let nativo = radice.join("dati/formati/aperti/prova/campione.mzML");
    std::fs::create_dir_all(nativo.parent().unwrap()).unwrap();
    std::fs::write(
        &nativo,
        mzml_di(
            &scan,
            Cod::Np {
                intensita: Np::Slof,
                zlib: false,
            },
            Some(metodo),
        ),
    )
    .unwrap();
    let vdir = radice.join("scratchpad/rosina/verita/aperti/prova");
    std::fs::create_dir_all(&vdir).unwrap();
    let mut v = verita_di(&scan);
    // the rounding of the numpress codec: m/z by Da, intensities within 1e-3 of the maximum
    v["tolleranza"] = json!({"mz_ppm": null, "mz_da": 1e-3, "intensita_rel": 1e-3});
    std::fs::write(vdir.join("campione.mzML.verita.json"), v.to_string()).unwrap();
    RADICE.set(radice.clone()).unwrap();
    let dir = percorso_prova("scratchpad/rosina/verita").unwrap();
    let files = verita::verita_in(&dir);
    assert_eq!(files.len(), 1);
    let v = verita::leggi(&files[0]).unwrap();
    assert!(v["tolleranza"].is_object());
    let n = percorso_prova("dati/formati/aperti/prova/campione.mzML").unwrap();
    assert!(n.is_file());
    verita::corpus("aperti", |r| r.ends_with(".mzML"), Tol::LR, percorso_prova);
    assert_eq!(verita::tolleranza_di(&v, Tol::HR).mz_da, 1e-3);
    std::fs::remove_dir_all(radice).unwrap();
}

/// The truth that `tools/rosina/verita.py` made of the synthetic mzML of `tests/rosina/` agrees with the mzML parser of the core (the two readers of the same file): the Python tool
/// and the Rust bench speak of the same numbers.
#[test]
fn la_verita_di_verita_py_concorda_con_il_parser_mzml() {
    use mzlab_core::mzml::{self, Item};
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/rosina");
    let v = verita::leggi(&dir.join("sintetico.verita.json")).unwrap();
    let (mut spettri, mut croms) = (Vec::new(), Vec::new());
    mzml::parse(
        std::io::BufReader::new(std::fs::File::open(dir.join("sintetico.mzML")).unwrap()),
        |it| match it {
            Item::Spectrum(s) => spettri.push(s),
            Item::Chromatogram(c) => croms.push(c),
            _ => {}
        },
    )
    .unwrap();
    let sc = v["scansioni"].as_array().unwrap();
    assert_eq!(sc.len(), spettri.len());
    assert_eq!(v["cromatogrammi"].as_array().unwrap().len(), croms.len());
    for (t, s) in sc.iter().zip(&spettri) {
        assert!((t["rt_min"].as_f64().unwrap() - s.rt).abs() < 1e-9, "RT");
        assert_eq!(t["livello"].as_u64(), Some(u64::from(s.level)));
        assert_eq!(t["n_punti"].as_u64(), Some(s.mz.len() as u64));
        assert_eq!(t["polarita"].as_i64(), Some(i64::from(s.polarity)));
        assert_eq!(t["precursore_mz"].as_f64(), s.precursor);
    }
    for (k, p) in v["punti"].as_object().unwrap() {
        let s = &spettri[k.parse::<usize>().unwrap()];
        for (i, m) in p["mz"].as_array().unwrap().iter().enumerate() {
            assert_eq!(m.as_f64(), Some(s.mz[i]));
            assert_eq!(p["intensita"][i].as_f64(), Some(s.intensity_f64(i)));
        }
    }
    for (t, c) in v["cromatogrammi"].as_array().unwrap().iter().zip(&croms) {
        assert_eq!(t["n"].as_u64(), Some(c.intensity.len() as u64));
        let somma: f64 = c.intensity.iter().sum();
        assert!((t["somma"].as_f64().unwrap() - somma).abs() <= 1e-6 * somma.abs().max(1.0));
    }
}
